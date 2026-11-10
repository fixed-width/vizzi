import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {Workspace,worldPoint} from './visual-model.js';
const token=n=>n.toString(16).repeat(64),snapshot=token(1);
const symbol=n=>({id:token(n),name:'function_'+n,kind:'function',parents:['Pipeline'],source:{path:'src/flow.rs',file:token(9),version:token(8),context:token(7),line:n*10,end_line:n*10+9}});
const call=(parent,candidate,line=parent.source.line+1)=>({owner:parent.id,name:candidate.name,spelling:'self.'+candidate.name,source:{...parent.source,line,end_line:line},candidates:[candidate],candidate_count:1});
const model=new Workspace(snapshot),a=symbol(2),b=symbol(3),c=symbol(4),d=symbol(5);
const root=model.openRoot(a,'src'),rootView=model.view(root);model.read(root,{top:88,left:14,section:a.source.line});const before=structuredClone(model.view(root));
const child=model.follow(root,call(a,b),b);assert.deepEqual(model.view(root),before);assert.equal(model.views.length,2);assert.equal(model.links.length,1);assert.ok(model.view(child).x>=rootView.x+rootView.w);
const stable=structuredClone(model.views);const branch=model.follow(root,call(a,c,a.source.line+2),c);assert.deepEqual(model.view(root),stable[0]);assert.deepEqual(model.view(child),stable[1]);assert.ok(model.view(branch).y>=model.view(child).y+model.view(child).h);
assert.equal(model.follow(root,call(a,b),b),child);assert.equal(model.links.length,2);assert.equal(model.views.length,3);
const grandchild=model.follow(child,call(b,d),d);const retained=structuredClone(model.view(root));model.reveal([child,grandchild],1200,800);assert.deepEqual(model.view(root),retained);assert.ok(model.camera.scale>=0.35);
const cycle=model.follow(grandchild,call(d,a),a);assert.equal(cycle,root);assert.equal(model.views.length,4);assert.equal(model.links.length,4);
model.move(child,1300,500);model.resize(child,510,470);model.collapse(child);assert.equal(model.view(child).collapsed,true);model.collapse(child);assert.equal(model.view(child).h,470);assert.deepEqual(model.view(root),retained);
const saved=model.serialize(),restored=Workspace.restore(saved,snapshot);assert.deepEqual(restored.views,model.views);assert.deepEqual(restored.links,model.links);assert.deepEqual(restored.camera,model.camera);assert.equal(restored.stale,false);
const old=Workspace.restore(saved,token(6));assert.equal(old.stale,true);assert.throws(()=>old.follow(root,call(a,b),b),/changed/i);old.move(root,90,90);assert.equal(old.view(root).x,90);
const scene=structuredClone(model.views);model.close(branch);assert.equal(model.views.length,3);assert.equal(model.links.some(l=>l.from===branch||l.to===branch),false);assert.deepEqual(model.view(root),scene.find(v=>v.id===root));
for(const invalid of [NaN,Infinity,1e10]){assert.throws(()=>model.move(root,invalid,1));assert.throws(()=>model.resize(root,invalid,300));}
assert.throws(()=>model.follow(root,{...call(a,b),owner:b.id},b));assert.throws(()=>model.follow(root,{...call(a,b),source:{...call(a,b).source,version:token(6)}},b));assert.throws(()=>model.follow(root,{...call(a,b),candidates:[]},b));
for(const bad of ['{}','null','not json',JSON.stringify({...JSON.parse(saved),schema:99}),JSON.stringify({...JSON.parse(saved),views:Array(25).fill(model.view(root))})])assert.throws(()=>Workspace.restore(bad,snapshot));
const malicious=JSON.parse(saved);malicious.views[0].x=null;assert.throws(()=>Workspace.restore(JSON.stringify(malicious),snapshot));
const camera=new Workspace(snapshot);camera.openRoot(a,'src');const point=worldPoint({x:230,y:170},camera.camera);camera.zoomAt(1.4,{x:230,y:170});assert.ok(Math.abs(worldPoint({x:230,y:170},camera.camera).x-point.x)<1e-8);assert.ok(Math.abs(worldPoint({x:230,y:170},camera.camera).y-point.y)<1e-8);
camera.fit(1200,800);assert.ok(camera.camera.scale<=1);assert.throws(()=>camera.fit(0,800));
console.log('visual model: retained caller/branch geometry and reading state, revisits/cycles, layout controls, camera, bounded persistence and version guards');

