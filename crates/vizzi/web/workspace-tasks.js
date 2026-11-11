import {TaskWorkspace} from './task-workspace.js';
import {TaskShelf} from './task-shelf.js';

function button(document, text, action) {
 const element = document.createElement('button');
 element.type = 'button';
 element.textContent = text;
 element.onclick = action;
 return element;
}

export function installTaskControls({read, replace, snapshot, changed, failed, storage = localStorage, document = globalThis.document}) {
 let shelf = null, key = null, renameId = null;
 const el = id => document.getElementById(id);
 const title = () => { el('task-title').textContent = shelf?.current().title ?? 'Untitled task'; };

 function write(next) {
  if (!key) throw new Error('Saved tasks could not be loaded. Existing storage is retained.');
  storage.setItem(key, next.serialize());
  shelf = next;
  title();
 }

 function save() {
  if (!shelf || !read()) return;
  try {
   const next = shelf.fork();
   next.update(read().serialize());
   write(next);
  } catch (error) { failed(error); }
 }

 function transaction(edit, activate = false) {
  try {
   const next = shelf.fork();
   next.update(read().serialize());
   edit(next);
   const model = activate ? TaskWorkspace.restore(next.current().layout, snapshot()) : null;
   write(next);
   if (model) replace(model);
   return true;
  } catch (error) { failed(error); return false; }
 }

 function showRename(id) {
  renameId = id;
  el('task-name').value = shelf.tasks.find(task => task.id === id).title;
  el('task-name-dialog').showModal();
  el('task-name').focus();
  el('task-name').select();
 }

 function renderShelf() {
  const list = el('task-list');
  list.replaceChildren();
  for (const task of shelf.tasks) {
   const row = document.createElement('div');
   row.className = 'task-row';
   const open = button(document, task.title, () => {
    if (task.id === shelf.active || transaction(next => next.open(task.id), true)) el('task-shelf-dialog').close();
   });
   open.dataset.task = task.id;
   if (task.id === shelf.active) open.setAttribute('aria-current', 'true');
   const date = document.createElement('small');
   date.textContent = new Date(task.updated).toLocaleString();
   const rename = button(document, 'Rename', () => showRename(task.id));
   rename.setAttribute('aria-label', 'Rename task ' + task.title);
   const remove = button(document, 'Delete', () => {
    transaction(next => next.remove(task.id));
    renderShelf();
   });
   remove.setAttribute('aria-label', 'Delete task ' + task.title);
   remove.disabled = task.id === shelf.active;
   row.append(open, date, rename, remove);
   list.append(row);
  }
 }

 el('new-workspace').onclick = () => {
  el('workspace-actions').open = false;
  transaction(next => next.create(new TaskWorkspace(snapshot()).serialize()), true);
 };
 el('rename-task').onclick = () => {
  if (shelf) showRename(shelf.active);
  el('workspace-actions').open = false;
 };
 el('show-task-shelf').onclick = () => {
  if (!shelf) return;
  save();
  renderShelf();
  el('workspace-actions').open = false;
  el('task-shelf-dialog').showModal();
 };
 el('task-name-form').onsubmit = event => {
  event.preventDefault();
  if (!transaction(next => next.rename(renameId, el('task-name').value))) return;
  el('task-name-dialog').close();
  if (el('task-shelf-dialog').open) renderShelf();
 };

 const groupChoice = el('group-choice');
 function groupFields() {
  const model = read(), id = groupChoice.value;
  const group = id ? model.group(id) : null;
  el('group-name').value = group?.title ?? '';
  const members = el('group-members');
  members.replaceChildren();
  const legend = document.createElement('legend');
  legend.textContent = 'Members';
  members.append(legend);
  for (const item of model.items()) {
   const label = document.createElement('label');
   const input = document.createElement('input');
   input.type = 'checkbox';
   input.value = item.id;
   input.checked = group?.members.includes(item.id) ?? false;
   const name = item.kind === 'source' ? [...item.symbol.parents, item.symbol.name].join('::') : `${item.query === 'callers' ? 'Callers' : 'References'} of ${item.target.name}`;
   const currentGroup = model.groupFor(item.id);
   label.append(input, document.createTextNode(name + (currentGroup && currentGroup.id !== id ? ' · ' + currentGroup.title : '')));
   members.append(label);
  }
 }
 function showGroups(id = '') {
  if (!read()) return;
  groupChoice.replaceChildren();
  const fresh = document.createElement('option');
  fresh.value = '';
  fresh.textContent = 'New group';
  groupChoice.append(fresh);
  for (const group of read().groups) {
   const option = document.createElement('option');
   option.value = group.id;
   option.textContent = group.title;
   groupChoice.append(option);
  }
  groupChoice.value = id;
  groupFields();
  el('workspace-actions').open = false;
  el('group-dialog').showModal();
 }
 groupChoice.onchange = groupFields;
 el('show-groups').onclick = () => showGroups();
 el('group-form').onsubmit = event => {
  event.preventDefault();
  try {
   const members = [...el('group-members').querySelectorAll('input:checked')].map(input => input.value);
   read().saveGroup(groupChoice.value || null, el('group-name').value, members);
   el('group-dialog').close();
   changed();
  } catch (error) { failed(error); }
 };
 for (const close of document.querySelectorAll('[data-close-dialog]')) close.onclick = () => el(close.dataset.closeDialog).close();

 return {
  save,
  showGroups,
  load(name, currentSnapshot) {
   key = 'vizzi.visual.tasks.v1.' + name;
   const raw = storage.getItem(key);
   try {
    const legacy = storage.getItem('vizzi.visual.v1.' + name);
    shelf = raw ? TaskShelf.restore(raw) : new TaskShelf(legacy ?? new TaskWorkspace(currentSnapshot).serialize());
    const model = TaskWorkspace.restore(shelf.current().layout, currentSnapshot);
    title();
    return model;
   } catch (error) {
    key = null;
    shelf = new TaskShelf(new TaskWorkspace(currentSnapshot).serialize());
    title();
    throw error;
   }
  },
 };
}

