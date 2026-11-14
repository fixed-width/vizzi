const cancelled = () => new DOMException('Vizzi workspace request cancelled or superseded', 'AbortError');
function check(signal) {
  if (signal.aborted) throw cancelled();
}
function wait(promise, signal) {
  if (signal.aborted) {
    Promise.resolve(promise).catch(() => {});
    return Promise.reject(cancelled());
  }
  return new Promise((resolve, reject) => {
    const abort = () => {
      finish();
      reject(cancelled());
    };
    const finish = () => signal.removeEventListener('abort', abort);
    signal.addEventListener('abort', abort, {
      once: true
    });
    Promise.resolve(promise).then(value => {
      finish();
      if (signal.aborted) reject(cancelled());else resolve(value);
    }, error => {
      finish();
      reject(error);
    });
  });
}
function connect(signal, controller) {
  const abort = () => controller.abort();
  signal.addEventListener('abort', abort, {
    once: true
  });
  if (signal.aborted) abort();
  return () => signal.removeEventListener('abort', abort);
}
function owned(before, create, validate = () => {}) {
  const controller = new AbortController();
  const record = {
    controller
  };
  record.pending = (async () => {
    await before;
    check(controller.signal);
    record.value = await create(controller.signal);
    validate();
    return record.value;
  })();
  record.pending.catch(() => {});
  record.close = () => {
    if (!record.closed) {
      controller.abort();
      record.closed = (async () => {
        await record.pending.catch(() => {});
        await record.value?.dispose();
      })();
    }
    return record.closed;
  };
  return record;
}
function createSourceWorkspaceLifetime({
  resolveProject,
  createSharing,
  launch,
  present,
  retirePresentation = () => {}
}) {
  let disposed = false,
    sequence = 0,
    target,
    sharing,
    sharingEpoch = Symbol(),
    sharingOrder = 0;
  let nativeCleanup = Promise.resolve(),
    sharingCleanup = Promise.resolve(),
    stopped = Promise.resolve();
  const operations = new Set();
  function retireSharing() {
    const old = sharing;
    sharing = undefined;
    if (old) sharingCleanup = Promise.all([sharingCleanup, old.close()]).then(() => {});
    return sharingCleanup;
  }
  function retireNative(group) {
    if (!group || group.retired) return;
    group.retired = true;
    group.controller.abort();
    if (group.native) nativeCleanup = Promise.all([nativeCleanup, group.native.close()]).then(() => {});
    if (target === group) {
      try {retirePresentation();} catch (error) {return error;}
    }
  }
  function selectNative(project, op) {
    let retirementError;
    if (target && op.order < target.order && target.root !== project.root) {
      op.controller.abort();
      throw cancelled();
    }
    if (!target || target.root !== project.root || target.controller.signal.aborted) {
      const previous = target;
      retirementError = retireNative(previous);
      if (previous && previous.root !== project.root) {
        for (const earlier of operations) if (earlier.order < op.order) earlier.controller.abort();
      }
      target = {
        root: project.root,
        order: op.order,
        controller: new AbortController()
      };
    } else target.order = Math.max(target.order, op.order);
    op.detach.push(connect(target.controller.signal, op.controller));
    if (retirementError) throw retirementError;
    return target;
  }
  function mayShare(op) {
    return op.sharingEpoch === sharingEpoch && op.order >= sharingOrder;
  }
  async function ensureSharing(project, op, signal, refresh = false) {
    if (!mayShare(op)) return;
    sharingOrder = op.order;
    for (;;) {
      check(signal);
      let current = sharing;
      try {
        if (current?.root === project.root && !refresh) {
          const connection = await wait(current.pending, AbortSignal.any([signal, current.controller.signal]));
          const valid = await wait(connection.isCurrent(), AbortSignal.any([signal, current.controller.signal]));
          if (!mayShare(op)) return;
          if (valid) return connection;
        }
        if (!mayShare(op)) return;
        retireSharing();
        current = owned(sharingCleanup, resourceSignal => createSharing(project, {
          signal: resourceSignal
        }));
        current.root = project.root;
        sharing = current;
        return await wait(current.pending, AbortSignal.any([signal, current.controller.signal]));
      } catch (error) {
        if (signal.aborted) throw error;
        if (current.controller.signal.aborted) {
          if (!mayShare(op)) return;
          continue;
        }
        if (sharing === current) {
          sharing = undefined;
          sharingCleanup = Promise.all([sharingCleanup, current.close()]).then(() => {});
        }
        throw error;
      }
    }
  }
  function ensureNative(project, op, group) {
    if (group.native) return group.native;
    const before = Promise.all([nativeCleanup, stopped]);
    let record;
    record = owned(before, async signal => {
      const detach = connect(group.controller.signal, record.controller);
      try {
        await ensureSharing(project, op, signal);
        check(signal);
        return await launch(project, {
          signal,
          onExit() {
            record.exited = true;
            if (target !== group || group.native !== record || !record.ready) return;
            retireNative(group);
            group.native = undefined;
          }
        });
      } finally {
        detach();
      }
    }, () => {
      if (record.exited) throw new Error('Vizzi exited before its workspace became ready');
      check(record.controller.signal);
      record.ready = true;
    });
    group.native = record;
    record.pending.catch(() => {
      if (group.native === record) group.native = undefined;
      nativeCleanup = Promise.all([nativeCleanup, record.close()]).then(() => {});
    });
    return record;
  }
  async function use(select, {
    resources = 'native',
    refresh = false,
    prepare,
    present: show,
    signal
  } = {}) {
    if (disposed) return;
    const controller = new AbortController();
    const op = {
      controller,
      order: ++sequence,
      sharingEpoch,
      detach: []
    };
    if (signal) op.detach.push(connect(signal, controller));
    operations.add(op);
    try {
      check(controller.signal);
      const selection = await wait(select(controller.signal), controller.signal);
      check(controller.signal);
      if (!selection) return;
      const project = await wait(resolveProject(selection, {
        signal: controller.signal
      }), controller.signal);
      op.project = project;
      await wait(stopped, controller.signal);
      let session, connection;
      if (resources === 'sharing') {
        connection = await ensureSharing(project, op, controller.signal, refresh);
        if (!connection || op.sharingEpoch !== sharingEpoch || op.order < sharingOrder) return;
        op.detach.push(connect(sharing.controller.signal, controller));
      } else {
        const group = selectNative(project, op);
        const existing = group.native;
        const native = existing || ensureNative(project, op, group);
        const handle = await wait(native.pending, controller.signal);
        if (existing) await ensureSharing(project, op, controller.signal);
        check(native.controller.signal);
        if (native.exited) throw new Error('Vizzi exited before its workspace became ready');
        op.detach.push(connect(native.controller.signal, controller));
        session = {
          root: project.root,
          url: handle.url
        };
      }
      check(controller.signal);
      const context = {
        project,
        session,
        connection,
        selection,
        signal: controller.signal
      };
      const prepared = prepare ? await wait(prepare(context), controller.signal) : selection;
      check(controller.signal);
      const display = show || (resources === 'native' ? present : undefined);
      const result = display ? await wait(display({
        ...context,
        prepared
      }), controller.signal) : prepared;
      check(controller.signal);
      return result;
    } catch (error) {
      if (controller.signal.aborted) return;
      if (resources === 'sharing' && (op.sharingEpoch !== sharingEpoch || op.order < sharingOrder)) return;
      if (resources === 'native' && !op.project && target && op.order < target.order) return;
      throw error;
    } finally {
      for (const detach of op.detach) detach();
      operations.delete(op);
    }
  }
  function stop(scope) {
    sharingEpoch = Symbol();
    if (scope === 'sharing') return retireSharing();
    for (const op of operations) op.controller.abort();
    const previous = target;
    const retirementError = retireNative(previous);
    target = undefined;
    retireSharing();
    stopped = Promise.all([nativeCleanup, sharingCleanup]).then(() => {});
    return retirementError ? stopped.then(() => {throw retirementError;}) : stopped;
  }
  function dispose() {
    disposed = true;
    return stop();
  }
  return {
    use,
    stop,
    dispose
  };
}
module.exports = {
  createSourceWorkspaceLifetime
};
