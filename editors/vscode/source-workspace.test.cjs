const {
  test
} = require('node:test');
const assert = require('node:assert/strict');
const {
  createSourceWorkspaceLifetime
} = require('./source-workspace.cjs');
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((a, b) => {
    resolve = a;
    reject = b;
  });
  return {
    promise,
    resolve,
    reject
  };
};
const turn = () => new Promise(setImmediate);
function fixture(t, {
  resolveProject = async selected => ({
    root: selected.root,
    connectionFile: selected.root + '.json'
  }),
  createSharing,
  launch,
  retirePresentation
} = {}) {
  const sharing = [],
    children = [],
    shown = [],
    retired = [];
  const lifetime = createSourceWorkspaceLifetime({
    resolveProject,
    createSharing: createSharing || (async project => {
      const entry = {
        root: project.root,
        closed: 0,
        isCurrent: async () => true,
        dispose: async () => {
          entry.closed++;
        }
      };
      sharing.push(entry);
      return entry;
    }),
    launch: launch || (async (project, options) => {
      const child = {
        root: project.root,
        url: 'http://127.0.0.1:' + String(7000 + children.length) + '/',
        closed: 0,
        exit: options.onExit,
        dispose: async () => {
          child.closed++;
        }
      };
      children.push(child);
      return child;
    }),
    present: async ({
      session
    }) => {
      shown.push(session.root);
      return session;
    },
    retirePresentation: retirePresentation || (() => retired.push(true))
  });
  t.after(() => lifetime.dispose());
  return {
    lifetime,
    sharing,
    children,
    shown,
    retired,
    open: (root, options) => lifetime.use(() => ({
      root
    }), options)
  };
}
test('same-root commands share startup and preserve their different presentations', async t => {
  const f = fixture(t);
  const ordinary = f.open('A');
  const review = f.open('A', {
    present: async ({
      session
    }) => {
      f.shown.push('review ' + session.root);
    }
  });
  await Promise.all([ordinary, review]);
  assert.equal(f.children.length, 1);
  assert.equal(f.sharing.length, 1);
  assert.deepEqual(f.shown, ['A', 'review A']);
});
test('Stop cancels an earlier browser action, and a fresh Open waits for cleanup', async t => {
  const ready = deferred(),
    started = deferred();
  let disposed = 0,
    starts = 0;
  const f = fixture(t, {
    launch: async () => {
      if (++starts === 1) {
        started.resolve();
        await ready.promise;
      }
      return {
        url: 'http://127.0.0.1:7474/',
        dispose: async () => {
          disposed++;
        }
      };
    }
  });
  const old = f.open('A');
  await started.promise;
  const browser = f.open('A', {
    present: async () => f.shown.push('browser')
  });
  const stop = f.lifetime.stop();
  const fresh = f.open('A');
  await turn();
  assert.equal(starts, 1);
  ready.resolve();
  await Promise.all([old, browser, stop, fresh]);
  assert.equal(starts, 2);
  assert.equal(disposed, 1);
  assert.deepEqual(f.shown, ['A']);
});
test('A then B then A retires old launches and ends at the latest selected project', async t => {
  const ready = deferred(),
    started = deferred();
  let starts = 0,
    closed = 0;
  const f = fixture(t, {
    launch: async project => {
      if (++starts === 1) {
        started.resolve();
        await ready.promise;
      }
      return {
        url: 'http://127.0.0.1:7474/',
        dispose: async () => {
          closed++;
        }
      };
    }
  });
  const one = f.open('A');
  await started.promise;
  const two = f.open('B');
  await turn();
  const three = f.open('A');
  await turn();
  ready.resolve();
  await Promise.all([one, two, three]);
  assert.deepEqual(f.shown, ['A']);
  assert.equal(closed, 1);
  assert.equal(starts, 2);
});
test('Stop invalidates unresolved selections without delaying a fresh request', async t => {
  const f = fixture(t),
    selected = deferred();
  const old = f.lifetime.use(() => selected.promise);
  await f.lifetime.stop();
  await f.open('B');
  selected.resolve({
    root: 'A'
  });
  await old;
  assert.deepEqual(f.shown, ['B']);
  assert.deepEqual(f.children.map(c => c.root), ['B']);
});
test('a late project resolution cannot replace a newer selected project', async t => {
  const old = deferred();
  const f = fixture(t, {
    resolveProject: selected => selected.root === 'A' ? old.promise : Promise.resolve(selected)
  });
  const a = f.open('A');
  await f.open('B');
  old.resolve({
    root: 'A'
  });
  await a;
  assert.deepEqual(f.shown, ['B']);
});
test('Stop Sharing during native startup leaves the launch and origin ownership intact', async t => {
  const ready = deferred(),
    started = deferred();
  let closed = 0;
  const f = fixture(t, {
    launch: async () => {
      started.resolve();
      await ready.promise;
      return {
        url: 'http://127.0.0.1:7474/',
        dispose: async () => {
          closed++;
        }
      };
    }
  });
  const open = f.open('A');
  await started.promise;
  await f.lifetime.stop('sharing');
  ready.resolve();
  await open;
  assert.equal(closed, 0);
  assert.deepEqual(f.shown, ['A']);
  assert.equal(f.sharing[0].closed, 1);
  await f.open('A');
  assert.equal(f.sharing.length, 2);
  assert.equal(closed, 0);
});
test('stopping sharing before selection settles prevents an earlier Open from restoring sharing', async t => {
  const f = fixture(t),
    selected = deferred();
  const open = f.lifetime.use(() => selected.promise);
  await f.lifetime.stop('sharing');
  selected.resolve({
    root: 'A'
  });
  await open;
  assert.equal(f.sharing.length, 0);
  assert.deepEqual(f.shown, ['A']);
});
test('an old process exit cannot close its replacement; current exit retains sharing', async t => {
  const f = fixture(t);
  await f.open('A');
  const old = f.children[0];
  await f.open('B');
  const before = f.retired.length;
  old.exit();
  assert.equal(f.retired.length, before);
  f.children[1].exit();
  assert.equal(f.retired.length, before + 1);
  await turn();
  assert.equal(f.sharing.at(-1).closed, 0);
  await f.open('B');
  assert.equal(f.children.length, 3);
});
test('preparation completed after a project switch cannot present or report its old failure', async t => {
  const f = fixture(t),
    ready = deferred(),
    started = deferred();
  const old = f.open('A', {
    prepare: async () => {
      started.resolve();
      await ready.promise;
      throw new Error('obsolete');
    }
  });
  await started.promise;
  await f.open('B');
  ready.resolve();
  await old;
  assert.deepEqual(f.shown, ['B']);
});
test('current browser failure preserves the native process for the next command', async t => {
  const f = fixture(t);
  await assert.rejects(f.open('A', {
    present: async () => {
      throw new Error('browser failed');
    }
  }), /browser failed/);
  assert.equal(f.children[0].closed, 0);
  await f.open('A');
  assert.equal(f.children.length, 1);
});
test('source-intent cancellation retires only that command while shared capture continues', async t => {
  const f = fixture(t),
    controller = new AbortController(),
    started = deferred(),
    ready = deferred();
  const old = f.open('A', {
    signal: controller.signal,
    prepare: async () => {
      started.resolve();
      await ready.promise;
    }
  });
  await started.promise;
  controller.abort();
  await old;
  await f.open('A');
  ready.resolve();
  assert.equal(f.children.length, 1);
  assert.equal(f.children[0].closed, 0);
  assert.deepEqual(f.shown, ['A']);
});
test('Stop Sharing releases a native Open without waiting for late sharing acquisition', async t => {
  const ready = deferred(),
    started = deferred();
  let closed = 0;
  const f = fixture(t, {
    createSharing: async () => {
      started.resolve();
      await ready.promise;
      return {
        isCurrent: async () => true,
        dispose: async () => {
          closed++;
        }
      };
    }
  });
  const opening = f.open('A');
  await started.promise;
  const stopping = f.lifetime.stop('sharing');
  try {
    await turn();
    assert.deepEqual(f.shown, ['A']);
    assert.equal(f.children.length, 1);
  } finally {
    ready.resolve();
    await Promise.all([opening, stopping]);
  }
  assert.equal(closed, 1);
});
test('exit before readiness reports the current launch failure and closes its late handle', async t => {
  let closed = 0;
  const f = fixture(t, {
    launch: async (_project, {
      onExit
    }) => {
      onExit();
      return {
        url: 'http://127.0.0.1:7474/',
        dispose: async () => {
          closed++;
        }
      };
    }
  });
  await assert.rejects(f.open('A'), /exited before/);
  await turn();
  assert.equal(closed, 1);
  assert.deepEqual(f.shown, []);
});
test('advanced sharing changes only sharing while a native workspace remains owned', async t => {
  const f = fixture(t);
  await f.open('A');
  await f.open('B', {
    resources: 'sharing',
    refresh: true,
    present: async ({
      project
    }) => project
  });
  assert.equal(f.children.length, 1);
  assert.equal(f.children[0].closed, 0);
  assert.equal(f.retired.length, 0);
  assert.deepEqual(f.sharing.map(s => s.root), ['A', 'B']);
});
test('a source request cancelled during shared startup does not cancel the native launch', async t => {
  const ready = deferred(),
    started = deferred(),
    controller = new AbortController();
  let starts = 0;
  const f = fixture(t, {
    launch: async () => {
      starts++;
      started.resolve();
      await ready.promise;
      return {
        url: 'http://127.0.0.1:7474/',
        dispose: async () => {}
      };
    }
  });
  const old = f.open('A', {
    signal: controller.signal
  });
  await started.promise;
  controller.abort();
  await old;
  const next = f.open('A');
  ready.resolve();
  await next;
  assert.equal(starts, 1);
  assert.deepEqual(f.shown, ['A']);
});
test('a shared startup failure preserves one error object and allows an explicit retry', async t => {
  const failure = new Error('launch failed');
  let starts = 0;
  const f = fixture(t, {
    launch: async () => {
      if (++starts === 1) throw failure;
      return {
        url: 'http://127.0.0.1:7474/',
        dispose: async () => {}
      };
    }
  });
  const results = await Promise.allSettled([f.open('A'), f.open('A')]);
  assert.equal(results[0].reason, failure);
  assert.equal(results[1].reason, failure);
  assert.equal(starts, 1);
  await f.open('A');
  assert.equal(starts, 2);
  assert.deepEqual(f.shown, ['A']);
});
test('an earlier project-resolution failure is silent after another project is selected', async t => {
  const pending = deferred();
  const f = fixture(t, {
    resolveProject: selected => selected.root === 'A' ? pending.promise : Promise.resolve(selected)
  });
  const old = f.open('A');
  await f.open('B');
  pending.reject(new Error('old project unavailable'));
  assert.equal(await old, undefined);
  assert.deepEqual(f.shown, ['B']);
});
test('Stop Sharing cancels pending advanced-sharing presentation while native Open can continue', async t => {
  const selected = deferred();
  const f = fixture(t);
  const old = f.lifetime.use(() => selected.promise, {
    resources: 'sharing',
    present: () => f.shown.push('old sharing')
  });
  await f.lifetime.stop('sharing');
  selected.resolve({
    root: 'A'
  });
  assert.equal(await old, undefined);
  assert.deepEqual(f.shown, []);
  assert.equal(f.sharing.length, 0);
});
test('the latest advanced-sharing choice suppresses earlier results and late failures', async t => {
  const pending = deferred();
  const f = fixture(t, {
    resolveProject: selected => selected.root === 'A' ? pending.promise : Promise.resolve(selected)
  });
  const options = {
    resources: 'sharing',
    present: ({
      project
    }) => f.shown.push(project.root)
  };
  const old = f.open('A', options);
  await f.open('B', options);
  pending.reject(new Error('obsolete sharing error'));
  assert.equal(await old, undefined);
  assert.deepEqual(f.shown, ['B']);
});
test('cancelling a picker creates no resources and disposal permanently closes the lifetime', async t => {
  const f = fixture(t);
  assert.equal(await f.lifetime.use(() => undefined), undefined);
  assert.equal(f.sharing.length, 0);
  assert.equal(f.children.length, 0);
  await f.open('A');
  await f.lifetime.dispose();
  await f.open('B');
  await f.lifetime.dispose();
  assert.equal(f.children.length, 1);
  assert.equal(f.children[0].closed, 1);
  assert.equal(f.sharing[0].closed, 1);
});
test('an already cancelled caller never starts selection, while current abort failures remain visible', async t => {
  const f = fixture(t),
    controller = new AbortController();
  controller.abort();
  let selected = 0;
  await f.lifetime.use(() => {
    selected++;
    return {
      root: 'A'
    };
  }, {
    signal: controller.signal
  });
  assert.equal(selected, 0);
  const error = new DOMException('operation interrupted independently', 'AbortError');
  await assert.rejects(f.open('A', {
    prepare: async () => {
      throw error;
    }
  }), e => e === error);
});
test('a replaced semantic manifest is refreshed once and explicit sharing refreshes even a current manifest', async t => {
  const f = fixture(t);
  await f.open('A');
  f.sharing[0].isCurrent = async () => false;
  await Promise.all([f.open('A'), f.open('A')]);
  assert.equal(f.sharing.length, 2);
  assert.equal(f.sharing[0].closed, 1);
  assert.equal(f.children.length, 1);
  await f.open('A', {
    resources: 'sharing',
    refresh: true
  });
  assert.equal(f.sharing.length, 3);
  assert.equal(f.sharing[1].closed, 1);
  assert.equal(f.children[0].closed, 0);
});
test('sharing failure is reported and partial acquisition can be retried without retaining a failed record', async t => {
  const failure = new Error('sharing failed');
  let attempts = 0,
    closed = 0;
  const f = fixture(t, {
    createSharing: async () => {
      if (++attempts === 1) throw failure;
      return {
        isCurrent: async () => true,
        dispose: async () => {
          closed++;
        }
      };
    }
  });
  await assert.rejects(f.open('A'), e => e === failure);
  await f.open('A');
  assert.equal(attempts, 2);
  assert.deepEqual(f.shown, ['A']);
  await f.lifetime.stop();
  assert.equal(closed, 1);
});
test('a native Open cannot replace more recent advanced sharing while its selection resolves', async t => {
  const selected = deferred();
  const f = fixture(t);
  const old = f.lifetime.use(() => selected.promise);
  await f.open('B', {
    resources: 'sharing',
    present: () => {}
  });
  selected.resolve({
    root: 'A'
  });
  await old;
  assert.deepEqual(f.sharing.map(s => s.root), ['B']);
  assert.deepEqual(f.shown, ['A']);
});
test('advanced-sharing preparation is cancelled independently when sharing stops', async t => {
  const f = fixture(t),
    ready = deferred(),
    started = deferred();
  const old = f.open('A', {
    resources: 'sharing',
    prepare: async () => {
      started.resolve();
      await ready.promise;
    }
  });
  await started.promise;
  await f.lifetime.stop('sharing');
  await old;
  ready.resolve();
  assert.deepEqual(f.shown, []);
  assert.equal(f.sharing[0].closed, 1);
});
test('completed operations release supplied abort listeners', async t => {
  const {
    getEventListeners
  } = require('node:events');
  const f = fixture(t),
    controller = new AbortController();
  let scope;
  await f.open('A', {
    signal: controller.signal,
    present: ({
      signal
    }) => {
      scope = signal;
    }
  });
  assert.equal(getEventListeners(controller.signal, 'abort').length, 0);
  assert.equal(getEventListeners(scope, 'abort').length, 0);
});
test('a resource waiting behind cleanup is never created after being retired', async t => {
  const gate = deferred();
  const created = [];
  const f = fixture(t, {
    createSharing: async project => {
      created.push(project.root);
      return {
        isCurrent: async () => true,
        dispose: async () => {
          if (project.root === 'A') await gate.promise;
        }
      };
    }
  });
  await f.open('A', {
    resources: 'sharing',
    present: () => {}
  });
  const b = f.open('B', {
    resources: 'sharing',
    present: () => {}
  });
  await turn();
  const stop = f.lifetime.stop('sharing');
  gate.resolve();
  await Promise.all([b, stop]);
  assert.deepEqual(created, ['A']);
});
test('same-root choices advance ordering even while an older other-root resolution is pending', async t => {
  const pending = deferred();
  const f = fixture(t, {
    resolveProject: selected => selected.root === 'B' ? pending.promise : Promise.resolve(selected)
  });
  await f.open('A');
  const b = f.open('B');
  await f.open('A');
  pending.resolve({
    root: 'B'
  });
  await b;
  assert.deepEqual(f.shown, ['A', 'A']);
  assert.equal(f.children.length, 1);
});
test('Stop waits for sharing cleanup before a new native Open can launch', async t => {
  const gate = deferred();
  let closed = 0;
  const f = fixture(t, {
    createSharing: async () => ({
      isCurrent: async () => true,
      dispose: async () => {
        closed++;
        await gate.promise;
      }
    })
  });
  await f.open('A');
  const stop = f.lifetime.stop();
  const next = f.open('B');
  await turn();
  assert.equal(f.children.length, 1);
  gate.resolve();
  await Promise.all([stop, next]);
  assert.equal(f.children.length, 2);
  assert.equal(closed, 1);
});
test('late failures from a sharing freshness check cannot affect a replacement', async t => {
  const f = fixture(t),
    pending = deferred(),
    started = deferred();
  await f.open('A');
  f.sharing[0].isCurrent = () => {
    started.resolve();
    return pending.promise;
  };
  const old = f.open('A');
  await started.promise;
  await f.open('B');
  pending.reject(new Error('old read failed'));
  await old;
  assert.deepEqual(f.shown, ['A', 'B']);
  assert.equal(f.children.at(-1).closed, 0);
});
test('completed transient selections are released', {
  skip: typeof global.gc !== 'function'
}, async t => {
  const f = fixture(t,{resolveProject:async selected=>selected});
  await f.open('A');
  let selected = {
    root: 'A',
    details: {
      text: 'transient'
    }
  };
  const reference = new WeakRef(selected);
  await f.lifetime.use(() => selected);
  selected = null;
  for (let i = 0; i < 20; i++) {
    await turn();
    global.gc();
    await turn();
    if (!reference.deref()) return;
  }
  assert.equal(reference.deref(), undefined, 'completed selection retained by the lifetime owner');
});
test('sequential ordinary opens reuse both the current sharing connection and native child', async t => {
  const f = fixture(t);
  await f.open('A');
  await f.open('A');
  assert.equal(f.sharing.length, 1);
  assert.equal(f.children.length, 1);
  assert.equal(f.sharing[0].closed, 0);
});
test('an old freshness result cannot replace newer advanced sharing', async t => {
  const f = fixture(t),
    read = deferred(),
    started = deferred();
  await f.open('A');
  f.sharing[0].isCurrent = () => {
    started.resolve();
    return read.promise;
  };
  const old = f.open('A');
  await started.promise;
  await f.open('B', {
    resources: 'sharing',
    present: () => {}
  });
  read.resolve(false);
  await old;
  assert.deepEqual(f.sharing.map(s => s.root), ['A', 'B']);
  assert.equal(f.sharing.at(-1).closed, 0);
  assert.equal(f.children.length, 1);
});
test('current sharing-freshness failures are reported without losing the native child', async t => {
  const f = fixture(t);
  await f.open('A');
  const failure = new Error('cannot verify connection');
  f.sharing[0].isCurrent = async () => {
    throw failure;
  };
  await assert.rejects(f.open('A'), error => error === failure);
  assert.equal(f.children[0].closed, 0);
});
test('restarted native ownership is closed by the next Stop', async t => {
  const f = fixture(t);
  await f.open('A');
  f.children[0].exit();
  await f.open('A');
  await f.lifetime.stop();
  assert.equal(f.children[1].closed, 1);
});
test('source cancellation during a sharing refresh preserves the shared resource acquisition', async t => {
  const ready = deferred(),
    started = deferred(),
    controller = new AbortController();
  let attempts = 0,
    closed = 0;
  const f = fixture(t, {
    createSharing: async () => {
      const number = ++attempts;
      if (number === 2) {
        started.resolve();
        await ready.promise;
      }
      return {
        isCurrent: async () => number !== 1,
        dispose: async () => {
          closed++;
        }
      };
    }
  });
  await f.open('A');
  const old = f.open('A', {
    signal: controller.signal
  });
  await started.promise;
  controller.abort();
  await old;
  const next = f.open('A');
  ready.resolve();
  await next;
  assert.equal(attempts, 2);
  assert.equal(closed, 1);
  assert.equal(f.children.length, 1);
});