// Persisted links must still refer to their captured caller, not merely have valid IDs.
for(const change of [
 l=>{l.source.version=token(6);},l=>{l.source.context=token(6);},
 l=>{l.source.file=token(6);},l=>{l.source.path='wrong.rs';},
 l=>{l.source.line=1;},l=>{l.source.end_line=999;},l=>{l.id='fabricated';}
]){const data=JSON.parse(saved);change(data.links[0]);assert.throws(()=>Workspace.restore(JSON.stringify(data),snapshot));}
const forged={...b,source:{...b.source,path:'different.rs'}};
assert.throws(()=>model.follow(root,call(a,b),forged));
const large=new Workspace(snapshot);
for(let n=1;n<=24;n++){const s=symbol(n);s.id=n.toString(16).padStart(64,'0');large.openRoot(s,'src');large.move(large.selected,n*512,64);}
assert.throws(()=>large.openRoot({...a,id:'f'.repeat(64)},'src'),/Close a fragment/);
large.fit(1200,800);
for(const v of large.views){assert.ok((v.x-large.camera.x)*large.camera.scale>=0);assert.ok((v.x+v.w-large.camera.x)*large.camera.scale<=1200);}

// Public document boundaries: invalid persisted data is rejected before it can mount views.
const document=()=>JSON.parse(saved), restore=data=>Workspace.restore(JSON.stringify(data),snapshot);
for(const bad of ['', 'g'.repeat(64), '0'.repeat(63), '0'.repeat(65), 'x'+token(1),token(1)+'x',12,null])assert.throws(()=>new Workspace(bad));
const badSource={path:[null,12,'','/src/a.rs','src//a.rs','src/./a.rs','src/../a.rs','src\\a.rs','a'.repeat(4097)],file:['bad'],version:['bad'],context:['bad'],line:[0,-1,1.5,100000001,'20'],end_line:[0,19,1.5,100000001,'29']};
for(const[key,values]of Object.entries(badSource))for(const value of values){const data=document();data.views[0].symbol.source[key]=value;assert.throws(()=>restore(data),`invalid source ${key}: ${value}`);}
for(const[key,values]of Object.entries({id:['bad'],name:['',12,'界'.repeat(43)],kind:[12,'x'.repeat(129)],parents:[null,'a',Array(17).fill('p'),['good',12],['good','p'.repeat(129)]],source:[null]}))for(const value of values){const data=document();data.views[0].symbol[key]=value;assert.throws(()=>restore(data),`invalid symbol ${key}`);}
for(const[key,values]of Object.entries({id:['v0','v01','v1x','xv1','v1a','v1000000'],kind:['note'],area:[12,'x'.repeat(4097)],collapsed:[null,'false'],symbol:[null],reading:[null],x:[-100001,100001,null,'10'],y:[-100001,100001,null,'10'],w:[319,961,null],h:[199,961,null]}))for(const value of values){const data=document();data.views[0][key]=value;assert.throws(()=>restore(data),`invalid view ${key}`);}
for(const[key,values]of Object.entries({top:[-1,10000001,null,'0'],left:[-1,10000001,null,'0'],section:[a.source.line-1,a.source.end_line+1,20.5,'20']}))for(const value of values){const data=document();data.views[0].reading[key]=value;assert.throws(()=>restore(data),`invalid reading ${key}`);assert.throws(()=>model.read(root,{...model.view(root).reading,[key]:value}));}
for(const[key,values]of Object.entries({x:[-100001,100001,null],y:[-100001,100001,null],scale:[0,.001,1.51,null]}))for(const value of values){const data=document();data.camera[key]=value;assert.throws(()=>restore(data),`invalid camera ${key}`);}
for(const change of [d=>d.views.push({...d.views[0]}),d=>d.views[1].symbol=d.views[0].symbol,d=>d.selected='v999',d=>d.next=4,d=>d.next=1e6+1,d=>d.next=4.5,d=>d.views[0]=null,d=>d.links[0]=null,d=>d.links.push(d.links[0])]){const data=document();change(data);assert.throws(()=>restore(data));}
for(const[key,values]of Object.entries({id:[12],from:['v999'],to:['v999'],kind:['resolved'],spelling:[null,'','a'.repeat(257)],source:[null]}))for(const value of values){const data=document();data.links[0][key]=value;assert.throws(()=>restore(data),`invalid link ${key}`);}
assert.throws(()=>Workspace.restore(saved+' '.repeat(524288),snapshot));assert.throws(()=>Workspace.restore(null,snapshot));assert.throws(()=>Workspace.restore('no json',snapshot),/Invalid visual workspace JSON/);
assert.doesNotThrow(()=>Workspace.restore(saved+' '.repeat(524288-new TextEncoder().encode(saved).length),snapshot));

