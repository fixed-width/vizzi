import {TaskWorkspace} from './task-workspace.js';

const LIMIT = 3 * 1024 * 1024;
const bytes = value => new TextEncoder().encode(value).length;
const require = (ok, message) => { if (!ok) throw new Error(message); };
const validTitle = title => typeof title === 'string' && title.trim().length > 0 && bytes(title) <= 256;

function validateLayout(layout) {
 require(typeof layout === 'string', 'Invalid saved task layout');
 const data = JSON.parse(layout);
 TaskWorkspace.restore(layout, data.snapshot);
}

export class TaskShelf {
 constructor(layout) {
  validateLayout(layout);
  this.active = 't1';
  this.next = 2;
  this.tasks = [{id: 't1', title: 'Untitled task', updated: Date.now(), layout}];
 }

 current() { return this.tasks.find(task => task.id === this.active); }

 fork() {
  const shelf = Object.create(TaskShelf.prototype);
  Object.assign(shelf, {active: this.active, next: this.next, tasks: this.tasks.map(task => ({...task}))});
  return shelf;
 }

 commit(tasks, active = this.active, next = this.next) {
  const raw = JSON.stringify({schema: 1, tasks, active, next});
  require(bytes(raw) <= LIMIT, 'Task shelf is full. Delete a saved task to make room.');
  this.tasks = tasks;
  this.active = active;
  this.next = next;
 }

 update(layout) {
  validateLayout(layout);
  this.commit(this.tasks.map(task => task.id === this.active ? {...task, layout, updated: Date.now()} : task));
 }

 create(layout, title = 'Untitled task') {
  validateLayout(layout);
  require(validTitle(title), 'Give the task a name of at most 256 bytes');
  require(this.tasks.length < 12 && this.next < 1000000, 'Task shelf is full. Delete a saved task to make room.');
  const id = 't' + this.next;
  this.commit([...this.tasks, {id, title: title.trim(), updated: Date.now(), layout}], id, this.next + 1);
  return id;
 }

 open(id) {
  const task = this.tasks.find(task => task.id === id);
  require(task, 'Saved task is absent');
  this.active = id;
  return task.layout;
 }

 rename(id, title) {
  require(this.tasks.some(task => task.id === id) && validTitle(title), 'Give the task a name of at most 256 bytes');
  this.commit(this.tasks.map(task => task.id === id ? {...task, title: title.trim(), updated: Date.now()} : task));
 }

 remove(id) {
  require(id !== this.active, 'Open another task before deleting the current task');
  require(this.tasks.some(task => task.id === id), 'Saved task is absent');
  this.commit(this.tasks.filter(task => task.id !== id));
 }

 serialize() {
  return JSON.stringify({schema: 1, active: this.active, next: this.next, tasks: this.tasks});
 }

 static restore(raw) {
  require(typeof raw === 'string' && bytes(raw) <= LIMIT, 'Invalid task shelf size');
  const data = JSON.parse(raw);
  require(data && data.schema === 1 && Array.isArray(data.tasks) && data.tasks.length > 0 && data.tasks.length <= 12 && Number.isSafeInteger(data.next) && data.next >= 2 && data.next <= 1000000, 'Invalid task shelf');
  const ids = new Set();
  for (const task of data.tasks) {
   require(task && /^t[1-9][0-9]{0,5}$/.test(task.id) && Number(task.id.slice(1)) < data.next && !ids.has(task.id) && validTitle(task.title) && Number.isSafeInteger(task.updated) && task.updated >= 0, 'Invalid saved task');
   validateLayout(task.layout);
   ids.add(task.id);
  }
  require(ids.has(data.active), 'Current saved task is absent');
  const shelf = Object.create(TaskShelf.prototype);
  Object.assign(shelf, {active: data.active, next: data.next, tasks: structuredClone(data.tasks)});
  return shelf;
 }
}