test('a project switch invalidates an older unresolved choice even if it later selects the new root',async t=>{
  const f=fixture(t),selected=deferred();await f.open('A');
  const old=f.lifetime.use(()=>selected.promise,{present:()=>f.shown.push('obsolete')});await f.open('B');selected.resolve({root:'B'});await old;assert.deepEqual(f.shown,['A','B']);
});
test('same-project restart preserves an unbound same-project choice and retires the old tab only once',async t=>{
  const f=fixture(t),selected=deferred();await f.open('A');const old=f.lifetime.use(()=>selected.promise);
  f.children[0].exit();assert.equal(f.retired.length,1);await f.open('A');assert.equal(f.retired.length,1);
  selected.resolve({root:'A'});await old;assert.deepEqual(f.shown,['A','A','A']);
});
test('both same-root opens continue when sharing stops during their shared acquisition',async t=>{
  const ready=deferred(),started=deferred();const f=fixture(t,{createSharing:async(_project,{signal})=>{started.resolve(signal);await ready.promise;return {isCurrent:async()=>true,dispose:async()=>{}};}});
  const one=f.open('A');const signal=await started.promise;const two=f.open('A');await turn();const stop=f.lifetime.stop('sharing');
  try{await turn();assert.equal(signal.aborted,true);assert.deepEqual(f.shown,['A','A']);}finally{ready.resolve();await Promise.all([one,two,stop]);}
});
test('native reuse does not wait for a stopped sharing-freshness read',async t=>{
  const f=fixture(t),read=deferred(),started=deferred();await f.open('A');f.sharing[0].isCurrent=()=>{started.resolve();return read.promise;};
  const opening=f.open('A');await started.promise;await f.lifetime.stop('sharing');
  try{await turn();assert.deepEqual(f.shown,['A','A']);}finally{read.resolve(false);await opening;}
});
test('obsolete freshness checks do not wait for newer same-root sharing acquisition',async t=>{
  const fresh=deferred(),read=deferred(),checking=deferred(),creating=deferred();let attempts=0;
  const f=fixture(t,{createSharing:async()=>{const number=++attempts;if(number===2){creating.resolve();await fresh.promise;}return {isCurrent:()=>{checking.resolve();return read.promise;},dispose:async()=>{}};}});
  await f.open('A');const old=f.open('A');await checking.promise;const advanced=f.open('A',{resources:'sharing',refresh:true,present:()=>{}});await creating.promise;
  try{await turn();assert.deepEqual(f.shown,['A','A']);}finally{fresh.resolve();read.resolve(false);await Promise.all([old,advanced]);}
});
test('current sharing selection failures remain visible after older successful sharing',async t=>{
  const f=fixture(t);await f.open('A',{resources:'sharing',present:()=>{}});const error=new Error('current selection failed');
  await assert.rejects(f.lifetime.use(()=>{throw error;},{resources:'sharing'}),e=>e===error);
});
test('Stop signals cooperative project resolution and suppresses its late failure',async t=>{
  const pending=deferred(),started=deferred();const f=fixture(t,{resolveProject:async(_selected,{signal})=>{started.resolve(signal);await pending.promise;throw new Error('obsolete resolution');}});
  const old=f.open('A');const signal=await started.promise;await f.lifetime.stop();assert.equal(signal.aborted,true);pending.resolve();await old;
});
test('failed operation scopes and repeated native startup failures leave no abort listeners',async t=>{
  const {getEventListeners}=require('node:events');const warnings=[];const warn=value=>{if(value.name==='MaxListenersExceededWarning')warnings.push(value);};process.on('warning',warn);t.after(()=>process.off('warning',warn));
  let attempts=0;const f=fixture(t,{launch:async()=>{if(++attempts<14)throw new Error('startup failed');return {url:'http://127.0.0.1:7474/',dispose:async()=>{}};}});
  for(let i=0;i<13;i++)await assert.rejects(f.open('A'),/startup failed/);
  let signal;await assert.rejects(f.open('A',{present:async context=>{signal=context.signal;throw new Error('presentation failed');}}),/presentation failed/);
  assert.equal(getEventListeners(signal,'abort').length,0);await turn();assert.deepEqual(warnings,[]);
});