// Inclusive limits and UTF-8 bounds remain usable, not merely rejected around the edges.
const edge=new Workspace(snapshot),edgeSymbol=symbol(2);edgeSymbol.name='界'.repeat(42)+'ab';edgeSymbol.kind='k'.repeat(128);edgeSymbol.parents=Array(16).fill('p'.repeat(128));Object.assign(edgeSymbol.source,{path:'a'.repeat(4096),line:1e8,end_line:1e8});
const edgeId=edge.openRoot(edgeSymbol,'x'.repeat(4096));edge.move(edgeId,-1e5,1e5);edge.resize(edgeId,320,960);edge.read(edgeId,{section:1e8,top:1e7,left:1e7});edge.camera={x:-1e5,y:1e5,scale:1.5};assert.deepEqual(Workspace.restore(edge.serialize(),snapshot).views,edge.views);
edge.resize(edgeId,960,200);edge.move(edgeId,1e5,-1e5);edge.camera.scale=.02;assert.deepEqual(Workspace.restore(edge.serialize(),snapshot).camera,edge.camera);
assert.equal(Workspace.restore(large.serialize(),snapshot).views.length,24);
const empty=new Workspace(snapshot);assert.equal(Workspace.restore(empty.serialize(),snapshot).selected,null);empty.openRoot(a,'');empty.openRoot(b,'');empty.close(empty.selected);assert.equal(empty.selected,empty.views[0].id);empty.close(empty.selected);assert.equal(empty.selected,null);empty.pan(12,35);empty.fit(500,700);assert.deepEqual(empty.camera,{x:0,y:0,scale:1});
const sourceMutation=new Workspace(snapshot),original=structuredClone(a),copyId=sourceMutation.openRoot(original,'src');original.name='changed';assert.equal(sourceMutation.view(copyId).symbol.name,a.name);const readingCopy={section:20,top:50,left:4};sourceMutation.read(copyId,readingCopy);readingCopy.top=90;assert.equal(sourceMutation.view(copyId).reading.top,50);
for(const target of [{...b,name:'different'},{...b,kind:'type'},{...b,parents:['Other']}])assert.throws(()=>model.follow(root,call(a,b),target));
for(const change of [s=>s.source.path='other.rs',s=>s.source.context=token(6),s=>s.source.file=token(6),s=>s.source.line=1,s=>s.source.end_line=999,s=>s.spelling='',s=>s.spelling='x'.repeat(257),s=>s.spelling=12,s=>s.candidates=null]){const site=call(a,b);change(site);assert.throws(()=>model.follow(root,site,b));}
assert.throws(()=>model.openRoot(a,12));assert.throws(()=>model.openRoot(a,'x'.repeat(4097)));assert.throws(()=>model.view('missing'));assert.throws(()=>model.close('missing'));

// Screen-space controls, bounded zoom and framing are independent of view coordinates.
const controls=new Workspace(snapshot);controls.zoomAt(.5,{x:0,y:0});controls.pan(20,30);assert.deepEqual(controls.camera,{x:-40,y:-60,scale:.5});
for(const [dx,dy]of[[NaN,0],[0,NaN],[100001,0],[0,-100001]])assert.throws(()=>controls.pan(dx,dy));
controls.pan(-1e5,-1e5);controls.pan(-1e5,-1e5);assert.deepEqual(controls.camera,{x:1e5,y:1e5,scale:.5});controls.pan(1e5,1e5);controls.pan(1e5,1e5);assert.deepEqual(controls.camera,{x:-1e5,y:-1e5,scale:.5});
controls.camera={x:0,y:0,scale:1};controls.zoomAt(99,{x:20,y:30});assert.equal(controls.camera.scale,1.5);controls.zoomAt(-1,{x:20,y:30});assert.equal(controls.camera.scale,.02);
for(const [scale,point]of[[NaN,{x:0,y:0}],[1,{x:NaN,y:0}],[1,{x:0,y:NaN}]])assert.throws(()=>controls.zoomAt(scale,point));
const tall=new Workspace(snapshot);const t1=tall.openRoot(a,'src'),t2=tall.openRoot(b,'src');tall.move(t1,1000,2000);tall.move(t2,1000,4000);tall.fit(1200,800);for(const v of tall.views){assert.ok((v.y-tall.camera.y)*tall.camera.scale>=0);assert.ok((v.y+v.h-tall.camera.y)*tall.camera.scale<=800);}
tall.reveal([t1,t2],500,300);assert.equal(tall.camera.scale,.65);tall.collapse(t2);tall.fit(1200,800);assert.ok(tall.camera.scale>700/2400);assert.throws(()=>tall.reveal([t1],1200,0));

