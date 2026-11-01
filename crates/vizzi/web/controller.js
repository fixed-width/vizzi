export function createController({ transport, onView = () => {}, onStatus = () => {}, onError = () => {} }) {
  let query = {}, viewRequest, completedViewRequest, statusRequest;
  let displayedSnapshot = null, pending = false;
  async function load(patch = {}) {
    query = { ...query, ...patch };
    const request = viewRequest = {};
    pending = true;
    const parameters = new URLSearchParams(Object.entries(query).filter(([, value]) => value != null));
    try {
      const view = await transport(`/api/view?${parameters}`);
      if (request !== viewRequest) return false;
      query = { ...query, file: view.selection.file, entity: view.selection.entity };
      displayedSnapshot = view.snapshot;
      onView(view);
      completedViewRequest = request;
      return true;
    } catch (error) {
      if (request === viewRequest) onError(error);
      return false;
    } finally {
      if (request === viewRequest) pending = false;
    }
  }
  async function poll() {
    const request = statusRequest = {};
    try {
      const status = await transport("/api/status");
      if (request !== statusRequest) return;
      onStatus(status);
      if (!pending && status.snapshot && (viewRequest !== completedViewRequest || status.snapshot !== displayedSnapshot)) await load();
    } catch (error) { if (request === statusRequest) onError(error); }
  }
  return { load, poll };
}
