import assert from 'node:assert/strict';
import {TaskWorkspace as Workspace} from './task-workspace.js';
import {frozenSource,frozenLabel} from './visual-model.js';
const id=n=>n.toString(16).padStart(64,'0'),commit='a'.repeat(40);
const symbol=(n,line=1)=>({id:id(n),tracking:id(n+50),name:'fn'+n,kind:'function',parents:[],source:{file:id(10),version:id(2),context:id(11),path:'lib.rs',line,end_line:line+20}});
function fixture(){const m=new Workspace(id(1)),original=symbol(2),root=m.openRoot(original,'lib.rs');m.showHistory(root,{token:id(30),tip:commit,commit,index:0,snapshot:id(2),symbol:symbol(2)});m.saveGroup(null,'Trail',[root]);return {m,root};}
const site=(caller,target,line=2)=>({id:id(40+line),owner:caller.id,name:target.name,spelling:target.name,source:{...caller.source,line,end_line:line},candidates:[target],candidate_count:1});
function frame(m,root,target){return {snapshot:id(2),symbol:target,captured:{owner:frozenSource(m.view(root)).owner,label:frozenLabel(m.view(root))},window:{line_start:target.source.line}};}
{
 const {m,root}=fixture(),caller=m.view(root).symbol,target=symbol(3,30),before=structuredClone(m.view(root));
 assert.equal(m.navigable(root),true);assert.equal(m.usable(root),false);const child=m.followCaptured(root,site(caller,target),frame(m,root,target));assert.equal(m.view(child).mode,'pinned');assert.equal(m.navigable(child),true);assert.equal(m.usable(child),false);assert.equal(m.groupFor(child).id,m.groupFor(root).id);assert.deepEqual(m.view(root),before);assert.equal(m.links[0].state,'captured');
 assert.equal(m.followCaptured(root,site(caller,target),frame(m,root,target)),child);assert.equal(m.views.length,2);assert.equal(m.followCaptured(child,site(target,caller,31),frame(m,root,caller)),root);assert.equal(m.views.length,2);
 m.markSnapshot(id(5));m.refresh(id(5),[],m.links.map(l=>({id:l.id,check:null})));assert.ok(m.links.every(l=>l.state==='captured'));
 const restored=Workspace.restore(m.serialize(),id(5));assert.deepEqual(restored.links,m.links);assert.equal(restored.view(child).captured.label,commit);
 restored.returnLive(child);assert.equal(restored.view(child).captured.returning,true);assert.equal(restored.navigable(child),false);restored.refresh(id(5),[{id:child,state:'unresolved'}],restored.links.map(l=>({id:l.id,check:null})));assert.ok(restored.links.every(l=>l.state==='unverified'));assert.doesNotThrow(()=>Workspace.restore(restored.serialize(),id(5)));
 restored.refresh(id(5),[{id:child,state:'current',symbol:symbol(5,30),position:{section:30,line:null,byte:0,reset:false}}],restored.links.map(l=>({id:l.id,check:null})));assert.equal(restored.view(child).captured,undefined);assert.equal(restored.usable(child),true);
}
for(const mutate of [f=>f.snapshot=id(8),f=>f.captured.owner.index=1,f=>f.symbol.source.file=id(80),f=>f.captured.label='',f=>f.captured.owner.token='bad']){
 const {m,root}=fixture(),target=symbol(3,30),bad=frame(m,root,target);mutate(bad);const before=m.serialize();assert.throws(()=>m.followCaptured(root,site(m.view(root).symbol,target),bad));assert.equal(m.serialize(),before);
}
{
 const {m,root}=fixture(),target=symbol(3,30),a=m.followCaptured(root,site(m.view(root).symbol,target),frame(m,root,target));
 const revision={token:id(30),tip:commit,commit:'b'.repeat(40),index:1,snapshot:id(2),symbol:m.view(root).symbol};m.showHistory(root,revision);const b=m.followCaptured(root,site(m.view(root).symbol,target),frame(m,root,target));assert.notEqual(a,b);assert.equal(m.views.length,3);assert.equal(m.links[0].state,'unverified');assert.equal(m.links[1].state,'captured');
 const saved=JSON.parse(m.serialize());saved.views.find(v=>v.id===b).captured.owner.kind='current';assert.throws(()=>Workspace.restore(JSON.stringify(saved),id(1)));
}
{
 const {m,root}=fixture(),target=symbol(3,30),child=m.followCaptured(root,site(m.view(root).symbol,target),frame(m,root,target));
 m.returnLive(child);assert.equal(m.links[0].state,'unverified');
 const restored=Workspace.restore(m.serialize(),id(1));assert.equal(restored.view(child).mode,'live');assert.equal(restored.view(child).captured.returning,true);assert.equal(restored.links[0].state,'unverified');
}
console.log('captured calls: revision-scoped reuse/cycles, source evidence, groups, stable views, persisted links and independent live recovery');
