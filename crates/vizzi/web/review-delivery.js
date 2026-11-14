const cancelled = () => new DOMException('Review delivery cancelled', 'AbortError');
const replaced = () => new Error('The requested comparison was replaced; retry the editor command');
export function createReviewPreparation({
  current,
  subscribe,
  status
}) {
  async function represented(ticket, signal) {
    if (signal?.aborted) throw cancelled();
    if (current()?.ticket === ticket) return;
    await new Promise((resolve, reject) => {
      let settled = false,
        checking = false,
        dirty = false,
        unsubscribe = () => {};
      function finish(error) {
        if (settled) return;
        settled = true;
        unsubscribe();
        signal?.removeEventListener('abort', abort);
        if (error) reject(error);else resolve();
      }
      const abort = () => finish(cancelled());
      async function verify() {
        if (checking || settled) return;
        checking = true;
        dirty = false;
        try {
          const latest = await status(signal);
          if (settled) return;
          if (signal?.aborted) {
            abort();
            return;
          }
          if (latest.ticket !== ticket) {
            finish(replaced());
            return;
          }
          if (current()?.ticket === ticket) {
            finish();
            return;
          }
        } catch (error) {
          finish(error);
          return;
        } finally {
          checking = false;
        }
        if (dirty) void verify();
      }
      const update = () => {
        if (settled) return;
        if (current()?.ticket === ticket) {
          finish();
          return;
        }
        dirty = true;
        void verify();
      };
      unsubscribe = subscribe(update);
      signal?.addEventListener('abort', abort, {
        once: true
      });
      if (signal?.aborted) abort();else void verify();
    });
  }
  return async function prepare(action, {
    signal
  } = {}) {
    if (signal?.aborted) throw cancelled();
    if (action.kind === 'review' && action.ticket !== undefined) await represented(action.ticket, signal);
    return {
      commit() {
        if (signal?.aborted) throw cancelled();
        if (action.kind === 'review' && action.ticket !== undefined && current()?.ticket !== action.ticket) return false;
      }
    };
  };
}