// Links are bounded independently of views, with existing trails still revisitable at capacity.
const linked=new Workspace(snapshot),linkedRoot=linked.openRoot(a,'src');for(let i=0;i<96;i++){const site=call(a,b);site.spelling='choice_'+i;linked.follow(linkedRoot,site,b);}assert.equal(linked.links.length,96);assert.equal(Workspace.restore(linked.serialize(),snapshot).links.length,96);
const repeat=call(a,b);repeat.spelling='choice_0';assert.equal(linked.follow(linkedRoot,repeat,b),linked.views[1].id);const extra=call(a,c);extra.spelling='overflow';assert.throws(()=>linked.follow(linkedRoot,extra,c));assert.equal(linked.views.length,2);
const overflow=JSON.parse(linked.serialize());overflow.links.push({...overflow.links[0],id:'extra'});assert.throws(()=>restore(overflow));
const longCall=call(a,b);longCall.spelling='x'.repeat(256);assert.doesNotThrow(()=>model.follow(root,longCall,b));assert.doesNotThrow(()=>Workspace.restore(model.serialize(),snapshot));
console.log('visual model: invalid/boundary documents, captured witnesses, full view/link allowances, camera bounds, framing and independent copies passed');

// Roots have no relationship validation to mask invalid source bindings.
for(const[key,values]of Object.entries(badSource))for(const value of values){const s=structuredClone(a);s.source[key]=value;assert.throws(()=>new Workspace(snapshot).openRoot(s,'src'));}
for(const invalid of [[snapshot],{toString:()=>snapshot}])assert.throws(()=>new Workspace(invalid));
const alone=JSON.parse(new Workspace(snapshot).serialize());alone.views=[structuredClone(rootView)];alone.next=2;alone.selected=null;
for(const id of ['xv1','v1x','v1a']){const data=structuredClone(alone);data.views[0].id=id;assert.throws(()=>restore(data));}
const duplicates=structuredClone(alone);duplicates.next=3;duplicates.views.push({...structuredClone(duplicates.views[0]),id:'v2'});assert.equal(restore(duplicates).views.length,2);
const viewOverflow=JSON.parse(large.serialize());viewOverflow.views.push({...structuredClone(viewOverflow.views[0]),id:'v25',symbol:{...structuredClone(a),id:'f'.repeat(64)}});viewOverflow.next=26;viewOverflow.views[24].reading.section=a.source.line;assert.throws(()=>restore(viewOverflow));
const linksOverflow=JSON.parse(linked.serialize());const added=structuredClone(linksOverflow.links[0]);added.spelling='extra';added.id=[added.from,added.to,added.source.file,added.source.line,added.spelling].join('|');linksOverflow.links.push(added);assert.throws(()=>restore(linksOverflow));
for(const spelling of ['',null,'x'.repeat(257)]){const data=document(),l=data.links[0];l.spelling=spelling;l.id=[l.from,l.to,l.source.file,l.source.line,spelling].join('|');assert.throws(()=>restore(data));}
for(const change of [s=>s.id=token(6),s=>s.source.file=token(6),s=>s.source.version=token(6),s=>s.source.context=token(6),s=>s.source.line++,s=>s.source.end_line++]){const candidate=structuredClone(b);change(candidate);assert.throws(()=>model.follow(root,call(a,b),candidate));}
const boundaryCalls=new Workspace(snapshot),boundaryRoot=boundaryCalls.openRoot(a,'src');assert.doesNotThrow(()=>boundaryCalls.follow(boundaryRoot,call(a,b,a.source.line),b));const boundary=call(a,b,a.source.end_line);assert.doesNotThrow(()=>boundaryCalls.follow(boundaryRoot,boundary,b));
assert.throws(()=>old.openRoot(a,'src'),/changed/i);
const closed=new Workspace(snapshot),closeA=closed.openRoot(a,'src'),closeB=closed.follow(closeA,call(a,b),b),closeC=closed.follow(closeB,call(b,c),c);closed.follow(closeA,call(a,c),c);closed.follow(closeC,call(c,a),a);closed.close(closeB);assert.equal(closed.links.length,2);assert.ok(closed.links.every(l=>l.from!==closeB&&l.to!==closeB));assert.equal(closed.selected,closeA);
const frame=new Workspace(snapshot),frameRoot=frame.openRoot(a,'src');frame.move(frameRoot,2000,3000);frame.fit(500,1000);const frameView=frame.view(frameRoot);assert.ok((frameView.x-frame.camera.x)*frame.camera.scale>=0);assert.ok((frameView.x+frameView.w-frame.camera.x)*frame.camera.scale<=500);

