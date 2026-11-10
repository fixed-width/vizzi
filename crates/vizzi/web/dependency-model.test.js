import {test} from 'node:test';import assert from 'node:assert/strict';import {Workspace} from './visual-model.js';
const token=n=>n.toString(16).padStart(64,'0');
const source={path:'src/lib.rs',file:token(1),version:token(2),context:token(3),line:1,end_line:3};
const dep={id:token(4),tracking:token(5),name:'start',kind:'function',parents:[],source,origin:{id:token(6),capture:token(7),label:'helper',language:'rust',version:null}};
test('reopening a recovered dependency preserves geometry and makes its capture usable again',()=>{
 const model=new Workspace(token(8)),id=model.openRoot(dep,'src');model.move(id,100,200);model.view(id).state='unresolved';assert.equal(model.usable(id),false);assert.equal(model.openRoot(dep,'src'),id);assert.equal(model.usable(id),true);assert.equal(model.view(id).x,100);assert.equal(model.view(id).y,200);
});
test('a dependency pinned to an earlier project remains separate from a current call trail',()=>{
 const model=new Workspace(token(8)),old=model.openRoot(dep,'src');model.pin(old,true);model.snapshot=token(9);const current=model.openRoot(dep,'src');assert.notEqual(current,old);assert.equal(model.view(old).snapshot,token(8));assert.equal(model.view(old).mode,'pinned');assert.equal(model.usable(current),true);assert.equal(model.views.length,2);
});
test('captured-call following recovers a dependency and keeps pinned project links well formed',()=>{
 const root={...dep,id:token(12),origin:undefined,source:{...source,file:token(13)}};const model=new Workspace(token(8)),old=model.openRoot(dep,'src');model.pin(old,true);model.snapshot=token(9);const caller=model.openRoot(root,'src');
 const call={owner:root.id,name:'start',spelling:'helper::start',source:{...root.source,line:2,end_line:2},candidates:[dep],candidate_count:1};const current=model.follow(caller,call,dep);assert.notEqual(current,old);assert.equal(Workspace.restore(model.serialize(),model.snapshot).links.length,1);model.view(current).state='unresolved';assert.equal(model.follow(caller,call,dep),current);assert.equal(model.usable(current),true);
 assert.equal(model.links.length,1);assert.equal(Workspace.restore(model.serialize(),model.snapshot).links.length,1);
 for(const origin of [{...dep.origin,capture:'invalid'},{...dep.origin,label:[]},{...dep.origin,id:[] }])assert.throws(()=>model.openRoot({...dep,origin},'src'));
});
