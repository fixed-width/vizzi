import {test} from 'node:test';import assert from 'node:assert/strict';
import {TaskWorkspace} from './task-workspace.js';import {createCallPreparation} from './call-delivery.js';
const id=n=>String(n).repeat(64),snapshot=id(1);
const symbol=n=>({id:id(n),name:'fn'+n,kind:'function',parents:[],source:{file:id(9),version:id(8),context:id(7),path:'src/a.rs',line:n*10,end_line:n*10+9}});
const caller=symbol(2),target=symbol(3),site={id:id(4),owner:caller.id,name:'fn3',spelling:'fn3',source:{...caller.source,line:21,end_line:21},candidates:[target],candidate_count:1};
function fixture({resolve=async site=>({site,attempted:false,note:'',semantic:false}),source=async symbol=>({symbol}),bind=async()=>({snapshot,caller,invocation:site})}={}){let model=new TaskWorkspace(snapshot);const applied=[];
 const prepare=createCallPreparation({read:()=>model,bind,resolve,source,apply:data=>applied.push(data)});return {model,applied,prepare,setModel:next=>{model=next;}};}
test('call preparation reads caller and destination without mutating the working set before commit',async()=>{const f=fixture();const prepared=await f.prepare({kind:'call',snapshot,call:site.id},{signal:new AbortController().signal});assert.equal(f.model.views.length,0);assert.deepEqual(f.applied,[]);assert.equal(prepared.commit(),true);assert.equal(f.model.views.length,2);assert.equal(f.model.links.length,1);assert.equal(f.applied[0].sources.size,2);});
test('a normal ambiguity chooser applies with readable caller without choosing for the user',async()=>{const f=fixture({resolve:async()=>({site:{...site,candidate_count:2},attempted:true,semantic:true,note:'choose'})});const p=await f.prepare({snapshot,call:site.id},{signal:new AbortController().signal});p.commit();assert.equal(f.model.views.length,1);assert.equal(f.model.links.length,0);assert.equal(f.applied[0].target,null);});
test('source loading failure refuses without opening partial bubbles',async()=>{const f=fixture({source:async()=>{throw new Error('source unavailable');}});await assert.rejects(f.prepare({snapshot,call:site.id},{signal:new AbortController().signal}),/source unavailable/);assert.deepEqual(f.model.views,[]);});
test('layout changes during preparation are retained on commit and repeated calls reuse fragments',async()=>{const f=fixture();const root=f.model.openRoot(caller,'src');f.model.move(root,300,200);const p=await f.prepare({snapshot,call:site.id},{signal:new AbortController().signal});f.model.move(root,900,600);p.commit();assert.equal(f.model.view(root).x,900);assert.equal(f.model.view(root).y,600);const again=await f.prepare({snapshot,call:site.id},{signal:new AbortController().signal});again.commit();assert.equal(f.model.views.length,2);assert.equal(f.model.links.length,1);});

test('source changes during resolution refuse before loading or applying fragments',async()=>{
 let reads=0;const f=fixture({resolve:async site=>{f.model.markSnapshot(id(5));return {site,attempted:false,semantic:false,note:''};},source:async()=>{reads++;}});
 await assert.rejects(f.prepare({snapshot,call:site.id},{signal:new AbortController().signal}),/source changed/i);assert.equal(reads,0);assert.deepEqual(f.model.views,[]);
});
test('cancelled source preparation cannot commit its already resolved data',async()=>{
 const f=fixture(),controller=new AbortController();const p=await f.prepare({snapshot,call:site.id},{signal:controller.signal});controller.abort();assert.throws(()=>p.commit(),/cancel/i);assert.deepEqual(f.model.views,[]);
});
test('a unique name match remains a chooser after unavailable semantic lookup',async()=>{
 const f=fixture({resolve:async site=>({site,attempted:true,semantic:false,note:'provider unavailable'})});const p=await f.prepare({snapshot,call:site.id},{signal:new AbortController().signal});p.commit();assert.equal(f.model.views.length,1);assert.equal(f.applied[0].target,null);
});