test('selection completion queued immediately before Stop cannot begin project resolution',async t=>{
  const selected=deferred(),started=deferred();let resolved=0;const f=fixture(t,{resolveProject:async selection=>{resolved++;return selection;}});
  const old=f.lifetime.use(()=>{started.resolve();return selected.promise;});await started.promise;
  selected.resolve({root:'A'});queueMicrotask(()=>f.lifetime.stop());await old;assert.equal(resolved,0);
});
test('a synchronously cancelled unresolved selector returns without waiting for it',async t=>{
  const f=fixture(t),controller=new AbortController();
  await f.lifetime.use(()=>{controller.abort();return new Promise(()=>{});},{signal:controller.signal});assert.deepEqual(f.shown,[]);
});
test('a synchronously cancelled selector consumes its rejected late result',async t=>{
  const f=fixture(t),controller=new AbortController();
  await f.lifetime.use(()=>{controller.abort();return Promise.reject(new Error('late selector failure'));},{signal:controller.signal});await turn();assert.deepEqual(f.shown,[]);
});
test('Stop between preparation completion and presentation prevents the presentation',async t=>{
  const ready=deferred(),started=deferred(),f=fixture(t);
  const old=f.open('A',{prepare:()=>{started.resolve();return ready.promise;}});await started.promise;
  ready.resolve('prepared');queueMicrotask(()=>f.lifetime.stop());await old;assert.deepEqual(f.shown,[]);
});
test('Stop at native startup continuations cannot start obsolete preparation or presentation',async t=>{
  for(let turns=0;turns<16;turns++) {
    const obsolete=[];let stopped=false,closing;const f=fixture(t,{launch:async()=>{
      let scheduled=Promise.resolve();for(let i=0;i<turns;i++)scheduled=scheduled.then(()=>{});
      closing=scheduled.then(()=>{stopped=true;return f.lifetime.stop();});
      return {url:'http://127.0.0.1:7474/',dispose:async()=>{}};
    }});
    await f.open('A',{prepare:()=>{if(stopped)obsolete.push('prepare');},present:()=>{if(stopped)obsolete.push('present');}});await closing;assert.deepEqual(obsolete,[]);
  }
});
test('failed native acquisition releases its signal after cleanup',{skip:typeof global.gc!=='function'},async t=>{
  let reference;const f=fixture(t,{launch:async(_project,{signal})=>{reference=new WeakRef(signal);throw new Error('failed acquisition');}});
  await assert.rejects(f.open('A'),/failed acquisition/);
  for(let i=0;i<20;i++){await turn();global.gc();await turn();if(!reference.deref())return;}
  assert.equal(reference.deref(),undefined,'failed native acquisition signal retained');
});

