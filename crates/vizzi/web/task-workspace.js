import {Workspace} from './visual-model.js';
import {fitViews, BUBBLE_SIZE} from './layout-growth.js';

const copy = value => structuredClone(value);
const bytes = text => new TextEncoder().encode(text).length;
const require = (ok, message) => { if (!ok) throw new Error(message); };
const titleValid = title => typeof title === 'string' && title.trim().length > 0 && bytes(title) <= 256;
const coordinate = value => Number.isFinite(value) && Math.abs(value) <= 100000;
const height = item => item.collapsed ? 54 : item.h;

export class TaskWorkspace extends Workspace {
 constructor(snapshot) {
  super(snapshot);
  this.references = [];
  this.groups = [];
  this.nextReference = 1;
  this.nextGroup = 1;
 }

 items() { return [...this.views, ...this.references]; }

 view(id) {
  const reference = this.references.find(item => item.id === id);
  return reference ?? super.view(id);
 }

 group(id) {
  const group = this.groups.find(item => item.id === id);
  require(group, 'Task group is absent');
  return group;
 }

 groupFor(id) { return this.groups.find(group => group.members.includes(id)); }
 isVisible(id) { return !this.groupFor(id)?.collapsed; }

 groupBounds(id) {
  const members = this.group(id).members.map(member => this.view(member));
  const x = Math.min(...members.map(item => item.x)) - 16;
  const y = Math.min(...members.map(item => item.y)) - 42;
  return {
   x, y,
   w: Math.max(...members.map(item => item.x + item.w)) - x + 16,
   h: Math.max(...members.map(item => item.y + height(item))) - y + 16,
  };
 }

 visibleItems() {
  const items = this.items().filter(item => this.isVisible(item.id));
  for (const group of this.groups.filter(group => group.collapsed)) {
   const bounds = this.groupBounds(group.id);
   items.push({id: group.id, kind: 'group', title: group.title, ...bounds, w: Math.min(bounds.w, 480), h: 54});
  }
  return items;
 }

 place(x, y, w, h) {
  for (const item of this.visibleItems().toSorted((a, b) => a.y - b.y)) {
   if (x < item.x + item.w + 24 && item.x < x + w + 24 && y < item.y + height(item) + 24 && item.y < y + h + 24) {
    y = item.y + height(item) + 32;
   }
  }
  return {x, y};
 }

 add(symbol, area, parent) {
  const before = new Set(this.views.map(view => view.id));
  const id = super.add(symbol, area, parent);
  const group = parent && this.groupFor(parent.id);
  if (group && !before.has(id)) group.members.push(id);
  return id;
 }

 addReferences(target, query, parentId) {
  require(['callers', 'references'].includes(query), 'Invalid reference query');
  const validator = new Workspace(this.snapshot);
  validator.openRoot(target, '');
  const existing = this.references.find(item => item.target.id === target.id && item.query === query);
  if (existing) return existing.id;
  require(this.references.length < 4, 'Close a result stack before opening another');
  const parent = this.view(parentId);
  const position = this.place(parent.x + parent.w + 48, parent.y, 440, 400);
  const id = 'r' + this.nextReference++;
  this.references.push({id, kind: 'references', target: copy(target), query, snapshot: this.snapshot, ...position, w: 440, h: 400, collapsed: false});
  const group = this.groupFor(parentId);
  if (group) group.members.push(id);
  return id;
 }

 openReference(id, row, target) {
  this.fresh();
  require(row?.symbol, 'This reference has no admitted declaration');
  const planned = TaskWorkspace.restore(this.serialize(), this.snapshot);
  const stack = planned.view(id);
  require(stack.kind === 'references', 'Reference stack is absent');
  const existingCaller = planned.views.some(view => view.symbol.id === row.symbol.id && view.snapshot === planned.snapshot);
  const existingTarget = planned.views.find(view => view.symbol.id === target.id && view.snapshot === planned.snapshot);
  const caller = planned.add(row.symbol, row.symbol.source.path, stack);
  if (!existingCaller && existingTarget) {
   const position = planned.place(existingTarget.x - planned.view(caller).w - 72, existingTarget.y, planned.view(caller).w, planned.view(caller).h);
   planned.move(caller, position.x, position.y);
  }
  let destination = null;
  if (row.site) destination = planned.follow(caller, row.site, target);
  planned.selected = caller;
  Object.assign(this, planned);
  return {caller, destination};
 }

 saveGroup(id, title, members) {
  require(titleValid(title), 'Give the group a name of at most 256 bytes');
  require(Array.isArray(members) && members.length > 0 && new Set(members).size === members.length, 'Choose distinct bubbles for the group');
  for (const member of members) this.view(member);
  require(id ? this.groups.some(group => group.id === id) : this.groups.length < 8, 'Task group allowance reached');
  const groupId = id ?? 'g' + this.nextGroup;
  const old = id && this.group(id);
  const next = this.groups.filter(group => group.id !== id).map(group => ({...group, members: group.members.filter(member => !members.includes(member))})).filter(group => group.members.length);
  next.push({id: groupId, title: title.trim(), members: [...members], collapsed: old?.collapsed ?? false});
  this.groups = next;
  if (!id) this.nextGroup++;
  return groupId;
 }