test('lightweight inline actions are resolved against the explicitly selected caller',async()=>{
 const {resolveCallChoice}=await import('./call-delivery.js');
 const result=await resolveCallChoice({id:site.id,name:'fn3',owner:caller.id,lookup:true},{snapshot,provider:true,warmup:async()=>{},request:async()=>({snapshot,invocation:site}),signal:new AbortController().signal});assert.equal(result.site.owner,caller.id);assert.equal(result.site.candidates[0].id,target.id);
 await assert.rejects(resolveCallChoice({id:site.id,owner:target.id,lookup:true},{snapshot,provider:true,warmup:async()=>{},request:async()=>({snapshot,invocation:site})}),/differs/);
});

test('manual calls without a provider and already bound calls retain their original evidence',async()=>{
 const {resolveCallChoice}=await import('./call-delivery.js');let requests=0,warmups=0;
 const options={snapshot,warmup:async()=>{warmups++;},request:async()=>{requests++;},verify:()=>{}};
 const plain=await resolveCallChoice(site,{...options,provider:false});assert.deepEqual(plain,{site,attempted:false,note:'',semantic:false});
 const bound={...site,binding:'rust_self_receiver'};const known=await resolveCallChoice(bound,{...options,provider:true});assert.equal(known.site,bound);assert.equal(known.attempted,false);assert.equal(requests,0);assert.equal(warmups,0);
});
test('semantic lookup preserves its source checks, context, detail and selected definition',async()=>{
 const {resolveCallChoice,directDestination}=await import('./call-delivery.js');const order=[];
 const semantic={...site,binding:'language_server_definition',semantic_context:id(6)};
 const result=await resolveCallChoice(site,{snapshot,provider:true,warmup:async()=>order.push('warmup'),verify:()=>order.push('verify'),request:async(s,c)=>{assert.equal(s,snapshot);assert.equal(c,site.id);order.push('request');return {snapshot,invocation:semantic,state:'unique',detail:'resolved'};}});
 assert.deepEqual(order,['warmup','verify','request','verify']);assert.equal(result.attempted,true);assert.equal(result.semantic,true);assert.equal(result.note,'resolved');assert.equal(directDestination(result),target);
});
test('lookup response snapshot, invocation identity and caller identity are checked independently',async()=>{
 const {resolveCallChoice}=await import('./call-delivery.js');
 for(const response of [{snapshot:id(5),invocation:site},{snapshot,invocation:{...site,id:id(5)}},{snapshot,invocation:{...site,owner:target.id}},{snapshot,invocation:null}]){
  await assert.rejects(resolveCallChoice({...site,lookup:true},{snapshot,provider:true,warmup:async()=>{},request:async()=>response}),/differs/);
 }
});
test('ordinary unavailable semantic results retain name candidates while explicit lookup adopts the returned invocation',async()=>{
 const {resolveCallChoice}=await import('./call-delivery.js');const returned={...site,candidates:[],candidate_count:0};
 for(const state of ['unresolved','unavailable']) {
  const result=await resolveCallChoice(site,{snapshot,provider:true,warmup:async()=>{},request:async()=>({snapshot,invocation:returned,state,detail:'not resolved'})});assert.equal(result.site,site);assert.equal(result.note,'not resolved');assert.equal(result.semantic,false);
  const explicit=await resolveCallChoice({...site,lookup:true},{snapshot,provider:false,warmup:async()=>{},request:async()=>({snapshot,invocation:returned,state})});assert.equal(explicit.site,returned);
 }
 for(const state of ['unique','ambiguous','outside']){const result=await resolveCallChoice(site,{snapshot,provider:true,warmup:async()=>{},request:async()=>({snapshot,invocation:returned,state})});assert.equal(result.site,returned);}
});
test('cancellation while joining definition warmup does not start the foreground lookup',async()=>{
 const {resolveCallChoice}=await import('./call-delivery.js');let release;const warm=new Promise(r=>{release=r;}),controller=new AbortController();let requests=0;
 const result=resolveCallChoice(site,{snapshot,provider:true,signal:controller.signal,warmup:()=>warm,request:async()=>{requests++;}});controller.abort();await assert.rejects(result,/cancel/i);release();assert.equal(requests,0);
});
test('definition lookup failures and changed active callers propagate without choosing a target',async()=>{
 const {resolveCallChoice}=await import('./call-delivery.js'),failure=new Error('lookup unavailable');
 await assert.rejects(resolveCallChoice(site,{snapshot,provider:true,warmup:async()=>{},request:async()=>{throw failure;}}),e=>e===failure);
 let requests=0;await assert.rejects(resolveCallChoice(site,{snapshot,provider:true,warmup:async()=>{},verify:()=>{throw failure;},request:async()=>{requests++;}}),e=>e===failure);assert.equal(requests,0);
});
test('direct destination selection never guesses among hidden or ambiguous candidates',async()=>{
 const {directDestination}=await import('./call-delivery.js');
 for(const changed of [{...site,candidate_count:2},{...site,candidates:[]},{...site,candidates:[target,caller],candidate_count:2}])assert.equal(directDestination({site:changed,attempted:false,semantic:true}),null);
 assert.equal(directDestination({site,attempted:true,semantic:false}),null);assert.equal(directDestination({site:{...site,binding:'rust_self_receiver'},attempted:true,semantic:false}),target);
});

