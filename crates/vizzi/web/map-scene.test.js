import {test} from 'node:test';import assert from 'node:assert/strict';
import {createMapModel} from './map-scene.js';
const files=paths=>paths.map((path,i)=>({id:'f'+i,path,bytes:10,coverage:'syntax',classification:{kind:'assigned'}}));
const sample=()=>files(['src/compiler/a.rs','src/compiler/b.rs','src/runtime/c.rs','tests/check.rs']);
const inside=(a,b)=>a.x>=b.x && a.y>=b.y && a.x+a.width<=b.x+b.width+.001 && a.y+a.height<=b.y+b.height+.001;
test('directory containment and compact paths are explicit and every file stays in its own rectangle',()=>{
 const model=createMapModel(sample()),scene=model.project();assert.equal(scene.files.length,4);assert.equal(scene.groups.find(g=>g.path==='src/compiler').parent,'src');
 for(const file of scene.files){const dir=scene.groups.find(g=>g.path===file.path.slice(0,file.path.lastIndexOf('/')));assert.ok(inside(file.rect,dir.rect));}
 for(const group of scene.groups){const parent=scene.groups.find(g=>g.path===group.parent);if(parent)assert.ok(inside(group.rect,parent.rect));}
 const compact=createMapModel(files(['a/b/c/x.rs','a/b/c/y.rs'])).project();assert.equal(compact.groups.length,1);assert.equal(compact.groups[0].label,'a/b/c');
});
test('collapse retains truthful counts and selection can reveal ancestors without reading source',()=>{
 const model=createMapModel(sample());model.toggle('src');let scene=model.project();assert.equal(scene.files.length,1);assert.equal(scene.groups.find(g=>g.path==='src').count,3);assert.equal(scene.hiddenFiles,3);
 model.reveal('f0');scene=model.project();assert.equal(scene.files.length,4);assert.ok(scene.groups.every(g=>!g.collapsed));
 model.focus('src/compiler');assert.equal(model.project().files.length,2);assert.deepEqual(model.project().breadcrumbs.map(d=>d.path),['.','src','src/compiler']);model.focus('.');assert.equal(model.project().files.length,4);
});
test('reordering siblings changes the map and survives reconstruction, filters preserve preferences, reset restores defaults',()=>{
 const model=createMapModel(sample());assert.equal(model.move('src/runtime','src/compiler'),true);let scene=model.project();assert.ok(scene.groups.findIndex(g=>g.path==='src/runtime')<scene.groups.findIndex(g=>g.path==='src/compiler'));
 assert.equal(model.move('src/compiler','tests'),false);model.toggle('src/runtime');const prefs=model.preferences();const restored=createMapModel(sample(),prefs);assert.deepEqual(restored.project(),scene=createMapModel(sample(),prefs).project());
 restored.setFiles(sample().filter(f=>f.path.startsWith('tests')));restored.setFiles(sample());assert.equal(restored.project().groups.find(g=>g.path==='src/runtime').collapsed,true);
 restored.reset();assert.ok(restored.project().groups.findIndex(g=>g.path==='src/compiler')<restored.project().groups.findIndex(g=>g.path==='src/runtime'));assert.equal(restored.project().files.length,4);
});
test('large, deeply nested and empty inputs have bounded geometry without overlapping sibling tiles',()=>{
 const paths=Array.from({length:2000},(_,i)=>`d${i%73}/nested/${i}.rs`);const scene=createMapModel(files(paths)).project();assert.ok(scene.groups.length<=4001);assert.ok(scene.files.length<=2000);
 for(const file of scene.files)assert.ok(Number.isFinite(file.rect.x) && file.rect.width>0 && inside(file.rect,{x:0,y:0,width:scene.width,height:scene.height}));
 const deep=createMapModel(files(Array.from({length:60},(_,i)=>'a/'.repeat(i)+i+'.rs')));assert.ok(deep.project().groups.length<=12);assert.ok(deep.project().hiddenFiles>0);const last=deep.project().groups.at(-1);deep.focus(last.path);assert.ok(deep.project().files.length>0);
 const empty=createMapModel([]).project();assert.deepEqual(empty.files,[]);assert.ok(empty.width>0 && empty.height>0);
});
test('invalid preferences cannot hide unknown paths or reorder across parents',()=>{
 const model=createMapModel(sample(),{collapsed:['missing'],order:[['src',['tests','src/runtime','src/runtime']]]});assert.equal(model.project().files.length,4);assert.equal(model.toggle('missing'),false);assert.equal(model.focus('missing'),false);assert.equal(model.move('src','missing'),false);
});

