import assert from 'node:assert/strict';
import {Workspace} from './visual-model.js';
const id=n=>n.toString(16).padStart(64,'0');
const symbol=(n,line=1)=>({id:id(n),tracking:id(100),name:'run',kind:'function',parents:[],source:{file:id(10),version:id(n),context:id(20),path:'lib.rs',line,end_line:line+150}});
const model=new Workspace(id(1)),a=symbol(2),root=model.openRoot(a,''),other=model.openRoot({...symbol(3),tracking:id(101),name:'other'},'');
model.move(root,500,600);model.resize(root,700,800);model.read(root,{section:97,top:15,left:4});
model.follow(root,{owner:a.id,spelling:'other',source:{...a.source,line:2,end_line:2},candidates:[model.view(other).symbol],candidate_count:1},model.view(other).symbol);
const before=structuredClone(model.view(root)),untouched=structuredClone(model.view(other));
const revision={token:id(30),tip:'a'.repeat(40),commit:'b'.repeat(40),index:0,snapshot:id(4),symbol:symbol(4,10)};
assert.throws(()=>model.showHistory(root,{...revision,index:-1}));
model.showHistory(root,revision);
assert.equal(model.links[0].state,'unverified');assert.equal(model.usable(root),false);assert.equal(model.view(root).mode,'pinned');
assert.equal(model.view(root).reading.section,106);
for(const key of ['id','x','y','w','h','collapsed'])assert.deepEqual(model.view(root)[key],before[key]);
assert.deepEqual(model.view(other),untouched);
const restored=Workspace.restore(model.serialize(),id(1));
assert.deepEqual(restored.view(root).history,model.view(root).history);
restored.showHistory(root,{...revision,index:1,commit:'c'.repeat(40),symbol:symbol(5,20)});
assert.deepEqual(restored.view(root).history.base.symbol,a);
const displayed=structuredClone(restored.view(root).symbol);restored.returnLive(root);assert.equal(restored.view(root).mode,'live');assert.equal(restored.view(root).state,'unresolved');assert.deepEqual(restored.view(root).symbol,displayed);assert.equal(restored.usable(root),false);assert.equal(restored.view(root).history.returning,true);
const unresolved=Workspace.restore(restored.serialize(),id(1));
const updates=unresolved.views.map(v=>v.id===root?{id:v.id,state:'unresolved'}:{id:v.id,state:'current',symbol:v.symbol,position:{section:v.symbol.source.line,line:null,byte:0,reset:false}});
unresolved.refresh(id(1),updates,unresolved.links.map(l=>({id:l.id,check:null})));assert.deepEqual(unresolved.view(root).symbol,displayed);assert.equal(unresolved.view(root).history.returning,true);
updates[0]={id:root,state:'current',symbol:a,position:{section:1,line:null,byte:0,reset:false}};unresolved.refresh(id(1),updates,unresolved.links.map(l=>({id:l.id,check:null})));assert.deepEqual(unresolved.view(root).symbol,a);assert.equal(unresolved.view(root).history,undefined);assert.equal(unresolved.usable(root),true);
for(const mutate of [h=>h.index=-1,h=>h.token='bad',h=>h.base.symbol.source.file='bad',h=>h.commit='HEAD',h=>h.base.history={}]){const bad=JSON.parse(model.serialize());mutate(bad.views[0].history);assert.throws(()=>Workspace.restore(JSON.stringify(bad),id(1)));}

const selected=new Workspace(id(1)),selectedId=selected.openRoot(a,'');
selected.read(selectedId,{section:97,top:15,left:4,anchor:{line:100,byte:6,offset:12}});
selected.showHistory(selectedId,{...revision,tip:revision.commit,same_text:true});
assert.deepEqual(selected.view(selectedId).reading,{section:106,top:15,left:4,anchor:{line:109,byte:6,offset:12}});
assert.deepEqual(Workspace.restore(selected.serialize(),id(1)).view(selectedId).reading,selected.view(selectedId).reading);
const prior={...revision,commit:'c'.repeat(40),index:1,snapshot:id(45),symbol:{...symbol(45),tracking:id(46),name:'oldRun',source:{...symbol(45).source,file:id(47),path:'old.rs'}}};
const pin=frame=>({commit:frame.commit,snapshot:frame.snapshot,file:frame.symbol.source.file,tracking:frame.symbol.tracking});
prior.lineage=[{at:1,newer:pin(revision),older:pin(prior)}];
model.showHistory(root,prior);
assert.equal(model.view(root).symbol.source.path,'old.rs');
assert.equal(model.links[0].source.path,'lib.rs');
assert.doesNotThrow(()=>Workspace.restore(model.serialize(),id(1)));
const badLink=JSON.parse(model.serialize());badLink.links[0].earlier_caller.path='forged.rs';
assert.throws(()=>Workspace.restore(JSON.stringify(badLink),id(1)));
for(const mutate of [edge=>edge.at=0,edge=>edge.older.snapshot=id(99),edge=>edge.newer.tracking=id(99),edge=>edge.older.file=id(99)]){
 const bad=JSON.parse(model.serialize());mutate(bad.views[0].history.lineage[0]);
 assert.throws(()=>Workspace.restore(JSON.stringify(bad),id(1)));
}
model.showHistory(root,{...revision,lineage:prior.lineage});
assert.equal(model.view(root).symbol.source.path,'lib.rs');
console.log('function history: validated revision bindings, relative reading position, stable layout, persistence and return to live');
