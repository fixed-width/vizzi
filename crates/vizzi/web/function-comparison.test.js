import assert from 'node:assert/strict';
import {TaskWorkspace as Workspace} from './task-workspace.js';
const id=n=>n.toString(16).padStart(64,'0');
const symbol=(n,line=1)=>({id:id(n),tracking:id(50),name:'run',kind:'function',parents:[],source:{file:id(10),version:id(n),context:id(11),path:'lib.rs',line,end_line:line+399}});
const frame=(n,line,label,changes)=>({snapshot:id(n),symbol:symbol(n,line),label,other_label:label==='old'?'Saved working tree (captured)':'old',changes,history:null,window:{line_start:changes[0][0]-3}});
const pair={token:id(30),newer:frame(3,1,'Saved working tree (captured)',[[182,184],[300,300]]),older:frame(4,10,'old',[[191,193],[310,312]])};
function fixture(){const m=new Workspace(id(1)),root=m.openRoot(symbol(2),'src');const other=m.openRoot({...symbol(5),tracking:id(51)},'src');m.saveGroup(null,'Reading',[root,other]);return {m,root,other};}
{
 const {m,root,other}=fixture(),before=structuredClone(m.view(root)),untouched=structuredClone(m.view(other));
 const opened=m.openComparison(root,pair);assert.equal(opened.newer,root);assert.equal(m.views.length,3);assert.equal(m.usable(root),false);assert.equal(m.view(root).reading.section,179);
 for(const key of ['id','x','y','w','h','collapsed'])assert.deepEqual(m.view(root)[key],before[key]);assert.deepEqual(m.view(other),untouched);assert.equal(m.groupFor(opened.older).id,m.groupFor(root).id);
 const restored=Workspace.restore(m.serialize(),id(1));assert.deepEqual(restored.view(root).comparison,m.view(root).comparison);
 const moved=restored.comparisonEdit(root,1);assert.deepEqual(new Set(moved),new Set([root,opened.older]));assert.equal(restored.view(root).reading.section,297);assert.equal(restored.view(opened.older).reading.section,307);assert.throws(()=>restored.comparisonEdit(root,1));
 restored.close(opened.older);assert.equal(restored.view(root).comparison.token,pair.token);restored.returnLive(root);assert.equal(restored.view(root).mode,'live');assert.equal(restored.view(root).comparison.returning,true);assert.equal(restored.usable(root),false);assert.doesNotThrow(()=>Workspace.restore(restored.serialize(),id(1)));
 const updates=restored.views.map(v=>({id:v.id,state:'current',symbol:v.id===root?symbol(2):v.symbol,position:{section:v.id===root?1:v.symbol.source.line,line:null,byte:0,reset:false}}));
 restored.refresh(id(1),updates.map(v=>v.id===root?{id:root,state:'unresolved'}:v),[]);assert.equal(restored.view(root).comparison.returning,true);assert.deepEqual(restored.view(root).symbol,pair.newer.symbol);assert.doesNotThrow(()=>Workspace.restore(restored.serialize(),id(1)));
 restored.refresh(id(1),updates,[]);assert.equal(restored.view(root).comparison,undefined);assert.equal(restored.usable(root),true);
}
for(const mutate of [p=>p.token='bad',p=>p.older.symbol.tracking=id(60),p=>p.newer.changes=[[1,5000]],p=>p.newer.changes=[[180,181],[170,171]],p=>p.older.label='x'.repeat(257)]){
 const {m,root}=fixture(),bad=structuredClone(pair);mutate(bad);const before=m.serialize();assert.throws(()=>m.openComparison(root,bad));assert.equal(m.serialize(),before);
}
for(const mutate of [v=>v.comparison.token='bad',v=>v.comparison.side='live',v=>v.comparison.cursor=2,v=>v.comparison.changes=[[0,2]],v=>v.comparison.returning='yes',v=>v.mode='live']){
 const {m,root}=fixture();m.openComparison(root,pair);const bad=JSON.parse(m.serialize());mutate(bad.views[0]);assert.throws(()=>Workspace.restore(JSON.stringify(bad),id(1)));
}
{
 const {m,root}=fixture(),same=structuredClone(pair);same.newer.snapshot=id(1);same.newer.symbol=symbol(2);const opened=m.openComparison(root,same);assert.equal(m.usable(root),false);m.close(root);assert.doesNotThrow(()=>Workspace.restore(m.serialize(),id(1)));m.returnLive(opened.older);assert.equal(m.view(opened.older).comparison.returning,true);
}
{
 const {m,root}=fixture();for(let n=6;n<28;n++)m.openRoot({...symbol(n),tracking:id(n+100)},'');const before=m.serialize();assert.throws(()=>m.openComparison(root,pair));assert.equal(m.serialize(),before);
}
console.log('function comparison: atomic paired views, complete edit navigation, retained geometry/groups, bounded validation, independent closure, persistence and live recovery');
