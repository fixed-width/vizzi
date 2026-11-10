import {test} from 'node:test';
import assert from 'node:assert/strict';
import {validModule,modulePresentation,modulePeers,moduleGeometryKey} from './module-identity.js';
const id=n=>n.toString(16).padStart(64,'0');
const module=(n,label='audio::engine',root='audio/src/lib.rs')=>({id:id(n),label,root,language:'rust',basis:'rust_declarations'});
const view=(n,m)=>({id:'v'+n,x:n*500,y:100,w:440,h:400,collapsed:false,symbol:{module:m}});
test('native module identity drives stable presentation independently of view or version',()=>{
 const a=view(1,module(2)),b=view(2,module(2)),c=view(3,module(3,'audio'));
 assert.deepEqual(modulePresentation(a,[a,b,c]),modulePresentation(b,[a,b,c]));assert.deepEqual([...modulePeers([a,b,c],id(2))],['v1','v2']);assert.equal(modulePresentation(view(4,null),[]),null);
 const renamed={...a,symbol:{...a.symbol,id:id(90),source:{version:id(91)}}};assert.deepEqual(modulePresentation(renamed,[renamed,b,c]),modulePresentation(a,[a,b,c]));
});
test('equal labels in different roots are visibly qualified, without relying on colour',()=>{
 const a=view(1,module(1,'audio','one/src/lib.rs')),b=view(2,module(2,'audio','two/src/lib.rs'));const first=modulePresentation(a,[a,b]),second=modulePresentation(b,[a,b]);assert.notEqual(first.label,second.label);assert.match(first.title,/one\/src\/lib.rs/);assert.match(second.title,/two\/src\/lib.rs/);
});
test('outline geometry ignores camera and reading state but tracks membership, movement and collapse',()=>{
 const a=view(1,module(1)),b=view(2,module(1));const key=moduleGeometryKey([a,b]);assert.equal(moduleGeometryKey([{...a,reading:{top:42}},b]),key);assert.notEqual(moduleGeometryKey([{...a,x:43},b]),key);assert.notEqual(moduleGeometryKey([{...a,collapsed:true},b]),key);assert.notEqual(moduleGeometryKey([{...a,symbol:{module:module(2)}},b]),key);
});
test('module descriptors accept legacy absence and supported provenance but reject malformed persisted data',()=>{
 for(const m of [undefined,null,module(1),{...module(2),language:'python',basis:'python_module'}, {...module(3),label:'é'.repeat(256),root:'a'.repeat(4096)}])assert.equal(validModule(m),true);
 for(const m of [false,0,'module',{}, {...module(1),id:'x'.repeat(64)}, {...module(1),id:id(1)+'0'}, {...module(1),label:''}, {...module(1),label:'é'.repeat(257)}, {...module(1),label:3}, {...module(1),root:''}, {...module(1),root:3}, {...module(1),root:'é'.repeat(2049)}, ...['/root','a/../b','a/./b','a//b','a\\b'].map(root=>({...module(1),root})), {...module(1),language:'python'}, {...module(1),basis:'compiler'}, {...module(1),language:'c'}])assert.equal(validModule(m),false,JSON.stringify(m));
});
test('palette identity and full-root disambiguation do not merge distinct modules',()=>{
 const a=view(1,module(7,'audio','one/lib.rs')),b=view(2,module(8,'audio','two/lib.rs')),c=view(3,module(9,'other','three.rs'));
 assert.equal(modulePresentation(a,[a,b,c]).colour,1);assert.equal(modulePresentation(b,[a,b,c]).colour,2);assert.equal(modulePresentation(c,[a,b,c]).label,'other');assert.equal(modulePresentation(a,[a,b,c]).label,'one/lib.rs · audio');assert.equal(modulePresentation({...a,symbol:{module:{}}},[a]),null);assert.deepEqual([...modulePeers([a,b,c],id(8))],['v2']);assert.deepEqual([...modulePeers([a,b,c],id(10))],[]);
 const key=moduleGeometryKey([a,b]);for(const property of ['x','y','w','h'])assert.notEqual(moduleGeometryKey([{...a,[property]:a[property]+1},b]),key);assert.notEqual(moduleGeometryKey([{...a,id:'v99'},b]),key);
});
test('small working sets distinguish colliding module colours and retain colours when peers are added',()=>{
 const a=view(1,module(1)),b=view(2,module(7,'audio')),c=view(3,module(13,'audio::other')),views=[a,b,c];
 const colours=views.map(v=>modulePresentation(v,views).colour);assert.equal(new Set(colours).size,3);assert.equal(colours[0],modulePresentation(a,[a,b]).colour);assert.equal(colours[1],modulePresentation(b,[a,b]).colour);assert.deepEqual(views.map(v=>modulePresentation(v,structuredClone(views)).colour),colours);
 const more=[...views,view(4,module(19)),view(5,module(25)),view(6,module(31))];assert.equal(new Set(more.map(v=>modulePresentation(v,more).colour)).size,6);assert.ok(Number.isInteger(modulePresentation(view(7,module(37)),[...more,view(7,module(37))]).colour));
});
test('mixed known and neutral views, Python labels and several disambiguation depths remain readable',()=>{
 const a=view(1,module(1)),empty=view(2,null),legacy=view(3,undefined),python=view(4,{...module(2,'pkg.worker','pkg/worker.py'),language:'python',basis:'python_module'});
 assert.equal(modulePresentation(a,[a,empty,legacy,python]).colour,1);assert.match(modulePresentation(python,[a,empty,legacy,python]).title,/Python module\nRoot: pkg\/worker.py/);assert.deepEqual([...modulePeers([a,empty,legacy,python],id(1))],['v1']);assert.notEqual(moduleGeometryKey([a,empty]),moduleGeometryKey([a,python]));
 const roots=['x/shared/lib.rs','y/shared/lib.rs','y/other/lib.rs'],views=roots.map((root,i)=>view(i+1,module(i+1,'audio',root)));assert.deepEqual(views.map(v=>modulePresentation(v,views).label),['x/shared/lib.rs · audio','y/shared/lib.rs · audio','other/lib.rs · audio']);
 const long=view(1,module(1));long.symbol.module.id='0'.repeat(56)+'f1234567';assert.equal(modulePresentation(long,[long]).colour,parseInt('f1234567',16)%6);
});
test('persisted descriptors reject coercible values and labels retain language and root context',()=>{
 for(const invalid of [{...module(1),id:[id(1)]},{...module(1),label:['audio']},{...module(1),root:['src/lib.rs']},{...module(1),label:new String('audio')}])assert.equal(validModule(invalid),false);
 const a=view(1,module(1)),p=modulePresentation(a,[a]);assert.match(p.title,/audio::engine/);assert.match(p.title,/Rust module/);assert.match(p.title,/Root: audio\/src\/lib.rs/);
 const long=view(2,{...module(2),id:'1234567f'.repeat(8)});assert.equal(modulePresentation(long,[long]).colour,parseInt('1234567f',16)%6);
 const roots=['pkg/a/x/lib.rs','pkg/b/y/lib.rs'],views=roots.map((root,i)=>view(i+1,module(i+1,'audio',root)));assert.deepEqual(views.map(v=>modulePresentation(v,views).label),['x/lib.rs · audio','y/lib.rs · audio']);
});
test('each provenance is accepted only with its supplying language',()=>{
 for(const language of ['rust','python','unknown'])for(const basis of ['rust_declarations','python_module'])assert.equal(validModule({...module(1),language,basis}),(language==='rust'&&basis==='rust_declarations')||(language==='python'&&basis==='python_module'));
});