// Opening near occupied edges, on either side, never overlaps an existing fragment.
for(const [x,y]of[[-370,64],[500,64],[64,-330],[64,460],[64,64]]){const placement=new Workspace(snapshot),existing=placement.openRoot(a,'src');placement.move(existing,x,y);const added=placement.openRoot(b,'src'),u=placement.view(existing),v=placement.view(added);assert.ok(v.x+v.w<=u.x||u.x+u.w<=v.x||v.y+v.h<=u.y||u.y+u.h<=v.y);}
const exhausted=new Workspace(snapshot),far=exhausted.openRoot(a,'src');exhausted.move(far,1e5,1e5);const unchanged=exhausted.serialize();assert.throws(()=>exhausted.follow(far,call(a,b),b));assert.equal(exhausted.serialize(),unchanged);
for(const value of [new String('src/a.rs')]){const s=structuredClone(a);s.source.path=value;assert.throws(()=>new Workspace(snapshot).openRoot(s,'src'));}
assert.throws(()=>Workspace.restore(new String(saved),snapshot));
for(const id of ['v01','v-1']){const data=structuredClone(alone);data.views[0].id=id;assert.throws(()=>restore(data));}
for(const existing of [[-1000,64],[1500,64],[64,-1500],[64,1500],[-400,64],[528,64],[64,-360],[64,488]]){const free=new Workspace(snapshot),first=free.openRoot(a,'src');free.move(first,...existing);const next=free.openRoot(b,'src');assert.equal(free.view(next).y,64,'an unoccupied starting position remains available');}
const sorted=new Workspace(snapshot),lower=sorted.openRoot(a,'src'),upper=sorted.openRoot(b,'src');sorted.move(lower,64,600);sorted.move(upper,64,64);const after=sorted.openRoot(c,'src');assert.ok(sorted.view(after).y>=1000,'placement clears vertically overlapping occupied intervals regardless of insertion order');
assert.ok(frame.camera.scale>.9,'framing a single fragment retains readable scale');
for(const [key,index,label]of [['views',0,'Invalid restored fragment'],['links',0,'Invalid restored link']]){const data=document();data[key][index]=null;assert.throws(()=>restore(data),new RegExp(label));}
const delimiter=new Workspace(snapshot),wide=structuredClone(a);wide.source.end_line=999;const delimiterRoot=delimiter.openRoot(wide,'src'),one=call(wide,b,20),two=call(wide,b,201);one.spelling='1foo';two.spelling='foo';delimiter.follow(delimiterRoot,one,b);delimiter.follow(delimiterRoot,two,b);assert.equal(delimiter.links.length,2);
const exponentId=structuredClone(alone);exponentId.views[0].id='v1e2';exponentId.next=101;assert.throws(()=>restore(exponentId));
assert.throws(()=>new Workspace(snapshot+'\n'));
const boxedId=structuredClone(alone);boxedId.views[0].id=['v1'];assert.throws(()=>restore(boxedId));
assert.throws(()=>new Workspace(new String(snapshot)));

