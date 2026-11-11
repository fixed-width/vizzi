import {test} from 'node:test';
import assert from 'node:assert/strict';
import {TaskWorkspace} from './task-workspace.js';
import {TaskShelf} from './task-shelf.js';
const token=n=>n.toString(16).padStart(64,'0');
const layout=()=>new TaskWorkspace(token(1)).serialize();
test('editing the active task preserves the other task and deleting it preserves the active task',()=>{
 const first=new TaskWorkspace(token(1));first.openRoot({id:token(2),name:'first',kind:'function',parents:[],source:{file:token(90),version:token(91),context:token(92),path:'lib.rs',line:1,end_line:2}},'lib.rs');
 const shelf=new TaskShelf(layout()),a=shelf.active,b=shelf.create(first.serialize(),'  Second  ');
 assert.equal(shelf.current().title,'Second');
 const revised=new TaskWorkspace(token(42)).serialize();
 shelf.open(a);shelf.update(revised);shelf.rename(a,'  First  ');
 assert.equal(shelf.tasks.find(t=>t.id===b).title,'Second');
 assert.equal(shelf.tasks.find(t=>t.id===b).layout,first.serialize());
 assert.equal(shelf.current().title,'First');
 assert.equal(shelf.current().layout,revised);
 shelf.remove(b);assert.equal(shelf.current().id,a);assert.equal(shelf.tasks.length,1);
 for(const operation of [()=>new TaskShelf('{}'),()=>shelf.create('{}'),()=>shelf.rename(a,'  '),()=>shelf.create(layout(),'  ')])assert.throws(operation);
});
test('tasks keep independent layouts and names across switching, reload and deletion',()=>{
 const shelf=new TaskShelf(layout()),first=shelf.active;shelf.rename(first,'Trace input');const second=shelf.create(layout(),'Investigate output');assert.notEqual(first,second);assert.equal(shelf.current().title,'Investigate output');shelf.open(first);assert.equal(shelf.current().title,'Trace input');const restored=TaskShelf.restore(shelf.serialize());assert.equal(restored.active,first);assert.equal(restored.tasks.length,2);restored.remove(second);assert.equal(restored.tasks.length,1);assert.throws(()=>restored.remove(first));
});
test('task storage rejects invalid layouts and changes atomically at its bounds',()=>{
 const shelf=new TaskShelf(layout()),before=shelf.serialize();assert.throws(()=>shelf.update('invalid'));assert.equal(shelf.serialize(),before);assert.throws(()=>shelf.rename(shelf.active,''));assert.equal(shelf.serialize(),before);for(let n=1;n<12;n++)shelf.create(layout(),'Task '+n);const full=shelf.serialize();assert.throws(()=>shelf.create(layout(),'Extra'));assert.equal(shelf.serialize(),full);assert.throws(()=>TaskShelf.restore('{"schema":1,"tasks":[]}'));
});
test('forks preserve archived tasks while isolating current edits and rejecting corrupt persisted data',()=>{
 const shelf=new TaskShelf(layout()),fork=shelf.fork();fork.rename(fork.active,'Different');assert.equal(shelf.current().title,'Untitled task');assert.equal(fork.current().title,'Different');const saved=JSON.parse(shelf.serialize());for(const change of [d=>d.active='missing',d=>d.tasks[0].layout='{}',d=>d.tasks[0].title='',d=>d.tasks[0].updated=-1,d=>d.tasks.push(d.tasks[0]),d=>d.next=1]){const bad=structuredClone(saved);change(bad);assert.throws(()=>TaskShelf.restore(JSON.stringify(bad)));}
});
test('task renaming, updates and persistence validate their public byte and identity bounds',()=>{
 const shelf=new TaskShelf(layout());shelf.rename(shelf.active,'é'.repeat(128));assert.equal(shelf.current().title,'é'.repeat(128));assert.equal(TaskShelf.restore(shelf.serialize()).current().title,shelf.current().title);assert.throws(()=>shelf.rename(shelf.active,'é'.repeat(129)));assert.throws(()=>shelf.open('missing'));assert.throws(()=>shelf.remove('missing'));assert.throws(()=>shelf.rename('missing','Name'));assert.throws(()=>shelf.create(layout(),''));assert.throws(()=>shelf.update(null));shelf.update(layout());assert.equal(shelf.current().layout,layout());
 const saved=JSON.parse(shelf.serialize());for(const change of [d=>d.schema=2,d=>d.next=1e6+1,d=>d.next=2.5,d=>d.tasks[0].id='t0',d=>d.tasks[0].id='t2',d=>d.tasks[0].updated=1.2,d=>d.tasks[0].title=null,d=>d.tasks[0]=null,d=>d.tasks=null]){const bad=structuredClone(saved);change(bad);assert.throws(()=>TaskShelf.restore(JSON.stringify(bad)));}
 assert.throws(()=>TaskShelf.restore(' '.repeat(3*1024*1024+1)));const before=shelf.serialize();assert.throws(()=>shelf.commit(Array(20).fill({layout:'x'.repeat(200000)})));assert.equal(shelf.serialize(),before);
});