 moveGroup(id, dx, dy) {
  require(Number.isFinite(dx) && Number.isFinite(dy), 'Invalid group movement');
  const planned = this.group(id).members.map(member => {
   const item = this.view(member);
   return {item, x: item.x + dx, y: item.y + dy};
  });
  require(planned.every(item => coordinate(item.x) && coordinate(item.y)), 'Group movement exceeds workspace bounds');
  for (const {item, x, y} of planned) Object.assign(item, {x, y});
 }

 collapseGroup(id, collapsed) { this.group(id).collapsed = Boolean(collapsed); }
 ungroup(id) { this.group(id); this.groups = this.groups.filter(group => group.id !== id); }

 closeGroup(id) {
  const members = [...this.group(id).members];
  for (const member of members) this.close(member);
 }

 collapse(id) {
  const item = this.view(id);
  if (item.kind === 'source') { super.collapse(id); return; }
  if (item.collapsed) this.fitSize(id, 'height', item.h);
  else item.collapsed = true;
 }

 close(id) {
  if (this.references.some(item => item.id === id)) this.references = this.references.filter(item => item.id !== id);
  else super.close(id);
  this.groups = this.groups.map(group => ({...group, members: group.members.filter(member => member !== id)})).filter(group => group.members.length);
 }

 fitSize(id, axis, extent) {
  const next = fitViews(this.items(), id, axis, extent);
  this.views = next.filter(item => item.kind === 'source');
  this.references = next.filter(item => item.kind === 'references');
  if (this.views.some(view => view.id === id)) this.selected = id;
 }

 reveal(ids, width, viewportHeight) {
  for (const id of ids) {
   const group = this.groupFor(id);
   if (group) group.collapsed = false;
  }
  super.reveal(ids, width, viewportHeight);
 }

 fit(width, viewportHeight) {
  const items = this.visibleItems();
  if (!items.length) { super.fit(width, viewportHeight); return; }
  require(Number.isFinite(width) && width > 0 && width <= 100000 && Number.isFinite(viewportHeight) && viewportHeight > 0 && viewportHeight <= 100000, 'Invalid viewport');
  const x = Math.min(...items.map(item => item.x));
  const y = Math.min(...items.map(item => item.y));
  const right = Math.max(...items.map(item => item.x + item.w));
  const bottom = Math.max(...items.map(item => item.y + height(item)));
  const scale = Math.max(.02, Math.min(1, (width - 64) / (right - x), (viewportHeight - 64) / (bottom - y)));
  this.camera = {x: (x + right) / 2 - width / (2 * scale), y: (y + bottom) / 2 - viewportHeight / (2 * scale), scale};
 }

 serialize() {
  const data = JSON.parse(super.serialize());
  data.collections = {references: this.references, groups: this.groups, nextReference: this.nextReference, nextGroup: this.nextGroup};
  const raw = JSON.stringify(data);
  require(bytes(raw) <= 524288, 'Task layout exceeds its storage allowance');
  return raw;
 }

 static restore(raw, snapshot) {
  const base = Workspace.restore(raw, snapshot);
  const model = Object.assign(new TaskWorkspace(base.snapshot), base);
  const collections = JSON.parse(raw).collections;
  if (collections === undefined) return model;
  require(collections && Array.isArray(collections.references) && collections.references.length <= 4 && Array.isArray(collections.groups) && collections.groups.length <= 8, 'Invalid task collections');
  require(Number.isSafeInteger(collections.nextReference) && collections.nextReference >= 1 && collections.nextReference <= 1000000 && Number.isSafeInteger(collections.nextGroup) && collections.nextGroup >= 1 && collections.nextGroup <= 1000000, 'Invalid collection identities');
  const ids = new Set(model.views.map(view => view.id));
  for (const item of collections.references) {
   require(item && /^r[1-9][0-9]{0,5}$/.test(item.id) && !ids.has(item.id) && Number(item.id.slice(1)) < collections.nextReference && item.kind === 'references' && ['callers', 'references'].includes(item.query), 'Invalid reference stack');
   const validator = new Workspace(item.snapshot);
   validator.openRoot(item.target, '');
   require(coordinate(item.x) && coordinate(item.y) && Number.isFinite(item.w) && item.w >= 320 && item.w <= BUBBLE_SIZE.maxWidth && Number.isFinite(item.h) && item.h >= 200 && item.h <= BUBBLE_SIZE.maxHeight && typeof item.collapsed === 'boolean', 'Invalid reference geometry');
   ids.add(item.id);
  }
  const groupIds = new Set(), members = new Set();
  for (const group of collections.groups) {
   require(group && /^g[1-9][0-9]{0,5}$/.test(group.id) && !groupIds.has(group.id) && Number(group.id.slice(1)) < collections.nextGroup && titleValid(group.title) && typeof group.collapsed === 'boolean' && Array.isArray(group.members) && group.members.length > 0 && group.members.length <= 28, 'Invalid task group');
   for (const member of group.members) {
    require(ids.has(member) && !members.has(member), 'Invalid task group member');
    members.add(member);
   }
   groupIds.add(group.id);
  }
  model.references = copy(collections.references);
  model.groups = copy(collections.groups);
  model.nextReference = collections.nextReference;
  model.nextGroup = collections.nextGroup;
  return model;
 }
}
