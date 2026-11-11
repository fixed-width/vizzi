import {SourcePane} from './source-pane.js';
import {renderCallText} from './call-actions.js';
import {originLabel} from './source-origin.js';
import {functionIndent} from './source-margin.js';

const qualified = symbol => [...symbol.parents, symbol.name].join('::');

export function installReferenceStacks(root, {read, post, transport, open, changed, failed}) {
 const document = root.ownerDocument;
 const records = new Map();
 let queue = Promise.resolve();
 function element(tag, text = '') {
  const item = document.createElement(tag);
  item.textContent = text;
  return item;
 }
 function button(text, action) {
  const item = element('button', text);
  item.type = 'button';
  item.onclick = action;
  return item;
 }
 function current(record) { return records.get(record.id) === record; }

 async function preview(record, row) {
  record.previewController?.abort();
  const controller = record.previewController = new AbortController();
  if (!record.data || record.data.snapshot !== read().snapshot) return;
  const source = row.symbol?.source ?? row.source;
  const snapshot = row.origin?.capture ?? record.data.snapshot;
  const start = row.symbol ? Math.max(source.line, row.source.line - 12) : Math.max(1, row.source.line - 3);
  const count = row.symbol ? Math.min(96, source.end_line - start + 1) : 16;
  const query = new URLSearchParams({snapshot, file: source.file, version: source.version, line_start: String(start), line_count: String(count), outline_count: '1', reflow: 'true'});
  try {
   const data = await transport((row.origin ? '/api/dependency/window?' : '/api/window?') + query, {signal: controller.signal});
   if (!current(record) || controller !== record.previewController || record.data.snapshot !== read().snapshot) return;
   if (data.snapshot !== snapshot || data.file !== source.file || data.version !== source.version || !Array.isArray(data.lines) || data.lines.length > 96) throw new Error('Reference preview source differs');
   record.previewTitle.textContent = row.symbol ? qualified(row.symbol) : source.path;
   record.previewPane.set(data.lines, {top: 0, left: 0}, functionIndent(data.lines[0]));
   for (const line of record.preview.querySelectorAll('.code-line')) line.classList.toggle('is-reference-line', Number(line.dataset.line) === row.source.line);
   record.preview.hidden = false;
  } catch (error) {
   if (error.name !== 'AbortError' && current(record)) {
    record.previewTitle.textContent = error.message;
    record.previewPane.set([]);
    record.preview.hidden = false;
   }
  }
 }

 function paint(record) {
  const data = record.data;
  record.list.replaceChildren();
  const stale = data.snapshot !== read().snapshot || read().stale;
  record.stale = stale;
  for (const row of data.items) {
   const entry = element('div');
   entry.className = 'reference-entry';
   const label = row.symbol ? qualified(row.symbol) : 'Reference outside an indexed declaration';
   const show = button(label, () => {
    if (!stale && row.symbol) open(record.id, row, data);
   });
   show.dataset.referenceRow = row.id;
   show.disabled = stale || !row.symbol;
   const location = element('small', `${row.origin ? originLabel(row.origin) + ' · ' : ''}${row.source.path}:${row.source.line}`);
   const snippet = element('code', row.snippet);
   show.append(location, snippet);
   show.onmouseenter = () => {
    clearTimeout(record.hoverTimer);
    record.hoverTimer = setTimeout(() => void preview(record, row), 120);
   };
   show.onmouseleave = () => clearTimeout(record.hoverTimer);
   show.onfocus = () => void preview(record, row);
   entry.append(show);
   record.list.append(entry);
  }
  const shown = data.items.length ? `${record.offset + 1}–${record.offset + data.items.length} of ${data.matching}` : '0 results';
  record.status.textContent = stale ? 'Repository updated. Refresh results.' : `${shown} · ${data.precision === 'semantic' ? 'Language references' : 'Name-based candidates'}${data.outside ? ` · ${data.outside} outside captured source` : ''}${data.omitted ? ` · ${data.omitted} omitted` : ''}`;
  record.status.title = data.detail ?? '';
  record.previous.disabled = stale || record.offset === 0;
  record.next.disabled = stale || data.next === null;
 }

 function request(record, offset = 0, cached = false) {
  record.controller?.abort();
  const controller = record.controller = new AbortController();
  record.status.textContent = 'Finding references…';
  queue = queue.catch(() => {}).then(async () => {
   if (!current(record) || controller.signal.aborted) return;
   const model = read();
   if (model.stale) {
    record.pendingRefresh = true;
    record.controller = null;
    record.status.textContent = 'Waiting for current source…';
    return;
   }
   const item = model.view(record.id);
   const target = item.target;
   try {
    const data = await post('/api/references', {snapshot: model.snapshot, symbol: target.id, tracking: target.tracking ?? null, file: target.source.file, context: target.source.context, capture: target.origin?.capture ?? null, kind: item.query, offset, query: cached ? record.data?.query : null}, controller.signal);
    if (!current(record) || controller !== record.controller || model !== read() || data.snapshot !== read().snapshot) return;
    if (!Array.isArray(data.items) || data.items.length > 8 || !['semantic', 'name_candidates'].includes(data.precision) || !Number.isSafeInteger(data.matching) || data.matching > 256) throw new Error('Invalid reference results');
    item.target = data.target;
    item.snapshot = data.snapshot;
    record.data = data;
    record.offset = offset;
    paint(record);
    changed();
   } catch (error) {
    if (error.name !== 'AbortError' && current(record)) record.status.textContent = error.message + ' Refresh results to retry.';
   }
  });
 }

 function drag(handle, id, resizing = false) {
  handle.tabIndex = 0;
  handle.onpointerdown = event => {
   if (event.button !== 0 || (!resizing && event.target.closest('button'))) return;
   event.preventDefault();
   handle.setPointerCapture(event.pointerId);
   const item = read().view(id), start = {x: event.clientX, y: event.clientY, left: item.x, top: item.y, w: item.w, h: item.h};
   const move = event => {
    try {
     const dx = (event.clientX - start.x) / read().camera.scale, dy = (event.clientY - start.y) / read().camera.scale;
     if (resizing) read().resize(id, Math.max(320, Math.min(1600, start.w + dx)), Math.max(200, Math.min(2400, start.h + dy)));
     else read().move(id, start.left + dx, start.top + dy);
     changed();
    } catch (error) { failed(error); }
   };
   const end = () => { handle.removeEventListener('pointermove', move); handle.removeEventListener('pointerup', end); handle.removeEventListener('pointercancel', end); };
   handle.addEventListener('pointermove', move);
   handle.addEventListener('pointerup', end);
   handle.addEventListener('pointercancel', end);
  };
  handle.onkeydown = event => {
   if (event.target !== handle) return;
   const delta = {ArrowLeft: [-16, 0], ArrowRight: [16, 0], ArrowUp: [0, -16], ArrowDown: [0, 16]}[event.key];
   if (!delta) return;
   event.preventDefault();
   try {
    const item = read().view(id);
    if (resizing) read().resize(id, Math.max(320, Math.min(1600, item.w + delta[0])), Math.max(200, Math.min(2400, item.h + delta[1])));
    else read().move(id, item.x + delta[0], item.y + delta[1]);
    changed();
   } catch (error) { failed(error); }
  };
 }

 function dispose(record) {
  record.controller?.abort();
  record.previewController?.abort();
  clearTimeout(record.hoverTimer);
  record.previewPane.dispose();
  record.element.remove();
 }
 return {
  reset() { for (const record of records.values()) dispose(record); records.clear(); queue = Promise.resolve(); },
  refresh() {
   const model = read();
   const ids = new Set(model?.references.map(item => item.id));
   for (const [id, record] of records) if (!ids.has(id)) { dispose(record); records.delete(id); }
   for (const item of model?.references ?? []) {
    let record = records.get(item.id);
    if (!record) {
     const container = element('section');
     container.className = 'reference-stack';
     container.dataset.referenceStack = item.id;
     const header = element('div');
     header.className = 'fragment-header';
     const heading = element('strong');
     const refresh = button('Refresh', () => request(record));
     const collapse = button('Collapse', () => { read().collapse(item.id); changed(); });
     const close = button('×', () => { read().close(item.id); changed(); });
     close.setAttribute('aria-label', 'Close reference stack');
     header.append(heading, refresh, collapse, close);
     drag(header, item.id);
     const body = element('div');
     body.className = 'reference-body';
     const status = element('p');
     status.setAttribute('role', 'status');
     const list = element('div');
     list.className = 'reference-list';
     const paging = element('div');
     const previous = button('Previous', () => request(record, Math.max(0, record.offset - 8), true));
     const next = button('Next', () => request(record, record.data.next, true));
     previous.disabled = next.disabled = true;
     paging.append(previous, next);
     const previewBox = element('section');
     previewBox.className = 'reference-preview';
     previewBox.hidden = true;
     const previewTitle = element('strong');
     const previewCode = element('div');
     previewCode.className = 'code-pane';
     const previewPane = new SourcePane(previewCode, (text, line, plan, margin) => renderCallText(text, line, [], () => {}, plan, margin));
     previewBox.append(previewTitle, previewCode);
     const resize = button('', () => {});
     resize.className = 'resize-handle';
     resize.setAttribute('aria-label', 'Resize reference stack');
     drag(resize, item.id, true);
     body.append(status, list, paging, previewBox);
     container.append(header, body, resize);
     root.append(container);
     record = {id: item.id, element: container, heading, collapse, refreshButton: refresh, body, status, list, previous, next, preview: previewBox, previewTitle, previewPane, offset: 0, data: null};
     records.set(item.id, record);
     if (!model.stale) request(record);
    }
    record.element.hidden = !model.isVisible(item.id);
    Object.assign(record.element.style, {left: item.x + 'px', top: item.y + 'px', width: item.w + 'px', height: (item.collapsed ? 54 : item.h) + 'px'});
    record.heading.textContent = `${item.query === 'callers' ? 'Callers' : 'References'} of ${qualified(item.target)}`;
    record.heading.title = record.heading.textContent;
    record.element.setAttribute('aria-label', record.heading.textContent);
    record.body.hidden = item.collapsed;
    record.refreshButton.disabled = model.stale;
    record.collapse.textContent = item.collapsed ? 'Expand' : 'Collapse';
    if (record.data && record.stale !== (record.data.snapshot !== model.snapshot || model.stale)) paint(record);
    if (record.pendingRefresh && !model.stale) { record.pendingRefresh = false; request(record); }
    if (!record.data && !record.controller && !model.stale) request(record);
   }
  },
 };
}
