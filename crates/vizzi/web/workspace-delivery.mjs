const PROTOCOL = 'vizzi-delivery',
  VERSION = 1;
const opaque = value => typeof value === 'string' && /^[0-9a-f]{32}$/.test(value);
const captured = value => typeof value === 'string' && /^[0-9a-f]{64}$/.test(value);
const object = value => value && typeof value === 'object' && !Array.isArray(value);
const fields = (value, allowed) => object(value) && Object.keys(value).every(key => allowed.includes(key));
const packet = value => ({
  protocol: PROTOCOL,
  version: VERSION,
  ...value
});
const validPacket = value => object(value) && value.protocol === PROTOCOL && value.version === VERSION;
const abortError = () => new DOMException('Workspace delivery cancelled', 'AbortError');
const bounded = error => String(error?.message ?? error).slice(0, 512);
export function deliveryId() {
  return [...crypto.getRandomValues(new Uint8Array(16))].map(value => value.toString(16).padStart(2, '0')).join('');
}
export function deliveryAction(value) {
  if (!object(value)) throw new Error('Invalid workspace delivery action');
  if (value.kind === 'reveal' && fields(value, ['kind'])) return {
    kind: 'reveal'
  };
  if (value.kind === 'review' && fields(value, ['kind', 'ticket']) && (value.ticket === undefined || typeof value.ticket === 'string' && value.ticket.length > 0 && value.ticket.length <= 256 && !/[\x00-\x1f\x7f]/.test(value.ticket))) return {
    kind: 'review',
    ...(value.ticket === undefined ? {} : {
      ticket: value.ticket
    })
  };
  if (value.kind === 'call' && fields(value, ['kind', 'snapshot', 'call', 'file']) && captured(value.snapshot) && captured(value.call) && (value.file === undefined || captured(value.file))) return {
    kind: 'call',
    snapshot: value.snapshot,
    call: value.call,
    ...(value.file === undefined ? {} : {file:value.file})
  };
  if (value.kind === 'function' && fields(value, ['kind', 'snapshot', 'file', 'symbol']) && [value.snapshot, value.file, value.symbol].every(captured)) return {
    kind: 'function',
    snapshot: value.snapshot,
    file: value.file,
    symbol: value.symbol
  };
  if (value.kind === 'file' && fields(value, ['kind', 'snapshot', 'file', 'version', 'line']) && [value.snapshot, value.file, value.version].every(captured) && Number.isSafeInteger(value.line) && value.line >= 1) return {
    kind: 'file',
    snapshot: value.snapshot,
    file: value.file,
    version: value.version,
    line: value.line
  };
  throw new Error('Invalid captured workspace destination');
}
export function createDeliveryClient({
  send,
  makeId = deliveryId,
  now = Date.now,
  schedule = setTimeout,
  clear = clearTimeout,
  timeout = 60000
}) {
  let current,
    document,
    mode,
    routing,
    closed = false;
  const transmit = (message, record) => {
    try {
      Promise.resolve(send(packet(message))).then(value => {
        if (value === false && record && current === record) finish(record, new Error('Could not confirm workspace navigation: the editor tab is unavailable'));
      }, error => {
        if (record && current === record) finish(record, error);
      });
    } catch (error) {
      if (record && current === record) finish(record, error);
    }
  };
  function finish(record, error, state = 'cancelled') {
    if (!record || current !== record) return;
    current = undefined;
    clear(record.timer);
    record.signal?.removeEventListener('abort', record.abort);
    if (error) record.reject(error);else record.resolve({
      state
    });
  }
  function cancel(record, reason = 'cancelled') {
    if (current !== record) return;
    transmit({
      type: 'cancel',
      id: record.id,
      document: record.document,
      reason
    });
    finish(record, undefined);
  }
  function prepare(record) {
    if (!document || routing || current !== record) return;
    record.document = document;
    record.phase = 'preparing';
    transmit({
      type: 'prepare',
      id: record.id,
      document,
      ...(record.continuation ? {
        transition: record.continuation
      } : {}),
      action: record.action,
      expires: record.expires
    }, record);
  }
  function deliver(action, {
    signal,
    validate = async () => {}
  } = {}) {
    if (closed) return Promise.resolve({
      state: 'cancelled'
    });
    action = deliveryAction(action);
    if (current) cancel(current, 'superseded');
    if (signal?.aborted) return Promise.resolve({
      state: 'cancelled'
    });
    const record = {
      id: makeId(),
      action,
      signal,
      validate,
      expires: now() + timeout,
      phase: 'waiting'
    };
    if (!opaque(record.id)) throw new Error('Invalid delivery identity');
    const result = new Promise((resolve, reject) => Object.assign(record, {
      resolve,
      reject
    }));
    current = record;
    record.abort = () => cancel(record, 'cancelled');
    signal?.addEventListener('abort', record.abort, {
      once: true
    });
    record.timer = schedule(() => {
      if (current !== record) return;
      transmit({
        type: 'cancel',
        id: record.id,
        document: record.document,
        reason: 'expired'
      });
      finish(record, new Error('Could not confirm workspace navigation before its deadline; retry the editor action'));
    }, timeout);
    prepare(record);
    return result;
  }
  function receive(message) {
    if (closed || !validPacket(message)) return;
    if (message.type === 'connected' && opaque(message.document) && typeof message.mode === 'string') {
      const old = document;
      document = message.document;
      mode = message.mode;
      const flight = routing;
      const arrived = flight && document !== flight.document && message.transition === flight.transition && mode === flight.mode;
      if (old !== document) routing = undefined;
      const record = current;
      if (!record) {
        if (opaque(message.transition)) transmit({
          type: 'cancel',
          id: message.transition,
          document,
          reason: 'obsolete'
        });
        return;
      }
      if (arrived) {
        record.continuation = flight.transition;
        prepare(record);
        return;
      }
      if (old === document) return;
      if (record.phase === 'waiting' && !flight) {
        prepare(record);
        return;
      }
      if (record.phase === 'committing') finish(record, new Error('Could not confirm workspace navigation after its document changed'));else cancel(record, 'document_changed');
      return;
    }
    if (message.type === 'cancelled' && opaque(message.transition)) {
      if (routing && message.transition === routing.transition && message.document === routing.document) {
        routing = undefined;
        if (current) finish(current, undefined);
        return;
      }
      if (current && message.transition === current.continuation && message.document === current.document) {
        finish(current, undefined);
        return;
      }
    }
    const record = current;
    if (!record || message.id !== record.id || message.document !== record.document) return;
    if (message.type === 'routing' && record.phase === 'preparing' && opaque(message.transition) && typeof message.mode === 'string') {
      record.phase = 'routing';
      routing = {
        document: record.document,
        transition: message.transition,
        mode: message.mode
      };
      transmit({
        type: 'route',
        id: record.id,
        document: record.document,
        transition: routing.transition
      }, record);
      return;
    }
    if (message.type === 'prepared' && record.phase === 'preparing' && opaque(message.ticket)) {
      record.phase = 'validating';
      record.ticket = message.ticket;
      Promise.resolve().then(() => record.validate()).then(() => {
        if (current !== record || record.phase !== 'validating' || record.document !== document) return;
        if (record.signal?.aborted) {
          cancel(record);
          return;
        }
        if (now() >= record.expires) {
          transmit({
            type: 'cancel',
            id: record.id,
            document: record.document,
            reason: 'expired'
          });
          finish(record, new Error('Could not confirm workspace navigation before its deadline'));
          return;
        }
        record.phase = 'committing';
        transmit({
          type: 'commit',
          id: record.id,
          document: record.document,
          ticket: record.ticket
        }, record);
      }, error => {
        if (current === record) {
          transmit({
            type: 'cancel',
            id: record.id,
            document: record.document,
            reason: 'source_changed'
          });
          finish(record, error);
        }
      });
      return;
    }
    if (message.type === 'applied' && record.phase === 'committing' && message.ticket === record.ticket) {
      finish(record, undefined, 'applied');
      return;
    }
    if (message.type === 'cancelled') {
      if (routing && message.document === routing.document && message.reason !== 'superseded') routing = undefined;
      finish(record, undefined);
      return;
    }
    if (message.type === 'refused' && typeof message.message === 'string' && message.message.length <= 512) finish(record, new Error(message.message));
  }
  function dispose() {
    if (current) cancel(current, 'closed');
    closed = true;
    document = undefined;
    routing = undefined;
  }
  return {
    deliver,
    receive,
    dispose
  };
}
const routes = {
  call: '/workspace',
  file: '/explore',
  review: '/review'
};
export function createDeliveryPage({
  mode,
  document = deliveryId(),
  transition = null,
  ready,
  prepare,
  send,
  makeId = deliveryId,
  now = Date.now,
  schedule = setTimeout,
  clear = clearTimeout,
  navigate = () => {},
  failed = () => {},
  changed = () => {}
}) {
  if (!opaque(transition)) transition = null;
  let current,
    closed = false,
    local,
    localReplace = true,
    continuation = transition,
    cancelledContinuation,
    departing;
  const completed = new Map();
  const emit = (value, reply = send) => reply(packet({
    document,
    ...value
  }));
  function finish(record, value) {
    if (current !== record) return;
    current = undefined;
    clear(record.timer);
    record.controller.abort();
    record.prepared?.dispose?.();
    const response = {
      id: record.id,
      ...value
    };
    completed.set(record.id, {
      signature: record.signature,
      response
    });
    if (completed.size > 32) completed.delete(completed.keys().next().value);
    emit(response, record.reply);
    changed(false);
  }
  function cancel(reason = 'cancelled') {
    if (current) finish(current, {
      type: 'cancelled',
      reason
    });
  }
  function refuse(record, error) {
    if (record && current === record) finish(record, {
      type: 'refused',
      message: bounded(error)
    });
  }
  function connect() {
    emit({
      type: 'connected',
      mode,
      transition
    });
  }
  async function resume() {
    const record = current;
    if (!record || record.busy || record.prepared || record.routing || !ready(record.action)) return;
    record.busy = true;
    try {
      const prepared = await prepare(record.action, {
        signal: record.controller.signal,
        replaceHistory: record.replaceHistory
      });
      if (current !== record) {
        prepared?.dispose?.();
        return;
      }
      if (now() >= record.expires) {
        refuse(record, new Error('Could not confirm workspace navigation before its deadline'));
        return;
      }
      if (!prepared || typeof prepared.commit !== 'function') throw new Error('Workspace navigation did not prepare a captured destination');
      record.prepared = prepared;
      record.ticket = makeId();
      emit({
        type: 'prepared',
        id: record.id,
        ticket: record.ticket
      }, record.reply);
    } catch (error) {
      if (current === record) refuse(record, error);
    }
  }
  function receive(message, reply = send, replaceHistory = false) {
    if (closed || !validPacket(message) || message.document !== document || !opaque(message.id)) return;
    if (message.type === 'prepare') {
      let action;
      try {
        action = deliveryAction(message.action);
      } catch (error) {
        emit({
          type: 'refused',
          id: message.id,
          message: bounded(error)
        }, reply);
        return;
      }
      if (cancelledContinuation && (message.id === cancelledContinuation || message.transition === cancelledContinuation)) {
        emit({
          type: 'cancelled',
          id: message.id,
          reason: 'manual_navigation'
        }, reply);
        return;
      }
      if (message.transition !== undefined && !opaque(message.transition)) {
        emit({
          type: 'refused',
          id: message.id,
          message: 'Invalid workspace continuation'
        }, reply);
        return;
      }
      const signature = JSON.stringify(action),
        previous = completed.get(message.id);
      if (previous) {
        emit(previous.signature === signature ? previous.response : {
          type: 'refused',
          id: message.id,
          message: 'Delivery identity was reused for another destination'
        }, reply);
        return;
      }
      if (current?.id === message.id) {
        if (current.signature !== signature) refuse(current, new Error('Delivery identity was reused for another destination'));else if (current.prepared) emit({
          type: 'prepared',
          id: current.id,
          ticket: current.ticket
        }, reply);
        return;
      }
      cancel('superseded');
      if (!Number.isSafeInteger(message.expires) || message.expires <= now()) {
        emit({
          type: 'refused',
          id: message.id,
          message: 'Workspace delivery expired'
        }, reply);
        return;
      }
      const record = {
        id: message.id,
        action,
        signature,
        reply,
        expires: Math.min(message.expires, now() + 60000),
        controller: new AbortController(),
        replaceHistory: Boolean(replaceHistory || continuation === message.id || continuation && message.transition === continuation)
      };
      continuation = null;
      current = record;
      record.timer = schedule(() => refuse(record, new Error('Could not confirm workspace navigation before its deadline')), record.expires - now());
      changed(true);
      const target = action.kind === 'reveal' ? mode === 'landing' ? 'file' : mode : action.kind === 'function' ? 'call' : action.kind;
      if (target !== mode) {
        record.routing = true;
        record.transition = record.id;
        record.mode = target;
        emit({
          type: 'routing',
          id: record.id,
          transition: record.transition,
          mode: target
        }, reply);
        return;
      }
      void resume();
      return;
    }
    const record = current;
    if (!record) {
      if (message.type === 'cancel' && message.id === continuation) {
        cancelledContinuation = continuation;
        continuation = null;
        changed(false);
      }
      return;
    }
    if (message.id !== record.id) return;
    if (message.type === 'cancel') {
      cancel(message.reason);
      return;
    }
    if (message.type === 'route' && record.routing && message.transition === record.transition) {
      departing = {
        id: record.id,
        transition: record.transition,
        reply: record.reply
      };
      navigate(routes[record.mode], record.transition);
      return;
    }
    if (message.type !== 'commit' || !record.prepared || message.ticket !== record.ticket) return;
    if (now() >= record.expires) {
      refuse(record, new Error('Workspace delivery expired before application'));
      return;
    }
    try {
      if (!ready(record.action)) throw new Error('The workspace view is no longer ready; retry the editor action');
      const result = record.prepared.commit();
      if (result && typeof result.then === 'function') throw new Error('Workspace application must be synchronous');
      if (result === false) throw new Error('Requested source changed; retry the editor action');
      finish(record, {
        type: 'applied',
        ticket: record.ticket
      });
    } catch (error) {
      refuse(record, error);
    }
  }
  function submit(action, {
    replaceHistory = true
  } = {}) {
    if (!local) {
      local = createDeliveryClient({
        send: message => receive(message, response => local.receive(response), localReplace),
        makeId,
        now,
        schedule,
        clear
      });
      local.receive(packet({
        type: 'connected',
        document,
        mode,
        transition: null
      }));
    }
    localReplace = replaceHistory;
    return local.deliver(action).catch(error => {
      failed(error);
      throw error;
    });
  }
  function manual() {
    const routing = Boolean(current?.routing || departing);
    if (continuation) {
      cancelledContinuation = continuation;
      emit({
        type: 'cancelled',
        id: continuation,
        transition: continuation,
        reason: 'manual_navigation'
      });
    }
    if (departing) emit({
      type: 'cancelled',
      id: departing.id,
      transition: departing.transition,
      reason: 'manual_navigation'
    }, departing.reply);
    departing = undefined;
    continuation = null;
    cancel('manual_navigation');
    changed(false);
    return routing;
  }
  function dispose() {
    cancel('closed');
    closed = true;
    local?.dispose();
    completed.clear();
  }
  function depart() {
    if (current?.routing || departing) {
      if (current) {
        clear(current.timer);
        current.controller.abort();
      }
      current = undefined;
      closed = true;
      local?.dispose();
      completed.clear();
    } else dispose();
  }
  return {
    connect,
    receive,
    resume,
    manual,
    dispose,
    depart,
    submit,
    pending: () => Boolean(current || continuation)
  };
}