test('sharing-only operations reuse a current connection unless refresh is explicitly requested',async t=>{
  const f=fixture(t),options={resources:'sharing',present:()=>{}};await f.open('A',options);await f.open('A',options);assert.equal(f.sharing.length,1);
  await f.open('A',{...options,refresh:true});assert.equal(f.sharing.length,2);assert.equal(f.children.length,0);
});
test('Stop Sharing suppresses a pending advanced selector failure',async t=>{
  const pending=deferred(),f=fixture(t);const old=f.lifetime.use(()=>pending.promise,{resources:'sharing'});
  await f.lifetime.stop('sharing');pending.reject(new Error('obsolete selector failed'));assert.equal(await old,undefined);
});
test('current sharing preparation failures are propagated while the connection remains owned',async t=>{
  const f=fixture(t),failure=new Error('current sharing preparation failed');
  await assert.rejects(f.open('A',{resources:'sharing',prepare:async()=>{throw failure;}}),e=>e===failure);assert.equal(f.sharing[0].closed,0);
});
test('a current failure in an older same-root command remains visible after another command succeeds',async t=>{
  const pending=deferred(),started=deferred(),f=fixture(t),failure=new Error('current comparison failed');
  const old=f.open('A',{prepare:async()=>{started.resolve();return pending.promise;}});await started.promise;await f.open('A');
  pending.reject(failure);await assert.rejects(old,e=>e===failure);
});
test('initial selection and project-resolution failures retain their original identity',async t=>{
  const failure=new Error('current project unavailable');const f=fixture(t,{resolveProject:async()=>{throw failure;}});
  await assert.rejects(f.lifetime.use(()=>{throw failure;}),e=>e===failure);await assert.rejects(f.open('A'),e=>e===failure);
});
test('late sharing acquisition cannot present its retired connection after a replacement',async t=>{
  for(let turns=0;turns<16;turns++) {
    const signals=new Map(),obsolete=[];let replacement;const f=fixture(t,{createSharing:async(project,{signal})=>{
      signals.set(project.root,signal);
      if(project.root==='A') {let scheduled=Promise.resolve();for(let i=0;i<turns;i++)scheduled=scheduled.then(()=>{});replacement=scheduled.then(()=>f.open('B',{resources:'sharing',present:()=>{}}));}
      return {isCurrent:async()=>true,dispose:async()=>{}};
    }});
    await f.open('A',{resources:'sharing',present:({project})=>{if(signals.get(project.root).aborted)obsolete.push(project.root);}});await replacement;assert.deepEqual(obsolete,[]);
  }
});