test('file metadata refreshes preserve layout, subsequent interactions use fresh metadata',()=>{
 const initial=sample(),model=createMapModel(initial),before=model.project();
 const next=initial.map(f=>({...f,bytes:99,coverage:'partial'}));model.setFiles(next);let after=model.project();assert.deepEqual(after.files.map(f=>f.rect),before.files.map(f=>f.rect));assert.ok(after.files.every(f=>f.bytes===99 && f.coverage==='partial'));assert.ok(initial.every(f=>f.bytes===10));
 model.toggle('src/compiler');model.toggle('src/compiler');after=model.project();assert.ok(after.files.every(f=>f.bytes===99));
});
test('many single-file directories and a highly skewed directory stay bounded',()=>{
 for(const paths of [Array.from({length:2000},(_,i)=>`dir${i}/a.rs`),[...Array.from({length:1999},(_,i)=>`huge/${i}.rs`),'root.rs']]){
  const scene=createMapModel(files(paths)).project();assert.ok(scene.groups.length<=4001);for(const f of scene.files)assert.ok(inside(f.rect,{x:0,y:0,width:scene.width,height:scene.height}));assert.ok(scene.files.every(f=>f.rect.width>0 && f.rect.height>0));
 }
});

test('compact hierarchy distinguishes path-component prefixes and is independent of input order',()=>{
 const paths=['other/X/a.rs','src/compiler/a.rs','other/Y/b.rs','src/runtime/b.rs','src/local.rs','a/b/one.rs','a/b2/two.rs','ab/c/three.rs','root.rs'];
 const first=createMapModel(files(paths)).project(),second=createMapModel(files([...paths].reverse())).project();
 assert.deepEqual(first.groups.map(g=>[g.path,g.parent,g.label,g.count]),second.groups.map(g=>[g.path,g.parent,g.label,g.count]));
 for(const [path,parent] of [['a/b','a'],['a/b2','a'],['src/compiler','src'],['src/runtime','src'],['other/X','other'],['other/Y','other'],['ab/c','.']])assert.equal(first.groups.find(g=>g.path===path).parent,parent);
 assert.equal(first.groups.find(g=>g.path==='src').count,3);assert.equal(first.groups.find(g=>g.path==='src/compiler').label,'compiler');
});
test('directory control metadata and breadcrumbs truthfully describe hierarchy, sibling moves and focus',()=>{
 const model=createMapModel(sample());const fields=s=>s.tree.map(g=>[g.path,g.parent,g.label,g.depth,g.earlier,g.later]);
 assert.deepEqual(fields(model.project()),[['src','.','src',0,false,true],['src/compiler','src','compiler',1,false,true],['src/runtime','src','runtime',1,true,false],['tests','.','tests',0,true,false]]);
 model.focus('src/compiler');assert.equal(model.project().focus,'src/compiler');assert.deepEqual(model.project().breadcrumbs.map(g=>g.label),['Repository','src','compiler']);
 model.move('src/compiler',null);assert.deepEqual(fields(model.project()).slice(1,3),[['src/runtime','src','runtime',1,false,true],['src/compiler','src','compiler',1,true,false]]);
});
test('collapsed groups reclaim space while maintaining the file and hidden-file accounting',()=>{
 const input=files([...Array.from({length:300},(_,i)=>`large/${i}.rs`),...Array.from({length:20},(_,i)=>`small/${i}.rs`)]),model=createMapModel(input),before=model.project();model.toggle('large');const after=model.project();
 assert.ok(after.width*after.height<before.width*before.height/2);assert.equal(after.hiddenFiles,300);assert.equal(after.files.length,20);assert.equal(after.groups.find(g=>g.path==='large').count,300);
 model.focus('large');assert.equal(model.project().files.length,300);assert.equal(model.project().hiddenFiles,0);
});
test('projections keep usable world area, sibling separation and complete file identity accounting',()=>{
 for(const paths of [Array.from({length:140},(_,i)=>`d${i%9}/nested/${i}.rs`),Array.from({length:2000},(_,i)=>`d${i}/a.rs`)]){
  const input=files(paths),scene=createMapModel(input).project();assert.deepEqual(scene.files.map(f=>f.id).sort(),input.map(f=>f.id).sort());assert.ok(scene.width*scene.height>=scene.files.length*8000);assert.ok(scene.width/scene.height>.4 && scene.width/scene.height<2.5);
  for(let i=0;i<scene.files.length;i++)for(let j=i+1;j<scene.files.length;j++){const a=scene.files[i].rect,b=scene.files[j].rect;assert.ok(a.x+a.width<=b.x+.001 || b.x+b.width<=a.x+.001 || a.y+a.height<=b.y+.001 || b.y+b.height<=a.y+.001);}
 }
});
test('saved focus and collapse are restored; missing focus resets and invalid state changes are harmless',()=>{
 const model=createMapModel(sample());model.focus('src/compiler');model.toggle('src/runtime');const restored=createMapModel(sample(),model.preferences());assert.equal(restored.project().focus,'src/compiler');
 let version=restored.revision;restored.setFiles(sample().filter(f=>f.path.startsWith('tests/')));assert.equal(restored.project().focus,'.');assert.ok(restored.revision>version);
 const before=restored.project();version=restored.revision;restored.reveal('absent');assert.equal(restored.move('.',null),false);assert.equal(restored.move('absent',null),false);assert.equal(restored.move('tests','tests'),false);assert.equal(restored.project(),before);assert.equal(restored.revision,version);
 restored.setFiles(sample());restored.focus('tests');restored.reveal('f0');assert.equal(restored.project().focus,'.');assert.equal(restored.project().files.length,3); // runtime is still closed
});
test('geometry projections and tree references stay stable until their geometry changes',()=>{
 const model=createMapModel(sample()),a=model.project();assert.equal(model.project(),a);model.setFiles(sample().map(f=>({...f,bytes:44})));const b=model.project();assert.equal(b.tree,a.tree);assert.equal(b.groups,a.groups);
 const version=model.revision;model.toggle('src');assert.ok(model.revision>version);assert.notEqual(model.project().tree,b.tree);const after=model.revision;model.toggle('src');assert.ok(model.revision>after);
});
test('file admission rejects excessive inputs atomically and metadata can change before the first projection',()=>{
 const input=sample(),model=createMapModel(input),before=model.project();assert.throws(()=>model.setFiles(files(Array.from({length:2001},(_,i)=>`d/${i}.rs`))));assert.deepEqual(model.project(),before);
 model.focus('tests');model.reveal('f0');assert.equal(model.project().focus,'.');model.toggle('src/compiler');model.toggle('src/compiler');assert.deepEqual(model.project().files.map(f=>f.id).sort(),input.map(f=>f.id).sort());
 const fresh=createMapModel(input);fresh.setFiles(input.map(f=>({...f,bytes:77})));assert.ok(fresh.project().files.every(f=>f.bytes===77));
});

