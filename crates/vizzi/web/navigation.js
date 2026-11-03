export function createNavigation({ capture, load, history, onState = () => {}, onNotice = () => {} }) {
  const session = crypto.randomUUID();
  let entries = [], cursor = 0, shown, active;
  const query = entry => ({ file: entry.file, entity: entry.entity, filter: entry.filter });
  const copy = value => structuredClone(value);
  const marker = key => ({ vizziNavigation: { schema: 1, session, key } });
  function notify() {
    onState({ canBack: cursor > 0, canForward: cursor < entries.length - 1,
      count: entries.length, pending: Boolean(active?.pending) });
  }
  function saveShown() {
    const entry = entries.find(entry => entry.key === shown);
    if (entry) entry.location = copy(capture());
  }
  function seed() {
    const key = crypto.randomUUID();
    entries = [{ key, location: copy(capture()) }]; cursor = 0; shown = key;
    history.replace(marker(key));
  }
  function observe(intent) {
    if (intent) {
      if (intent !== active || intent.committed) return;
      const current = copy(capture());
      if (!entries.length) seed();
      else if (intent.returning) {
        const entry = entries.find(entry => entry.key === intent.key);
        entry.location = current; shown = entry.key;
        if (intent.target.entity && current.entity !== intent.target.entity) {
          onNotice("Saved declaration is no longer available. Showing the current file.");
        } else if (current.version !== intent.target.version) {
          onNotice("This location changed since your visit. Showing current source.");
        } else onNotice(null);
      } else if (entries[cursor].key !== shown || current.file !== intent.before.file || current.entity !== intent.before.entity) {
        entries = entries.slice(0, cursor + 1);
        const key = crypto.randomUUID();
        entries.push({ key, location: current });
        if (entries.length > 50) entries.shift();
        cursor = entries.length - 1; shown = key; history.push(marker(key));
      }
      intent.committed = true; intent.pending = false;
    } else if (!entries.length && !active?.pending) seed();
    notify();
  }
  async function request(intent, patch) {
    active = intent; onNotice(null); notify();
    try { return await load(patch, intent); }
    finally {
      intent.pending = false;
      if (active === intent && intent.returning && !intent.committed && !intent.unavailable) {
        onNotice("Could not restore this location. The current view is kept; use Back or Forward to retry.");
      }
      notify();
    }
  }
  function navigate(patch) {
    saveShown();
    const before = copy(capture());
    return request({ before, pending: true }, { ...query(before), ...patch });
  }
  function traverse(state) {
    const token = state?.vizziNavigation;
    if (!token) return Promise.resolve(false);
    saveShown();
    const index = token.schema === 1 && token.session === session ? entries.findIndex(entry => entry.key === token.key) : -1;
    if (index < 0) {
      active = undefined; seed(); onNotice("This saved location expired. Navigation history restarted."); notify();
      return Promise.resolve(false);
    }
    cursor = index;
    const entry = entries[index], target = copy(entry.location);
    return request({ returning: true, key: entry.key, target, pending: true }, query(target));
  }
  function allow(next, intent) {
    if (!intent) return true;
    if (intent !== active) return false;
    if (intent.returning && next.selection.file !== intent.target.file) {
      intent.unavailable = true;
      onNotice(`Cannot restore ${intent.target.path ?? "this location"}. The current view is kept; use Back or Forward to continue.`);
      return false;
    }
    return true;
  }
  function restoration(next, intent) {
    if (!intent || intent !== active || !intent.returning || next.source?.id !== intent.target.file
      || next.source.version !== intent.target.version || next.selection.entity !== intent.target.entity) return undefined;
    return copy(intent.target);
  }
  return {
    observe, navigate, traverse, allow, restoration,
    move(delta) {
      if ((delta === -1 || delta === 1) && cursor + delta >= 0 && cursor + delta < entries.length) history.go(delta);
    },
  };
}
