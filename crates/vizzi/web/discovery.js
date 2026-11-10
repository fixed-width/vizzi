import {renderStyledText} from '/presentation.js';

// Source discovery owns temporary selection; saved interpretation stays in the companion.
export function installDiscovery({transport, keep, openEditor, showError, browseEnabled=true}) {
  const el = id => document.getElementById(id);
  const node = (tag, text) => {const n = document.createElement(tag);n.textContent = text;return n;};
  let path = '', offset = 0, data, status, board, preview, lastSnapshot;
  let request = 0, previewRequest = 0, searchTimer, saving = false, nextNames = 0;
  let areaController, previewController;
  const languages = counts => Object.keys(counts).map(name => name === 'unassigned' ? 'unassigned source' : name).join(', ');
  function savedLocation() {
    return board?.evidence.find(e => preview && e.source.file === preview.location.file && e.source.version === preview.location.version && e.source.line === preview.location.line);
  }
  function syncSaved() {
    const saved = savedLocation();
    el('keep-discovered').textContent = saved ? 'Saved' : 'Save location';
    el('keep-discovered').disabled = !board || saving || !preview?.available || Boolean(saved);
    el('preview-editor').hidden = !saved;
  }
  function breadcrumbs() {
    const nav = el('discovery-breadcrumbs');nav.replaceChildren();
    const parts = [['Repository', '']];
    let prefix = '';
    for (const part of path.split('/').filter(Boolean)) {prefix += (prefix ? '/' : '') + part;parts.push([part, prefix]);}
    for (const [name, target] of parts) {
      const button = node('button', name);button.type = 'button';
      if (target === path) button.setAttribute('aria-current', 'location');
      button.onclick = () => navigate(target);nav.append(button);
    }
  }
  function render() {
    breadcrumbs();
    el('discovery-summary').textContent = `${data.scope_files.toLocaleString()} indexed files${path ? ` in ${path}` : ''} · ${languages(data.languages) || 'no selected source'}`;
    const coverage = [];
    if (data.source_only) coverage.push(`${data.source_only} source-only files`);
    if (data.deferred) coverage.push(`${data.deferred} files awaiting analysis`);
    el('discovery-coverage').hidden = coverage.length === 0;
    el('discovery-coverage').textContent = coverage.join(' · ');
    const container = el('discovery-areas');container.replaceChildren();
    for (const item of data.entries) {
      const directory = !item.location, button = node('button', '');button.type = 'button';button.className = 'discovery-entry';button.dataset.directory = directory;
      button.append(node('h2', item.label));
      const count = node('span', directory ? `${item.files} ${item.files === 1 ? 'file' : 'files'} · ${languages(item.languages)}` : `${item.location.kind} · ${languages(item.languages)}`);count.className = 'entry-count';button.append(count);
      if (item.examples.length) {const names = node('span', item.examples.join(', '));names.className = 'entry-names';button.append(names);}
      if (item.location && el('discover-search').value.trim()) {const source = node('span', item.path);source.className = 'entry-path';button.append(source);}
      if (directory) {const weight = node('span', '');weight.className = 'area-weight';const bar = node('span', '');bar.style.width = `${100 * item.files / Math.max(data.scope_files, 1)}%`;weight.append(bar);button.append(weight);}
      button.onclick = () => directory ? navigate(item.path) : inspect({...item, snapshot: data.snapshot});
      container.append(button);
    }
    el('discovery-empty').hidden = data.entries.length > 0;
    if (data.scope_files === 0) el('discovery-empty').textContent = 'No source is indexed in this area. Return to the repository or check the configured source scope.';
    else el('discovery-empty').textContent = 'No matching names or files. Try a different word or return to the repository.';
    el('discovery-back').hidden = offset === 0;el('discovery-more').hidden = data.next == null;
    el('discovery-counts').hidden = offset === 0 && data.next == null;
    el('discovery-counts').textContent = `${data.entries.length ? offset + 1 : 0}–${offset + data.entries.length} of ${data.matching} ${el('discover-search').value.trim() ? 'matches' : 'areas and files'}`;
  }
  async function load() {
    if (!status?.snapshot) return;
    const token = ++request;areaController?.abort();areaController = new AbortController();
    el('discovery-areas').setAttribute('aria-busy', 'true');
    try {
      const next = await transport(`/api/discovery?${new URLSearchParams({path, search: el('discover-search').value, offset: String(offset)})}`, {signal: areaController.signal});
      if (token !== request) return;
      if (!Array.isArray(next.entries) || next.entries.length > 24) throw new Error('Invalid discovery page');
      data = next;render();
    } catch (e) {if (e.name !== 'AbortError' && token === request) showError(e);}
    finally {if (token === request) el('discovery-areas').setAttribute('aria-busy', 'false');}
  }
  function navigate(target) {clearTimeout(searchTimer);path = target;offset = 0;el('discover-search').value = '';void load();}
  async function inspect(item, namesOffset = item.location.index ?? 0) {
    const token = ++previewRequest;previewController?.abort();previewController = new AbortController();
    preview = {...item, location: {...item.location}, available: false};
    el('source-preview').hidden = false;el('preview-title').textContent = item.label;el('preview-path').textContent = item.path;
    el('preview-source').textContent = '';el('preview-status').textContent = 'Loading captured source…';el('preview-names').replaceChildren();el('preview-more-names').hidden = true;syncSaved();
    try {
      const location = item.location;
      const query = {snapshot: item.snapshot, file: location.file, version: location.version, line_start: String(location.line), line_count: '12', outline_start: String(namesOffset), outline_count: '12'};
      const window = await transport(`/api/window?${new URLSearchParams(query)}`, {signal: previewController.signal});
      if (token !== previewRequest) return;
      if (window.snapshot !== item.snapshot || window.file !== location.file || window.version !== location.version || window.lines.length > 12 || window.outline.length > 12) throw new Error('Source preview version differs');
      preview.available = window.line_total > 0;
      el('preview-status').textContent = preview.available ? `Captured lines ${window.line_start}–${window.lines.at(-1)?.number ?? window.line_start} of ${window.line_total}` : 'Captured source has no supported text preview.';
      for (const row of window.lines) {const line = node('span', ''), number = node('span', `${row.number}  `), text = node('span', '');number.className = 'preview-line-number';renderStyledText(text, row);line.append(number, text);el('preview-source').append(line, document.createTextNode('\n'));}
      for (const entry of window.outline) {
        const button = node('button', ''), kind = node('span', entry.kind+' ');button.type = 'button';kind.className = 'kind-label';kind.dataset.kind = entry.kind;button.append(kind, document.createTextNode(`${entry.name}${entry.scope ? ` (${entry.scope})` : ''}`));
        button.onclick = () => inspect({...item, label: entry.name, location: {...location, line: entry.start_line, index: entry.index, kind: entry.kind}}, namesOffset);
        el('preview-names').append(button);
      }
      nextNames = window.outline_start + window.outline.length;
      el('preview-more-names').hidden = nextNames >= window.outline_total;
      syncSaved();
    } catch (e) {if (e.name !== 'AbortError' && token === previewRequest) {el('preview-status').textContent = e.message;showError(e);}}
  }
  el('discover-search').addEventListener('input', () => {++request;areaController?.abort();clearTimeout(searchTimer);offset = 0;searchTimer = setTimeout(load, 180);});
  el('discovery-back').onclick = () => {offset = Math.max(0, offset - 24);void load();};
  el('discovery-more').onclick = () => {if (data?.next != null) {offset = data.next;void load();}};
  el('close-preview').onclick = () => {++previewRequest;previewController?.abort();preview = undefined;el('source-preview').hidden = true;};
  el('preview-more-names').onclick = () => {if (preview) void inspect(preview, nextNames);};
  el('keep-discovered').onclick = async () => {
    if (!board || !preview?.available || saving || savedLocation()) return;
    const selected = preview;saving = true;syncSaved();
    try {await keep({snapshot: selected.snapshot, ...selected.location, title: selected.label});}
    catch (e) {showError(e);}finally {saving = false;syncSaved();}
  };
  el('preview-editor').onclick = () => {const saved = savedLocation();if (saved) void openEditor(saved.id);};
  breadcrumbs();
  return {
    inspect,
    receiveStatus(next) {
      status = next;el('repository-title').textContent = next.name || 'Explore this repository';
      if (browseEnabled && next.snapshot && lastSnapshot !== next.snapshot) {lastSnapshot = next.snapshot;void load();}
      if (next.error && !next.snapshot) el('discovery-summary').textContent = 'Source indexing failed. Retry to build the overview.';
    },
    saved(view) {board = view.board;syncSaved();},
    closePreview() {el('close-preview').click();},
  };
}
