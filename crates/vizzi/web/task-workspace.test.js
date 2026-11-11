import {test} from 'node:test';
import assert from 'node:assert/strict';
import {TaskWorkspace} from './task-workspace.js';
const token=n=>n.toString(16).padStart(64,'0');
const symbol=n=>({id:token(n),name:'fn'+n,kind:'function',parents:[],source:{file:token(90),version:token(91),context:token(92),path:'lib.rs',line:n,end_line:n+1}});
test('eight distinct task groups are allowed and later additions leave them intact',()=>{
 const model=new TaskWorkspace(token(1));
 for(let n=0;n<8;n++){const a=model.openRoot(symbol(n+2),'lib.rs');const g=model.saveGroup(null,'Group '+n,[a]);assert.equal(model.group(g).title,'Group '+n);}
 const ninth=model.openRoot(symbol(20),'lib.rs'),before=model.serialize();
 assert.throws(()=>model.saveGroup(null,'Ninth',[ninth]));assert.equal(model.serialize(),before);assert.throws(()=>model.group('missing'));
 const group=model.groups[7];model.saveGroup(group.id,'é'.repeat(128),group.members);assert.equal(model.group(group.id).title,'é'.repeat(128));
 assert.equal(TaskWorkspace.restore(model.serialize(),token(1)).groups.length,8);
});
test('collapsed result stacks remain valid saved tasks',()=>{
 const model=new TaskWorkspace(token(1)),a=model.openRoot(symbol(2),'lib.rs'),r=model.addReferences(symbol(2),'callers',a);
 model.collapse(r);
 const restored=TaskWorkspace.restore(model.serialize(),token(1));
 assert.equal(restored.view(r).collapsed,true);assert.equal(restored.selected,a);
});
test('group boundaries enclose every member and collapsed groups replace only their members',()=>{
 const model=new TaskWorkspace(token(1)),a=model.openRoot(symbol(2),'lib.rs'),b=model.openRoot(symbol(3),'lib.rs'),c=model.openRoot(symbol(4),'lib.rs');
 model.move(a,-250,-300);model.move(b,700,800);
 const g=model.saveGroup(null,'  Read  ',[a,b]);assert.equal(model.group(g).title,'Read');
 assert.equal(model.visibleItems().length,3);
 const bounds=model.groupBounds(g);
 for(const id of [a,b]){const item=model.view(id);assert.ok(Number.isFinite(bounds.x));assert.ok(bounds.x<item.x);assert.ok(bounds.y<item.y);assert.ok(bounds.x+bounds.w>item.x+item.w);assert.ok(bounds.y+bounds.h>item.y+item.h);}
 model.collapseGroup(g,true);assert.deepEqual(model.visibleItems().filter(i=>i.kind==='source').map(i=>i.id),[c]);
 const proxy=model.visibleItems().find(i=>i.id===g);assert.ok(proxy.w<=480);assert.equal(proxy.h,54);
 model.saveGroup(g,'Renamed',[a]);assert.equal(model.group(g).collapsed,true);assert.equal(model.isVisible(b),true);assert.equal(model.isVisible(a),false);
 model.ungroup(g);assert.equal(model.visibleItems().length,3);assert.throws(()=>model.ungroup(g));
 assert.throws(()=>model.saveGroup(null,' ',[a]));assert.throws(()=>model.saveGroup(null,'Empty',[]));assert.throws(()=>model.saveGroup(null,'Dup',[a,a]));assert.throws(()=>model.saveGroup('missing','Bad',[a]));
});
test('caller expansion retains the call witness, group and existing layout without creating duplicate views',()=>{
 const model=new TaskWorkspace(token(1)),target=symbol(2),root=model.openRoot(target,'lib.rs'),stack=model.addReferences(target,'callers',root),group=model.saveGroup(null,'Trace',[root,stack]),callerSymbol=symbol(3);
 const row={symbol:callerSymbol,site:{owner:callerSymbol.id,spelling:target.name,source:{...callerSymbol.source},candidate_count:1,candidates:[target]}};
 const opened=model.openReference(stack,row,target);
 assert.equal(opened.destination,root);assert.equal(model.selected,opened.caller);assert.equal(model.views.length,2);assert.equal(model.links.length,1);assert.equal(model.links[0].from,opened.caller);assert.equal(model.links[0].to,root);assert.ok(model.group(group).members.includes(opened.caller));
 assert.ok(model.view(opened.caller).x+model.view(opened.caller).w<model.view(root).x);
 model.move(opened.caller,-900,700);model.openReference(stack,row,target);assert.equal(model.view(opened.caller).x,-900);assert.equal(model.views.length,2);assert.equal(model.links.length,1);
 const value=model.openReference(stack,{symbol:symbol(4)},target);assert.equal(value.destination,null);assert.equal(model.links.length,1);
 assert.throws(()=>model.openReference(root,row,target));assert.throws(()=>model.openReference(stack,{},target));
 model.stale=true;assert.throws(()=>model.openReference(stack,row,target));
});
test('new items avoid occupied source and result rectangles and inherit an existing group',()=>{
 const model=new TaskWorkspace(token(1)),a=model.openRoot(symbol(2),'lib.rs'),g=model.saveGroup(null,'Trace',[a]),r=model.addReferences(symbol(2),'callers',a);
 assert.ok(model.group(g).members.includes(r));assert.ok(model.view(r).x>=model.view(a).x+model.view(a).w);
 const s=model.addReferences(symbol(3),'references',a),one=model.view(r),two=model.view(s);
 assert.ok(two.y>=one.y+one.h||two.x>=one.x+one.w||one.x>=two.x+two.w);
 model.fitSize(a,'width',600);assert.equal(model.selected,a);assert.equal(model.references.length,2);assert.equal(model.views.length,1);
 model.close(r);assert.equal(model.references.length,1);assert.ok(!model.group(g).members.includes(r));
 const empty=new TaskWorkspace(token(1));empty.fit(800,600);assert.ok(Number.isFinite(empty.camera.scale));
 for(const [w,h] of [[0,600],[800,0],[Infinity,600],[800,Infinity]])assert.throws(()=>model.fit(w,h));
});
test('groups move and collapse a working set and survive source-only layout migration',()=>{
 const model=new TaskWorkspace(token(1)),a=model.openRoot(symbol(2),'lib.rs'),b=model.openRoot(symbol(3),'lib.rs');const g=model.saveGroup(null,'Read request',[a,b]);const before=structuredClone(model.views);model.moveGroup(g,40,50);assert.equal(model.view(a).x,before[0].x+40);assert.equal(model.view(b).y,before[1].y+50);model.collapseGroup(g,true);assert.equal(model.isVisible(a),false);model.collapseGroup(g,false);assert.equal(model.isVisible(a),true);const restored=TaskWorkspace.restore(model.serialize(),token(1));assert.deepEqual(restored.groups,model.groups);assert.deepEqual(restored.views,model.views);
 const old=JSON.parse(model.serialize());delete old.collections;assert.equal(TaskWorkspace.restore(JSON.stringify(old),token(1)).groups.length,0);
});
test('group edits are bounded, atomic, and source closure removes obsolete membership',()=>{
 const model=new TaskWorkspace(token(1)),a=model.openRoot(symbol(2),'lib.rs'),b=model.openRoot(symbol(3),'lib.rs'),g=model.saveGroup(null,'Read',[a,b]),before=model.serialize();assert.throws(()=>model.moveGroup(g,1e6,0));assert.equal(model.serialize(),before);assert.throws(()=>model.saveGroup(g,'Bad',['missing']));assert.equal(model.serialize(),before);model.close(a);assert.deepEqual(model.groups[0].members,[b]);model.close(b);assert.equal(model.groups.length,0);
});
test('reference query descriptors share canvas geometry and retain queries across reload',()=>{
 const model=new TaskWorkspace(token(1)),a=model.openRoot(symbol(2),'lib.rs'),r=model.addReferences(model.view(a).symbol,'callers',a);model.move(r,900,500);const g=model.saveGroup(null,'Trace',[a,r]);model.collapseGroup(g,true);assert.equal(model.isVisible(r),false);model.reveal([r],1400,900);assert.equal(model.isVisible(r),true);const restored=TaskWorkspace.restore(model.serialize(),token(1));assert.equal(restored.references[0].query,'callers');assert.equal(restored.view(r).x,900);model.closeGroup(g);assert.equal(model.views.length,0);assert.equal(model.references.length,0);
});
test('collection boundaries, membership moves and malformed stored collections are enforced',()=>{
 const model=new TaskWorkspace(token(1)),a=model.openRoot(symbol(2),'lib.rs'),b=model.openRoot(symbol(3),'lib.rs');const g=model.saveGroup(null,'First',[a]);model.saveGroup(null,'Second',[a,b]);assert.equal(model.groups.length,1);assert.notEqual(model.groups[0].id,g);const original=JSON.parse(model.serialize());
 for(const change of [d=>d.collections.groups[0].members.push(a),d=>d.collections.groups[0].members=['missing'],d=>d.collections.groups[0].title='',d=>d.collections.groups[0].collapsed='true',d=>d.collections.nextGroup=0,d=>d.collections.references=[null],d=>d.collections.groups=[null]]){const bad=structuredClone(original);change(bad);assert.throws(()=>TaskWorkspace.restore(JSON.stringify(bad),token(1)));}
 for(const title of ['', '界'.repeat(86)])assert.throws(()=>model.saveGroup(null,title,[a]));
 for(let n=0;n<4;n++)model.addReferences(symbol(n+4),n%2?'callers':'references',a);assert.throws(()=>model.addReferences(symbol(20),'callers',a));model.resize('r1',500,300);model.collapse('r1');assert.equal(model.view('r1').collapsed,true);model.collapse('r1');assert.equal(model.view('r1').h,300);
 const saved=JSON.parse(model.serialize());for(const change of [d=>d.collections.references[0].w=0,d=>d.collections.references[0].target.name='',d=>d.collections.references[0].query='other',d=>d.collections.references.push(d.collections.references[0]),d=>d.collections.nextReference=0]){const bad=structuredClone(saved);change(bad);assert.throws(()=>TaskWorkspace.restore(JSON.stringify(bad),token(1)));}
});
test('opening a caller is atomic when the source allowance cannot admit its target',()=>{
 const model=new TaskWorkspace(token(1)),a=model.openRoot(symbol(2),'lib.rs'),stack=model.addReferences(symbol(2),'callers',a);for(let n=3;n<=25;n++)model.openRoot(symbol(n),'lib.rs');const before=model.serialize(),caller=symbol(30),target=symbol(31),site={owner:caller.id,spelling:'target',source:{...caller.source},candidate_count:1,candidates:[target]};assert.throws(()=>model.openReference(stack,{symbol:caller,site},target));assert.equal(model.serialize(),before);
});
test('source and result placement, fitting and group inheritance keep the complete canvas readable',()=>{
 const model=new TaskWorkspace(token(1)),a=model.openRoot(symbol(2),'lib.rs'),stack=model.addReferences(symbol(2),'callers',a),g=model.saveGroup(null,'Trace',[a,stack]);const target=symbol(3),site={owner:model.view(a).symbol.id,spelling:'next',source:{...model.view(a).symbol.source,line:2,end_line:2},candidates:[target],candidate_count:1};const child=model.follow(a,site,target);assert.ok(model.group(g).members.includes(child));assert.equal(model.addReferences(symbol(2),'callers',a),stack);model.fitSize(stack,'width',800);assert.equal(model.view(stack).w,800);model.fit(1400,900);for(const item of model.visibleItems()){assert.ok((item.x-model.camera.x)*model.camera.scale>=0);assert.ok((item.x+item.w-model.camera.x)*model.camera.scale<=1400);}model.collapseGroup(g,true);assert.equal(model.visibleItems().length,1);assert.equal(model.visibleItems()[0].kind,'group');model.fit(1400,900);model.ungroup(g);assert.equal(model.groups.length,0);assert.equal(model.views.length,2);assert.equal(model.references.length,1);assert.throws(()=>model.group(g));assert.throws(()=>model.fit(0,900));
});
test('stored reference geometry and collection identifiers reject malformed values at the interface',()=>{
 const model=new TaskWorkspace(token(1)),a=model.openRoot(symbol(2),'lib.rs');model.addReferences(symbol(2),'references',a);model.saveGroup(null,'Name',[a]);const data=JSON.parse(model.serialize());
 for(const [field,values] of Object.entries({id:['bad','r0','r2',null],kind:['source'],query:['unknown'],snapshot:['bad'],target:[null],x:[null,-100001,100001],y:[null,-100001,100001],w:[319,1601,null],h:[199,2401,null],collapsed:[null,'false']}))for(const value of values){const bad=structuredClone(data);bad.collections.references[0][field]=value;assert.throws(()=>TaskWorkspace.restore(JSON.stringify(bad),token(1)),field);}
 for(const change of [d=>d.collections.groups[0].id='g0',d=>d.collections.groups[0].id='g2',d=>d.collections.groups[0].members=[],d=>d.collections.groups[0].members=['r999'],d=>d.collections.groups[0].title='界'.repeat(86),d=>d.collections.groups.push(d.collections.groups[0]),d=>d.collections.nextGroup=1,d=>d.collections.nextGroup=1e6+1,d=>d.collections.nextReference=1.5,d=>d.collections.nextReference=1e6+1,d=>d.collections.references=null]){const bad=structuredClone(data);change(bad);assert.throws(()=>TaskWorkspace.restore(JSON.stringify(bad),token(1)));}
});