const bound=new Workspace(snapshot),boundRoot=bound.openRoot(a,'src'),boundSite={...call(a,b),binding:'rust_self_receiver'};const boundChild=bound.follow(boundRoot,boundSite,b);assert.equal(bound.links[0].kind,'self');assert.equal(Workspace.restore(bound.serialize(),snapshot).links[0].kind,'self');assert.equal(boundChild,bound.views[1].id);
assert.throws(()=>bound.follow(boundRoot,{...boundSite,candidates:[b,c],candidate_count:2},b));
assert.throws(()=>bound.follow(boundRoot,{...boundSite,binding:'unknown'},b));
assert.throws(()=>bound.follow(boundRoot,{...boundSite,candidates:[b,c],candidate_count:1},b));
assert.throws(()=>bound.follow(boundRoot,{...boundSite,candidate_count:2},b));
assert.equal(model.links[0].kind,'candidate','a name candidate must not be promoted to a source-local binding');
const semanticModel=new Workspace(snapshot),semanticRoot=semanticModel.openRoot(a,'src');
const semanticSite={...call(a,b),binding:'language_server_definition',semantic_context:snapshot};
semanticModel.follow(semanticRoot,semanticSite,b);assert.equal(semanticModel.links[0].kind,'definition');assert.equal(semanticModel.links[0].semantic_context,snapshot);assert.equal(Workspace.restore(semanticModel.serialize(),snapshot).links[0].kind,'definition');
assert.throws(()=>semanticModel.follow(semanticRoot,{...semanticSite,semantic_context:'missing'},b));
for(const total of [0,1.5,65,undefined])assert.throws(()=>semanticModel.follow(semanticRoot,{...semanticSite,candidate_count:total},b));
const otherContext=token(6);semanticModel.follow(semanticRoot,{...semanticSite,semantic_context:otherContext},b);assert.equal(semanticModel.links.length,2,'distinct semantic configurations keep distinct link evidence');assert.deepEqual(Workspace.restore(semanticModel.serialize(),snapshot).links.map(l=>l.semantic_context),[snapshot,otherContext]);
for(const context of ['missing',token(7)]){const data=JSON.parse(semanticModel.serialize());data.links[0].semantic_context=context;assert.throws(()=>Workspace.restore(JSON.stringify(data),snapshot));}

const legacyLayout=readFileSync(new URL('./fixtures/visual-v1.json',import.meta.url),'utf8'),legacyData=JSON.parse(legacyLayout);
const legacyModel=Workspace.restore(legacyLayout,legacyData.snapshot);assert.equal(legacyModel.views.length,5);assert.equal(legacyModel.links.length,4);assert.ok(legacyModel.links.every(l=>l.kind==='candidate'));
for(const badKind of ['bogus','resolved']){const data=JSON.parse(semanticModel.serialize());const link=data.links[0];link.kind=badKind;link.id=[link.from,link.to,link.source.file,link.source.line,link.spelling].join('|');assert.throws(()=>Workspace.restore(JSON.stringify(data),snapshot));}
{const data=JSON.parse(semanticModel.serialize()),link=data.links[0];link.semantic_context='invalid';link.id=[link.from,link.to,link.source.file,link.source.line,link.spelling,link.semantic_context].join('|');assert.throws(()=>Workspace.restore(JSON.stringify(data),snapshot));}

assert.throws(()=>semanticModel.follow(semanticRoot,{...semanticSite,binding:'unknown'},b));
assert.throws(()=>semanticModel.follow(semanticRoot,{...semanticSite,binding:null},b));
{const data=JSON.parse(saved);data.links[0].semantic_context=snapshot;assert.throws(()=>Workspace.restore(JSON.stringify(data),snapshot));}

const anchoredModel=new Workspace(snapshot),anchoredRoot=anchoredModel.openRoot(a,'src');
const sourceReading={section:a.source.line,top:44,left:0,anchor:{line:a.source.line+1,byte:32,offset:4}};
anchoredModel.read(anchoredRoot,sourceReading);assert.deepEqual(Workspace.restore(anchoredModel.serialize(),snapshot).view(anchoredRoot).reading,sourceReading);
for(const anchor of [{line:a.source.line-1,byte:0,offset:0},{line:a.source.end_line+1,byte:0,offset:0},{line:a.source.line,byte:-1,offset:0},{line:a.source.line,byte:32769,offset:0},{line:a.source.line,byte:1.5,offset:0},{line:a.source.line,byte:0,offset:Infinity}])assert.throws(()=>anchoredModel.read(anchoredRoot,{...sourceReading,anchor}));