test('Stop cleanup remains a barrier even when another Stop Sharing makes a fresh Open skip sharing',async t=>{
  const gate=deferred(),selected=deferred();const f=fixture(t,{createSharing:async()=>({isCurrent:async()=>true,dispose:async()=>{await gate.promise;}})});
  await f.open('A');const stop=f.lifetime.stop();const fresh=f.lifetime.use(()=>selected.promise);const sharingStop=f.lifetime.stop('sharing');selected.resolve({root:'B'});
  try{await turn();assert.equal(f.children.length,1);}finally{gate.resolve();await Promise.all([stop,fresh,sharingStop]);}
  assert.equal(f.children.length,2);
});
test('a failed freshness check retires its owned sharing resource',async t=>{
  const f=fixture(t);await f.open('A');f.sharing[0].isCurrent=async()=>{throw new Error('connection cannot be checked');};
  await assert.rejects(f.open('A'),/cannot be checked/);await turn();assert.equal(f.sharing[0].closed,1);assert.equal(f.children[0].closed,0);
});
test('native factories are not invoked with an aborted signal at sharing-startup continuations',async t=>{
  for(let turns=0;turns<16;turns++) {
    let closing;const obsolete=[];const f=fixture(t,{createSharing:async()=>{
      let scheduled=Promise.resolve();for(let i=0;i<turns;i++)scheduled=scheduled.then(()=>{});
      closing=scheduled.then(()=>f.lifetime.stop());return {isCurrent:async()=>true,dispose:async()=>{}};
    },launch:async(_project,{signal})=>{if(signal.aborted)obsolete.push('launch');return {url:'http://127.0.0.1:7474/',dispose:async()=>{}};}});
    await f.open('A');await closing;assert.deepEqual(obsolete,[]);
  }
});