test('displayed files and directory counts follow collapse and focus actions for every selected file',()=>{
 const input=files(['root.rs','src/local.rs','src/compiler/a.rs','src/compiler/b.rs','src/runtime/c.rs','tests/unit/a.rs','tests/integration/a.rs']);const model=createMapModel(input),closed=new Set();let focus='.';
 const isWithin=(file,dir)=>dir==='.' || file.startsWith(dir+'/');
 const check=()=>{const scene=model.project(),scoped=input.filter(f=>isWithin(f.path,focus)),expected=scoped.filter(f=>![...closed].some(p=>isWithin(f.path,p)));assert.deepEqual(scene.files.map(f=>f.id).sort(),expected.map(f=>f.id).sort());assert.equal(scene.hiddenFiles,scoped.length-expected.length);for(const dir of scene.groups){assert.ok(input.some(f=>isWithin(f.path,dir.path)),`invented directory ${dir.path}`);assert.equal(dir.count,input.filter(f=>isWithin(f.path,dir.path)).length);}};
 check();for(const path of ['src','tests/unit','src']){model.toggle(path);if(closed.has(path))closed.delete(path);else closed.add(path);check();}
 model.focus('src/compiler');focus='src/compiler';closed.delete(focus);check();model.reveal('f2');check(); // selecting inside the focused directory must keep its focus
 model.reveal('f6');focus='.';for(const path of [...closed])if(isWithin(input[6].path,path))closed.delete(path);check();
});
test('corrupt preference containers are ignored and bounded valid entries are retained',()=>{
 for(const value of [null,'bad',{collapsed:'src'},{order:42},{order:[42,[],[null,[]],['src','bad']]},{focus:{}},{focus:'missing'}])assert.equal(createMapModel(sample(),value).project().files.length,4);
 const mixed=createMapModel(sample(),{collapsed:[null,42,{},'src'],order:[['src',[null,'src/runtime','tests',42]]]});assert.equal(mixed.project().files.length,1);
 const limit='x'.repeat(16384),tooLong=limit+'x',bounded=createMapModel(sample(),{collapsed:[limit,tooLong,...Array.from({length:2005},(_,i)=>'absent'+i)],order:Array.from({length:2005},(_,i)=>['parent'+i,['child'+i]])}).preferences();
 assert.ok(bounded.collapsed.includes(limit));assert.ok(!bounded.collapsed.includes(tooLong));assert.ok(bounded.collapsed.length<=2000);assert.ok(bounded.order.length<=2000);
});
