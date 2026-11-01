export function createController({ transport, onView = () => {}, onStatus = () => {}, onError = () => {} }) {
  let query = { filter: "" }, viewEpoch = 0, statusEpoch = 0;
  let displayedSnapshot = null, pending = false;
  async function load(patch = {}) {
    query = { ...query, ...patch };
    const epoch = ++viewEpoch;
    pending = true;
    const parameters = new URLSearchParams(Object.entries(query).filter(([, value]) => value != null));
    try {
      const view = await transport(`/api/view?${parameters}`);
      if (epoch !== viewEpoch) return false;
      query = { ...query, file: view.selection.file, entity: view.selection.entity };
      displayedSnapshot = view.snapshot;
      onView(view);
      return true;
    } catch (error) {
      if (epoch === viewEpoch) onError(error);
      return false;
    } finally {
      if (epoch === viewEpoch) pending = false;
    }
  }
  async function poll() {
    const epoch = ++statusEpoch;
    try {
      const status = await transport("/api/status");
      if (epoch !== statusEpoch) return;
      onStatus(status);
      if (!pending && status.snapshot && status.snapshot !== displayedSnapshot) await load();
    } catch (error) { if (epoch === statusEpoch) onError(error); }
  }
  return { load, poll };
}