export function installTaskGroups(root, {read, changed, edit, failed}) {
 const document = root.ownerDocument, records = new Map();
 return {
  refresh() {
   const model = read();
   const ids = new Set(model?.groups.map(group => group.id));
   for (const [id, record] of records) if (!ids.has(id)) { record.element.remove(); records.delete(id); }
   for (const group of model?.groups ?? []) {
    let record = records.get(group.id);
    if (!record) {
     const element = document.createElement('section');
     element.className = 'task-group';
     element.dataset.group = group.id;
     const header = document.createElement('div');
     header.className = 'task-group-header';
     header.tabIndex = 0;
     const name = document.createElement('strong');
     const collapse = button(document, 'Collapse', () => { read().collapseGroup(group.id, !read().group(group.id).collapsed); changed(); });
     const rename = button(document, 'Edit', () => edit(group.id));
     const ungroup = button(document, 'Ungroup', () => { read().ungroup(group.id); changed(); });
     const close = button(document, 'Close', () => { read().closeGroup(group.id); changed(); });
     header.append(name, collapse, rename, ungroup, close);
     header.addEventListener('pointerdown', event => {
      if (event.button !== 0 || event.target.closest('button')) return;
      event.preventDefault();
      header.setPointerCapture(event.pointerId);
      let x = event.clientX, y = event.clientY;
      const move = event => {
       try { read().moveGroup(group.id, (event.clientX - x) / read().camera.scale, (event.clientY - y) / read().camera.scale); x = event.clientX; y = event.clientY; changed(); } catch (error) { failed(error); }
      };
      const end = () => { header.removeEventListener('pointermove', move); header.removeEventListener('pointerup', end); header.removeEventListener('pointercancel', end); };
      header.addEventListener('pointermove', move);
      header.addEventListener('pointerup', end);
      header.addEventListener('pointercancel', end);
     });
     header.onkeydown = event => {
      if (event.target !== header) return;
      const delta = {ArrowLeft: [-16, 0], ArrowRight: [16, 0], ArrowUp: [0, -16], ArrowDown: [0, 16]}[event.key];
      if (delta) { event.preventDefault(); try { read().moveGroup(group.id, ...delta); changed(); } catch (error) { failed(error); } }
     };
     element.append(header);
     root.append(element);
     record = {element, header, name, collapse, rename, ungroup, close};
     records.set(group.id, record);
    }
    const bounds = model.groupBounds(group.id);
    Object.assign(record.element.style, {left: bounds.x + 'px', top: bounds.y + 'px', width: (group.collapsed ? Math.min(480, bounds.w) : bounds.w) + 'px', height: (group.collapsed ? 54 : bounds.h) + 'px'});
    record.element.classList.toggle('collapsed', group.collapsed);
    record.name.textContent = group.title;
    record.name.title = group.title;
    record.header.setAttribute('aria-label', 'Move group ' + group.title);
    for (const [name, control] of [['Edit', record.rename], ['Ungroup', record.ungroup], ['Close', record.close]]) control.setAttribute('aria-label', `${name} group ${group.title}`);
    record.collapse.textContent = group.collapsed ? 'Expand' : 'Collapse';
    record.collapse.setAttribute('aria-label', `${group.collapsed ? 'Expand' : 'Collapse'} group ${group.title}`);
   }
  },
 };
}
