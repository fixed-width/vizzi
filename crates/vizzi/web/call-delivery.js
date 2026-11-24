import { TaskWorkspace } from './task-workspace.js';
function check(signal) {
  if (signal?.aborted) throw new DOMException('Call navigation cancelled', 'AbortError');
}
function waiting(promise, signal) {
  check(signal);
  if (!signal) return promise;
  return new Promise((resolve, reject) => {
    const abort = () => {
        done();
        reject(new DOMException('Call navigation cancelled', 'AbortError'));
      },
      done = () => signal.removeEventListener('abort', abort);
    signal.addEventListener('abort', abort, {
      once: true
    });
    Promise.resolve(promise).then(value => {
      done();
      if (signal.aborted) abort();else resolve(value);
    }, error => {
      done();
      reject(error);
    });
  });
}
export async function resolveCallChoice(site, {
  snapshot,
  provider,
  warmup,
  request,
  signal,
  verify = () => {}
}) {
  let attempted = false,
    note = '',
    semantic = false;
  if ((site.lookup || provider) && (!site.binding || site.lookup) && site.id) {
    attempted = provider && !site.binding;
    if (!site.binding) await waiting(warmup(), signal);
    check(signal);
    verify();
    const response = await request(snapshot, site.id, signal);
    check(signal);
    verify();
    if (response.snapshot !== snapshot || response.invocation?.id !== site.id || response.invocation.owner !== site.owner) throw new Error('Definition response differs from the selected call');
    note = response.detail ?? '';
    if (site.lookup || ['unique', 'ambiguous', 'outside'].includes(response.state)) {
      site = response.invocation;
      semantic = site.binding === 'language_server_definition';
    }
  }
  return {
    site,
    attempted,
    note,
    semantic
  };
}
export function directDestination({
  site,
  attempted,
  semantic
}) {
  return site.candidates.length === 1 && site.candidate_count === 1 && (!attempted || semantic || site.binding === 'rust_self_receiver') ? site.candidates[0] : null;
}
export function createCallPreparation({
  read,
  bind,
  resolve,
  source,
  apply
}) {
  return async function prepare(action, {
    signal
  }) {
    check(signal);
    const model = read();
    if (!model || model.stale || model.snapshot !== action.snapshot) throw new Error('Source changed since the call was selected. Select it again.');
    const binding = await bind(action, signal);
    check(signal);
    if (read() !== model || model.stale || model.snapshot !== action.snapshot || binding.snapshot !== action.snapshot || binding.invocation?.id !== action.call || binding.invocation.owner !== binding.caller?.id || action.file && binding.caller?.source.file !== action.file) throw new Error('The selected call differs from current captured source');
    const choice = await resolve({
      ...binding.invocation,
      lookup: true
    }, signal, action.snapshot, binding.caller.source.path);
    check(signal);
    if (read() !== model || model.stale || model.snapshot !== action.snapshot) throw new Error('Captured source changed while preparing call navigation');
    const caller = binding.caller,
      target = directDestination(choice),
      area = caller.source.path.split('/').slice(0, -1).join('/') || caller.source.path;
    function plan() {
      const planned = TaskWorkspace.restore(model.serialize(), model.snapshot);
      const callerId = planned.openRoot(caller, area);
      const targetId = target ? planned.follow(callerId, choice.site, target) : null;
      planned.view(callerId).collapsed = false;
      if (targetId) planned.view(targetId).collapsed = false;
      return {
        planned,
        caller: callerId,
        target: targetId
      };
    }
    const initial = plan(),
      wanted = [...new Map([initial.planned.view(initial.caller), ...(initial.target ? [initial.planned.view(initial.target)] : [])].map(view => [view.symbol.id, view])).values()];
    const sources = new Map();
    await Promise.all(wanted.map(async view => {
      if (sources.has(view.symbol.id)) return;
      const data = await source(view.symbol, view.reading, signal, action.snapshot);
      sources.set(view.symbol.id, data);
    }));
    check(signal);
    if (read() !== model || model.stale || model.snapshot !== action.snapshot) throw new Error('Captured source changed while loading call source');
    let committed = false;
    return {
      commit() {
        check(signal);
        if (committed || read() !== model || model.stale || model.snapshot !== action.snapshot) throw new Error('Captured source changed before call navigation');
        const result = plan();
        Object.assign(model, result.planned);
        apply({
          model,
          caller: result.caller,
          target: result.target,
          choice,
          sources
        });
        committed = true;
        return true;
      }
    };
  };
}