test('Stop after presentation completion discards its obsolete return value',async t=>{
  const ready=deferred(),started=deferred(),f=fixture(t);
  const old=f.open('A',{present:()=>{started.resolve();return ready.promise;}});await started.promise;
  ready.resolve('old result');queueMicrotask(()=>f.lifetime.stop());assert.equal(await old,undefined);
});

test('a newer sharing request can recover when the earlier shared acquisition fails',async t=>{
  const failed=deferred(),started=deferred(),failure=new Error('earlier sharing failed');let attempts=0;
  const f=fixture(t,{createSharing:async()=>{if(++attempts===1){started.resolve();await failed.promise;}return {isCurrent:async()=>true,dispose:async()=>{}};}});
  const one=f.open('A',{resources:'sharing',present:()=> 'old'});await started.promise;
  const two=f.open('A',{resources:'sharing',present:()=> 'new'});await turn();failed.reject(failure);
  assert.equal(await one,undefined);assert.equal(await two,'new');assert.equal(attempts,2);
});

test('tab-retirement failure cannot prevent Stop from reaping owned resources or a later Open',async t=>{
  const failure=new Error('editor channel closed');let fail=true;
  const f=fixture(t,{retirePresentation:()=>{if(fail){fail=false;throw failure;}}});await f.open('A');
  await assert.rejects(f.lifetime.stop(),e=>e===failure);assert.equal(f.children[0].closed,1);assert.equal(f.sharing[0].closed,1);
  await f.open('B');assert.equal(f.children.length,2);
});
test('tab-retirement failure during a root switch still closes the old native child',async t=>{
  const failure=new Error('old tab unavailable');let fail=true;
  const f=fixture(t,{retirePresentation:()=>{if(fail){fail=false;throw failure;}}});await f.open('A');
  await assert.rejects(f.open('B'),e=>e===failure);await turn();assert.equal(f.children[0].closed,1);await f.open('B');assert.equal(f.children.length,2);
});
test('a tab error during unexpected exit cannot throw through the child-exit notification',async t=>{
  let fail=true;const f=fixture(t,{retirePresentation:()=>{if(fail){fail=false;throw new Error('tab gone');}}});await f.open('A');
  assert.doesNotThrow(()=>f.children[0].exit());await turn();assert.equal(f.children[0].closed,1);assert.equal(f.sharing[0].closed,0);await f.open('B');
});

