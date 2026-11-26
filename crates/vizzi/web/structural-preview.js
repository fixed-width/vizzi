import {renderStyledText} from '/presentation.js';

export function installStructuralPreview({transport, keep, openEditor, showError}) {
  const el = id => document.getElementById(id);
  const node = (tag, text) => {const n = document.createElement(tag);n.textContent = text;return n;};
  let board, preview, previewRequest = 0, saving = false, nextNames = 0;
  let previewController;
  function savedLocation() {
    return board?.evidence.find(e => preview && e.source.file === preview.location.file && e.source.version === preview.location.version && e.source.line === preview.location.line);
  }
  function syncSaved() {
    const saved = savedLocation();
    el('keep-discovered').textContent = saved ? 'Saved' : 'Save location';
    el('keep-discovered').disabled = !board || saving || !preview?.available || Boolean(saved);
    el('preview-editor').hidden = !saved;
  }
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
  el('close-preview').onclick = () => {++previewRequest;previewController?.abort();preview = undefined;el('source-preview').hidden = true;};
  el('preview-more-names').onclick = () => {if (preview) void inspect(preview, nextNames);};
  el('keep-discovered').onclick = async () => {
    if (!board || !preview?.available || saving || savedLocation()) return;
    const selected = preview;saving = true;syncSaved();
    try {await keep({snapshot: selected.snapshot, ...selected.location, title: selected.label});}
    catch (e) {showError(e);}finally {saving = false;syncSaved();}
  };
  el('preview-editor').onclick = () => {const saved = savedLocation();if (saved) void openEditor(saved.id);};
  return {
    inspect,
    saved(view) {el('preview-editor').textContent = `Open in ${view.editor}`;board = view.board;syncSaved();},
    close() {el('close-preview').click();},
  };
}
