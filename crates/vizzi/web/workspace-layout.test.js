import {test} from 'node:test';
import assert from 'node:assert/strict';
import {Workspace} from './visual-model.js';
const hash=n=>n.toString(16).padStart(64,'0');
const symbol=n=>({id:hash(n),name:'run'+n,kind:'function',parents:[],source:{file:hash(n+50),version:hash(1),context:hash(2),path:'lib.rs',line:1,end_line:100}});
function scene(){const m=new Workspace(hash(1));for(let i=1;i<=4;i++)m.openRoot(symbol(i),'');for(const [id,x,y] of [['v1',100,100],['v2',100,530],['v3',100,970],['v4',900,530]])m.move(id,x,y);return m;}
test('revealing a destination preserves every explicit zoom and leaves an already visible bubble alone',()=>{
 for(const scale of [.2,.75,1.5]){const m=scene();m.camera={x:50,y:50,scale};m.reveal(['v1'],1400,1000);assert.equal(m.camera.scale,scale);const before={...m.camera};m.reveal(['v1'],1400,1000);assert.deepEqual(m.camera,before);m.move('v2',4000,6000);m.reveal(['v1','v2'],1400,1000);assert.equal(m.camera.scale,scale);const v=m.view('v2');assert.ok((v.x-m.camera.x)*scale>=0);assert.ok((v.x+v.w-m.camera.x)*scale<=1400);assert.ok((v.y-m.camera.y)*scale>=0);assert.ok((v.y+v.h-m.camera.y)*scale<=1000);}
});
test('height growth pushes a local chain down without changing width, other bubbles, reading or camera',()=>{
 const m=scene();m.read('v1',{section:1,top:12,left:4,anchor:{line:2,byte:0,offset:0}});const before=structuredClone(m.views),camera={...m.camera};m.fitSize('v1','height',900);assert.equal(m.view('v1').h,900);assert.equal(m.view('v1').w,440);assert.equal(m.view('v1').x,100);assert.equal(m.view('v1').y,100);assert.equal(m.view('v2').y,1024);assert.equal(m.view('v3').y,1448);assert.deepEqual(m.view('v4'),before[3]);assert.deepEqual(m.view('v1').reading,before[0].reading);assert.deepEqual(m.camera,camera);assert.deepEqual(Workspace.restore(m.serialize(),hash(1)).views,m.views);
});
test('width growth pushes right and resolves earlier fixed obstacles deterministically',()=>{
 const m=scene();m.move('v2',570,100);m.move('v3',1040,100);m.move('v4',150,1000);m.fitSize('v1','width',900);assert.equal(m.view('v1').w,900);assert.equal(m.view('v1').h,400);assert.equal(m.view('v2').x,1024);assert.equal(m.view('v3').x,1488);assert.equal(m.view('v4').x,150);
 const a=scene(),b=scene();a.views.reverse();a.fitSize('v1','height',900);b.fitSize('v1','height',900);for(const v of b.views)assert.deepEqual(a.view(v.id),v);
});
test('fitting is bounded, can shrink, and rejects impossible plans atomically',()=>{
 const m=scene();m.fitSize('v1','width',1e6);assert.equal(m.view('v1').w,1600);m.fitSize('v1','height',1e6);assert.equal(m.view('v1').h,2400);const neighbours=structuredClone(m.views.slice(1));m.fitSize('v1','width',1);m.fitSize('v1','height',1);assert.equal(m.view('v1').w,320);assert.equal(m.view('v1').h,200);assert.deepEqual(m.views.slice(1),neighbours);const same=m.serialize();assert.deepEqual(Workspace.restore(same,hash(1)).views,m.views);
 for(const [id,axis,size]of[['absent','height',900],['v1','bad',900],['v1','height',NaN],['v1','height',Infinity],['v1','width',0],['v1','height',-1]]){assert.throws(()=>m.fitSize(id,axis,size));assert.equal(m.serialize(),same);}
 const edge=scene();edge.move('v1',100,99900);edge.move('v2',100,99950);const before=edge.serialize();assert.throws(()=>edge.fitSize('v1','height',900));assert.equal(edge.serialize(),before);
});
test('uncollapsing makes room while collapsed neighbours occupy only header space',()=>{
 const m=scene();m.collapse('v1');m.move('v2',100,180);m.collapse('v2');m.move('v3',100,260);m.collapse('v1');assert.equal(m.view('v1').collapsed,false);assert.equal(m.view('v2').y,524);assert.equal(m.view('v3').y,602);assert.equal(m.view('v2').collapsed,true);
});
test('a moved bubble clears an earlier fixed obstacle, while a fit that needs no growth leaves overlaps alone',()=>{
 const m=scene();m.move('v4',600,510);m.resize('v4',440,1000);m.move('v2',300,530);m.fitSize('v1','height',900);assert.equal(m.view('v4').y,510);assert.equal(m.view('v2').y,1534);
 const same=scene();same.move('v2',100,200);const prior=same.serialize();same.fitSize('v1','height',400);const after=JSON.parse(same.serialize()),before=JSON.parse(prior);assert.deepEqual(after.views,before.views);
});
test('untouched overlapping bubbles and unrelated fixed obstacles do not join the push',()=>{
 const m=scene();m.move('v4',1000,510);m.resize('v4',440,1500);m.fitSize('v1','height',900);assert.equal(m.view('v2').y,1024);assert.equal(m.view('v4').y,510);
 const untouched=scene();untouched.move('v2',1000,1100);untouched.move('v3',1000,1200);const prior=structuredClone(untouched.views);untouched.fitSize('v1','height',900);assert.deepEqual(untouched.view('v2'),prior[1]);assert.deepEqual(untouched.view('v3'),prior[2]);
});
test('bubble edges may touch without becoming an obstruction on either axis',()=>{
 for(const [axis,x,y]of[['height',-340,100],['height',540,100],['height',100,-300],['width',-340,100],['width',100,-300],['width',100,500]]){const m=new Workspace(hash(1));const a=m.openRoot(symbol(1),''),b=m.openRoot(symbol(2),'');m.move(a,100,100);m.move(b,x,y);const before=structuredClone(m.view(b));m.fitSize(a,axis,600);assert.deepEqual(m.view(b),before,axis+' '+x+','+y);}
});
test('horizontal overflow is atomic and positions at the exact world boundary are accepted',()=>{
 const m=scene();m.move('v1',99900,100);m.move('v2',99950,100);const old=m.serialize();assert.throws(()=>m.fitSize('v1','width',900));assert.equal(m.serialize(),old);
 for(const axis of ['width','height']){const edge=new Workspace(hash(1)),a=edge.openRoot(symbol(1),''),b=edge.openRoot(symbol(2),'');edge.move(a,axis==='width'?99376:100,axis==='height'?99276:100);edge.move(b,axis==='width'?99970:100,axis==='height'?99970:100);edge.fitSize(a,axis,axis==='width'?600:700);assert.equal(edge.view(b)[axis==='width'?'x':'y'],100000);}
});
test('reveal includes a fitting pair, prioritises an oversized destination, and uses screen-space margins',()=>{
 for(const scale of [.2,.5,1.5]){const m=new Workspace(hash(1)),a=m.openRoot(symbol(1),''),b=m.openRoot(symbol(2),'');m.move(a,100,100);m.move(b,700,100);m.camera={x:2000,y:2000,scale};m.reveal([a,b],2400,1600);for(const id of [a,b]){const v=m.view(id);assert.ok((v.x-m.camera.x)*scale>=24-1e-8);assert.ok((v.y-m.camera.y)*scale>=24-1e-8);}assert.equal(m.camera.scale,scale);}
 const m=scene();m.camera={x:0,y:0,scale:1};m.reveal(['v1'],1000,1000);assert.deepEqual(m.camera,{x:0,y:0,scale:1});m.reveal([],1000,1000);assert.deepEqual(m.camera,{x:0,y:0,scale:1});m.resize('v2',1600,2400);m.reveal(['v1','v2'],600,300);assert.equal((m.view('v2').x-m.camera.x),24);assert.equal(m.view('v2').y-m.camera.y,24);assert.equal(m.camera.scale,1);
 for(const [x,y]of [[3000,100],[100,3000]]){const n=scene();n.move('v2',x,y);n.reveal(['v1','v2'],1000,800);const v=n.view('v2');assert.ok(v.x>=n.camera.x+24);assert.ok(v.x+v.w<=n.camera.x+976);assert.ok(v.y>=n.camera.y+24);assert.ok(v.y+v.h<=n.camera.y+776);}
 for(const [x,y,w,h]of[[700,100,1088,1000],[100,700,1000,1048]]){const n=scene();n.move('v2',x,y);n.camera={x:2000,y:2000,scale:1};n.reveal(['v1','v2'],w,h);for(const id of ['v1','v2']){const v=n.view(id);assert.ok(v.x>=n.camera.x+24);assert.ok(v.y>=n.camera.y+24);}}
});
test('navigation handles a single oversized minimap target and preserves a visible distant position',()=>{
 const m=scene();m.resize('v1',1600,2400);m.reveal(['v1'],600,300);assert.equal(m.view('v1').x-m.camera.x,24);assert.equal(m.view('v1').y-m.camera.y,24);
 m.move('v2',4000,6000);m.camera={x:3900,y:5900,scale:1};m.reveal(['v2'],1200,1000);assert.deepEqual(m.camera,{x:3900,y:5900,scale:1});
 for(const [x,y,w,h]of[[700,100,1070,1000],[100,700,1000,1020]]){const n=scene();n.move('v2',x,y);n.camera={x:2000,y:2000,scale:1};n.reveal(['v1','v2'],w,h);const v=n.view('v2');assert.ok(v.x>=n.camera.x+24);assert.ok(v.x+v.w<=n.camera.x+w-24);assert.ok(v.y>=n.camera.y+24);assert.ok(v.y+v.h<=n.camera.y+h-24);}
});
test('expansion keeps a forward gap even when the new edge exactly reaches another bubble',()=>{
 for(const axis of ['width','height']){const m=new Workspace(hash(1)),a=m.openRoot(symbol(1),''),b=m.openRoot(symbol(2),'');m.move(a,100,100);m.move(b,axis==='width'?700:100,axis==='height'?700:100);m.fitSize(a,axis,600);assert.equal(m.view(b)[axis==='width'?'x':'y'],724);}
});