test('overlapping same-root opens wait for their initial sharing acquisition and share its failure', async t => {
  const started = deferred(), acquisition = deferred();
  const failure = new Error('initial sharing failed');
  const f = fixture(t, {
    createSharing: async () => {
      started.resolve();
      await acquisition.promise;
      return {isCurrent: async () => true, dispose: async () => {}};
    }
  });
  const results = Promise.allSettled([f.open('A'), f.open('A')]);
  await started.promise;
  await turn();
  const beforeFailure = {children: f.children.length, shown: [...f.shown]};
  acquisition.reject(failure);
  const settled = await results;
  assert.equal(beforeFailure.children, 0);
  assert.deepEqual(beforeFailure.shown, []);
  assert.deepEqual(settled.map(result => result.status), ['rejected', 'rejected']);
  assert.equal(settled[0].reason, failure);
  assert.equal(settled[1].reason, failure);
  assert.equal(f.children.length, 0);
});

test('a pending same-root startup is joined without a freshness wait or implicit retry after failure', async t => {
  const started = deferred(), startup = deferred(), freshness = deferred();
  const failure = new Error('shared native startup failed');
  let launches = 0, freshnessChecks = 0;
  const f = fixture(t, {
    createSharing: async () => ({
      isCurrent: async () => {freshnessChecks++;return freshness.promise;},
      dispose: async () => {}
    }),
    launch: async () => {
      if (++launches === 1) {started.resolve();await startup.promise;}
      return {url: 'http://127.0.0.1:7474/', dispose: async () => {}};
    }
  });
  const first = f.open('A');
  const firstResult = Promise.allSettled([first]);
  await started.promise;
  const second = f.open('A');
  const secondResult = Promise.allSettled([second]);
  await turn();
  startup.reject(failure);
  await turn();
  freshness.resolve(true);
  const [one, two] = await Promise.all([firstResult, secondResult]);
  assert.equal(launches, 1);
  assert.equal(freshnessChecks, 0);
  assert.equal(one[0].status, 'rejected');
  assert.equal(two[0].status, 'rejected');
  assert.equal(one[0].reason, failure);
  assert.equal(two[0].reason, failure);
  assert.deepEqual(f.shown, []);
});

test('a fresh Open after Stop Sharing restores sharing only after joining the pending startup', async t => {
  const started = deferred(), ready = deferred();
  const f = fixture(t, {
    launch: async () => {
      started.resolve();
      await ready.promise;
      return {url: 'http://127.0.0.1:7474/', dispose: async () => {}};
    }
  });
  const first = f.open('A');
  await started.promise;
  await f.lifetime.stop('sharing');
  const second = f.open('A');
  await turn();
  const acquisitionsBeforeReady = f.sharing.length;
  ready.resolve();
  await Promise.all([first, second]);
  assert.equal(acquisitionsBeforeReady, 1);
  assert.equal(f.sharing.length, 2);
  assert.equal(f.sharing[1].closed, 0);
  assert.deepEqual(f.shown, ['A', 'A']);
});