test('only the requested captured invocation and its admitted caller can be prepared',async()=>{
 for(const binding of [{snapshot:id(5),caller,invocation:site},{snapshot,caller,invocation:{...site,id:id(5)}},{snapshot,caller,invocation:{...site,owner:target.id}},{snapshot,caller:null,invocation:site},{snapshot,caller,invocation:null}]) {
  const f=fixture({bind:async()=>binding});await assert.rejects(f.prepare({snapshot,call:site.id},{signal:new AbortController().signal}),/differs/);assert.deepEqual(f.model.views,[]);
 }
});
test('unavailable or obsolete workspaces refuse before querying an invocation',async()=>{
 let queries=0;const f=fixture({bind:async()=>{queries++;}});f.setModel(null);await assert.rejects(f.prepare({snapshot,call:site.id},{signal:new AbortController().signal}),/source changed/i);assert.equal(queries,0);
 const g=fixture({bind:async()=>{queries++;}});await assert.rejects(g.prepare({snapshot:id(6),call:site.id},{signal:new AbortController().signal}),/source changed/i);assert.equal(queries,0);
});
test('a task replaced while invocation data arrives cannot prepare or mutate its old model',async()=>{
 const f=fixture({bind:async()=>{f.setModel(new TaskWorkspace(snapshot));return {snapshot,caller,invocation:site};}});await assert.rejects(f.prepare({snapshot,call:site.id},{signal:new AbortController().signal}),/differs/);assert.deepEqual(f.model.views,[]);
});
test('source preparation always requests fresh lookup and retains source-area identity',async()=>{
 const f=fixture({resolve:async current=>{assert.equal(current.lookup,true);return {site:current,attempted:false,note:'',semantic:false};}});const p=await f.prepare({snapshot,call:site.id},{signal:new AbortController().signal});p.commit();assert.equal(f.model.views[0].area,'src');
});
test('a source publication or task replacement during source loading prevents prepared confirmation',async()=>{
 for(const replace of [false,true]){const f=fixture({source:async()=>{if(replace)f.setModel(new TaskWorkspace(snapshot));else f.model.markSnapshot(id(6));return {};}});await assert.rejects(f.prepare({snapshot,call:site.id},{signal:new AbortController().signal}),/source changed/i);assert.deepEqual(f.model.views,[]);}
});
test('committing a preparation twice or after source/task replacement cannot repeat its effects',async()=>{
 const f=fixture();const p=await f.prepare({snapshot,call:site.id},{signal:new AbortController().signal});p.commit();assert.throws(()=>p.commit(),/source changed/i);assert.equal(f.applied.length,1);
 for(const replace of [false,true]) {const g=fixture();const next=await g.prepare({snapshot,call:site.id},{signal:new AbortController().signal});if(replace)g.setModel(new TaskWorkspace(snapshot));else g.model.markSnapshot(id(6));assert.throws(()=>next.commit(),/source changed/i);assert.deepEqual(g.applied,[]);}
});
test('reopening collapsed requested panes makes caller and destination readable while preserving their geometry',async()=>{
 const f=fixture();const root=f.model.openRoot(caller,'src');const destination=f.model.follow(root,site,target);f.model.collapse(root);f.model.collapse(destination);const before=f.model.views.map(v=>[v.x,v.y,v.w,v.h]);
 const p=await f.prepare({snapshot,call:site.id},{signal:new AbortController().signal});p.commit();assert.ok(f.model.views.every(v=>!v.collapsed));assert.deepEqual(f.model.views.map(v=>[v.x,v.y,v.w,v.h]),before);
});
