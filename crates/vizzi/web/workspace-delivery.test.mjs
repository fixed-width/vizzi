import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createDeliveryClient,createDeliveryPage} from './workspace-delivery.mjs';
const id=n=>String(n).repeat(32), native=n=>String(n).repeat(64);
const deferred=()=>{let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return {promise,resolve,reject};};
const turn=()=>new Promise(setImmediate);
function fixture({ready=()=>true,prepare,validate=async()=>{},mode='call',now=Date.now,timeout=60000,sendResult}={}) {
  const applied=[],events=[],errors=[];let client,page,counter=0;
  client=createDeliveryClient({send:message=>{events.push(['client',message]);queueMicrotask(()=>page.receive(message));return sendResult?.(message);},makeId:()=>id(++counter),now,timeout});
  page=createDeliveryPage({mode,document:id(2),ready,prepare:prepare|| (async action=>({commit(){applied.push(action);}})),
    send:message=>{events.push(['page',message]);queueMicrotask(()=>client.receive(message));},makeId:()=>id(3),now,failed:error=>errors.push(error)});
  page.connect();
  return {client,page,applied,events,errors,deliver:(action,options={})=>client.deliver(action,{validate,...options}),close(){client.dispose();page.dispose();}};
}
const call={kind:'call',snapshot:native(1),call:native(2)};
test('delivery prepares, verifies original source, commits and confirms the applied result',async t=>{
  const order=[];const f=fixture({prepare:async()=>{order.push('prepare');return {commit(){order.push('commit');}};},validate:async()=>order.push('verify')});t.after(()=>f.close());
  const result=await f.deliver(call);assert.equal(result.state,'applied');assert.deepEqual(order,['prepare','verify','commit']);
});
test('a posted action remains pending until the application is ready and applied',async t=>{
  let ready=false,done=false;const f=fixture({ready:()=>ready});t.after(()=>f.close());
  const request=f.deliver(call).then(result=>{done=true;return result;});await turn();assert.equal(done,false);assert.deepEqual(f.applied,[]);
  ready=true;f.page.resume();assert.equal((await request).state,'applied');assert.equal(f.applied.length,1);
});
test('manual navigation cancels a waiting action and suppresses its late failure',async t=>{
  const gate=deferred(),started=deferred();const f=fixture({prepare:async()=>{started.resolve();await gate.promise;throw new Error('obsolete');}});t.after(()=>f.close());
  const request=f.deliver(call);await started.promise;f.page.manual();assert.equal((await request).state,'cancelled');gate.resolve();await turn();assert.deepEqual(f.applied,[]);assert.deepEqual(f.errors,[]);
});
test('source changed after browser preparation refuses delivery without committing',async t=>{
  const f=fixture({validate:async()=>{throw new Error('editor source changed');}});t.after(()=>f.close());
  await assert.rejects(f.deliver(call),/source changed/);assert.deepEqual(f.applied,[]);
});
test('the latest request replaces a hidden waiting action',async t=>{
  let ready=false,counter=0;const f=fixture({ready:()=>ready});t.after(()=>f.close());
  const first=f.deliver(call);await turn();const second=f.deliver({...call,call:native(3)});await turn();ready=true;f.page.resume();
  assert.equal((await first).state,'cancelled');assert.equal((await second).state,'applied');assert.deepEqual(f.applied,[{...call,call:native(3)}]);
});

test('false application result is a refusal rather than a successful delivery',async t=>{const f=fixture({prepare:async()=>({commit:()=>false})});t.after(()=>f.close());await assert.rejects(f.deliver(call),/source changed/i);assert.deepEqual(f.applied,[]);});
test('preparation failures are returned only to the current editor command',async t=>{const f=fixture({prepare:async()=>{throw new Error('lookup failed');}});t.after(()=>f.close());await assert.rejects(f.deliver(call),/lookup failed/);assert.deepEqual(f.errors,[]);});
test('source verification cancelled by manual navigation cannot send a late commit',async t=>{
 const checked=deferred(),gate=deferred();const f=fixture({validate:async()=>{checked.resolve();await gate.promise;}});t.after(()=>f.close());const request=f.deliver(call);await checked.promise;f.page.manual();assert.equal((await request).state,'cancelled');gate.resolve();await turn();assert.equal(f.events.filter(([side,m])=>side==='client'&&m.type==='commit').length,0);
});
test('expiry during source verification is unconfirmed and never applies the prepared target',async t=>{
 let clock=1;const f=fixture({now:()=>clock,validate:async()=>{clock=60002;}});t.after(()=>f.close());await assert.rejects(f.deliver(call),/could not confirm/i);assert.deepEqual(f.applied,[]);
});
test('a hidden view at final commit refuses instead of reporting successful navigation',async t=>{
 let visible=true;const f=fixture({ready:()=>visible,validate:async()=>{visible=false;}});t.after(()=>f.close());await assert.rejects(f.deliver(call),/no longer ready/);assert.deepEqual(f.applied,[]);
});
test('aborting before delivery sends nothing, while closing cancels pending delivery',async t=>{
 const f=fixture({ready:()=>false});t.after(()=>f.close());const controller=new AbortController();controller.abort();assert.equal((await f.deliver(call,{signal:controller.signal})).state,'cancelled');assert.equal(f.events.filter(([side,m])=>side==='client'&&m.type==='prepare').length,0);
 const request=f.deliver(call);await turn();f.client.dispose();assert.equal((await request).state,'cancelled');
});
test('a closed transport rejecting a cancellation does not produce an unhandled failure',async t=>{
 const f=fixture({ready:()=>false,sendResult:message=>message.type==='cancel'?false:true});t.after(()=>f.close());const request=f.deliver(call);await turn();f.client.dispose();assert.equal((await request).state,'cancelled');await turn();
});
test('repeated payloads have new identities while a replayed identity never applies twice',async t=>{
 const f=fixture();t.after(()=>f.close());await f.deliver(call);await f.deliver(call);assert.equal(f.applied.length,2);
 const original=f.events.find(([side,m])=>side==='client'&&m.type==='prepare')[1];f.page.receive(original);await turn();assert.equal(f.applied.length,2);
 const reused={...original,action:{...call,call:native(4)}};f.page.receive(reused);await turn();assert.equal(f.applied.length,2);
});
test('stale document and preparation-ticket replies cannot apply a target',async t=>{
 const gate=deferred(),started=deferred();const f=fixture({prepare:async()=>{started.resolve();await gate.promise;return {commit:()=>f.applied.push('target')};}});t.after(()=>f.close());const request=f.deliver(call);await started.promise;
 const original=f.events.find(([side,m])=>side==='client'&&m.type==='prepare')[1];
 f.page.receive({...original,type:'commit',ticket:id(9)});f.client.receive({...original,type:'prepared',document:id(8),ticket:id(9)});assert.deepEqual(f.applied,[]);
 f.page.manual();gate.resolve();assert.equal((await request).state,'cancelled');
});
test('fixed action shapes reject arbitrary paths, commands and malformed captured identities',()=>{
 const f=fixture();try{for(const action of [{...call,call:'../a'},{...call,path:'/etc/passwd'},{...call,command:'run'},{kind:'file',snapshot:native(1),file:native(2),version:native(3),line:0},{kind:'file',snapshot:native(1),file:native(2),version:native(3),line:1.5},{kind:'review',ticket:'\n'},{kind:'reveal',url:'http://evil'}])assert.throws(()=>f.deliver(action),/invalid/i);}finally{f.close();}
});
test('mode routing prepares in the new document and verifies source only after its preparation',async t=>{
 const pages=new Map(),order=[];let client;
 const make=(mode,document,transition=null)=>{
  const page=createDeliveryPage({mode,document,transition,ready:()=>true,prepare:async()=>{order.push('prepare '+mode);return {commit(){order.push('commit '+mode);}};},send:m=>queueMicrotask(()=>client.receive(m)),navigate:(path,ticket)=>{assert.equal(path,'/explore');page.depart();make('file',id(4),ticket).connect();}});pages.set(document,page);return page;
 };
 client=createDeliveryClient({send:m=>queueMicrotask(()=>pages.get(m.document)?.receive(m)),makeId:()=>id(1)});t.after(()=>{client.dispose();for(const page of pages.values())page.dispose();});
 make('call',id(2)).connect();const result=await client.deliver({kind:'file',snapshot:native(1),file:native(2),version:native(3),line:5},{validate:async()=>order.push('verify')});assert.equal(result.state,'applied');assert.deepEqual(order,['prepare file','verify','commit file']);
});
test('opening a fresh editor tab reveals the Source browser',async t=>{
 const pages=new Map(),opened=[];let client;
 const make=(mode,document,transition=null)=>{
  const page=createDeliveryPage({mode,document,transition,ready:()=>true,
   prepare:async()=>({commit(){opened.push(mode);}}),send:message=>queueMicrotask(()=>client.receive(message)),
   navigate:(path,ticket)=>{assert.equal(path,'/explore');page.depart();make('file',id(4),ticket).connect();}});
  pages.set(document,page);return page;
 };
 client=createDeliveryClient({send:message=>queueMicrotask(()=>pages.get(message.document)?.receive(message)),makeId:()=>id(1)});
 t.after(()=>{client.dispose();for(const page of pages.values())page.dispose();});
 make('landing',id(2)).connect();
 assert.equal((await client.deliver({kind:'reveal'})).state,'applied');
 assert.deepEqual(opened,['file']);
});
test('unexpected page replacement cancels a prepared action and does not replay it',async t=>{
 const gate=deferred(),started=deferred(),f=fixture({validate:async()=>{started.resolve();await gate.promise;}});t.after(()=>f.close());const request=f.deliver(call);await started.promise;
 f.client.receive({protocol:'vizzi-delivery',version:1,type:'connected',document:id(9),mode:'file',transition:null});assert.equal((await request).state,'cancelled');gate.resolve();await turn();assert.deepEqual(f.applied,[]);
});
test('missing acknowledgment after application is unconfirmed and commit is not retried',async t=>{
 let client,commits=0;client=createDeliveryClient({timeout:30,makeId:()=>id(1),send:m=>{if(m.type==='prepare')queueMicrotask(()=>client.receive({...m,type:'prepared',ticket:id(3)}));if(m.type==='commit')commits++;}});t.after(()=>client.dispose());
 client.receive({protocol:'vizzi-delivery',version:1,type:'connected',document:id(2),mode:'call',transition:null});await assert.rejects(client.deliver(call),/could not confirm/i);assert.equal(commits,1);
});

test('a newer delivery follows an already dispatched mode switch instead of preparing in the departing document',async t=>{
 const pages=new Map(),sent=[],applied=[],routed=deferred();let client,counter=0,arrival;
 const make=(mode,document,transition=null)=>{
  const page=createDeliveryPage({mode,document,transition,ready:()=>true,prepare:async action=>({commit(){applied.push(action);}}),send:m=>queueMicrotask(()=>client.receive(m)),navigate:(_path,ticket)=>{page.depart();arrival=()=>make('file',id(4),ticket).connect();routed.resolve();}});pages.set(document,page);return page;
 };
 client=createDeliveryClient({makeId:()=>id(++counter),send:m=>{sent.push(m);queueMicrotask(()=>pages.get(m.document)?.receive(m));}});t.after(()=>{client.dispose();for(const page of pages.values())page.dispose();});
 make('call',id(3)).connect();const first=client.deliver({kind:'file',snapshot:native(1),file:native(2),version:native(3),line:5});await routed.promise;
 const second=client.deliver({kind:'file',snapshot:native(1),file:native(4),version:native(3),line:9});await turn();
 const obsoletePrepares=sent.filter(m=>m.type==='prepare'&&m.id===id(2)&&m.document===id(3));arrival();
 assert.equal((await first).state,'cancelled');assert.equal((await second).state,'applied');assert.deepEqual(obsoletePrepares,[]);assert.deepEqual(applied.map(a=>a.file),[native(4)]);
});

test('manual navigation before a routed preparation resumes cancels that identity permanently',async t=>{
 const messages=[],applied=[];const document=id(3),transition=id(1);
 const page=createDeliveryPage({mode:'file',document,transition,ready:()=>true,prepare:async action=>({commit(){applied.push(action);}}),send:m=>messages.push(m),makeId:()=>id(4)});t.after(()=>page.dispose());
 page.manual();page.connect();
 const resumed={protocol:'vizzi-delivery',version:1,type:'prepare',id:transition,document,action:{kind:'file',snapshot:native(1),file:native(2),version:native(3),line:5},expires:Date.now()+60000};
 page.receive(resumed);await turn();for(const message of messages.filter(m=>m.type==='prepared'))page.receive({...message,type:'commit'});
 assert.deepEqual(applied,[]);assert.ok(messages.some(m=>m.type==='cancelled'&&m.id===transition));
});

test('manual cancellation of an arriving continuation also rejects the latest request carried by that transition',async t=>{
 const messages=[],applied=[];const document=id(3),transition=id(1),latest=id(2);
 const page=createDeliveryPage({mode:'file',document,transition,ready:()=>true,prepare:async action=>({commit(){applied.push(action);}}),send:m=>messages.push(m),makeId:()=>id(4)});t.after(()=>page.dispose());
 page.manual();page.connect();
 const action={kind:'file',snapshot:native(1),file:native(2),version:native(3),line:5};
 page.receive({protocol:'vizzi-delivery',version:1,type:'prepare',id:latest,document,transition,action,expires:Date.now()+60000});await turn();
 for(const message of messages.filter(m=>m.type==='prepared'))page.receive({...message,type:'commit'});
 assert.deepEqual(applied,[]);assert.ok(messages.some(m=>m.type==='cancelled'&&m.id===latest));
});

test('a newer different-mode action survives the first switch and routes again to its own destination',async t=>{
 const pages=new Map(),applied=[],routed=deferred();let client,nextDocument=3,counter=0,arrival,routes=0;
 const make=(mode,document,transition=null)=>{const page=createDeliveryPage({mode,document,transition,ready:()=>true,prepare:async action=>({commit(){applied.push(action);}}),send:m=>queueMicrotask(()=>client.receive(m)),navigate:(path,ticket)=>{page.depart();const target=path==='/workspace'?'call':'file';assert.equal(path,routes===0?'/explore':'/workspace');const connect=()=>make(target,id(++nextDocument),ticket).connect();if(++routes===1){arrival=connect;routed.resolve();}else connect();}});pages.set(document,page);return page;};
 client=createDeliveryClient({makeId:()=>id(++counter),send:m=>queueMicrotask(()=>pages.get(m.document)?.receive(m))});t.after(()=>{client.dispose();for(const page of pages.values())page.dispose();});
 make('call',id(3)).connect();const old=client.deliver({kind:'file',snapshot:native(1),file:native(2),version:native(3),line:1});await routed.promise;const latest=client.deliver(call);arrival();assert.equal((await old).state,'cancelled');assert.equal((await latest).state,'applied');assert.deepEqual(applied,[call]);assert.equal(routes,2);
});

test('manual navigation in the arriving page cancels the latest queued action before it can resume',async t=>{
 const pages=new Map(),applied=[],routed=deferred();let client,counter=0,arrival;
 const make=(mode,document,transition=null)=>{const page=createDeliveryPage({mode,document,transition,ready:()=>true,prepare:async action=>({commit(){applied.push(action);}}),send:m=>queueMicrotask(()=>client.receive(m)),navigate:(_path,ticket)=>{page.depart();arrival=()=>{const next=make('file',id(4),ticket);next.manual();next.connect();};routed.resolve();}});pages.set(document,page);return page;};
 client=createDeliveryClient({makeId:()=>id(++counter),send:m=>queueMicrotask(()=>pages.get(m.document)?.receive(m))});t.after(()=>{client.dispose();for(const page of pages.values())page.dispose();});
 make('call',id(3)).connect();const old=client.deliver({kind:'file',snapshot:native(1),file:native(2),version:native(3),line:1});await routed.promise;const waiting=client.deliver({kind:'file',snapshot:native(1),file:native(4),version:native(3),line:2});arrival();assert.equal((await old).state,'cancelled');assert.equal((await waiting).state,'cancelled');assert.deepEqual(applied,[]);
 const fresh={kind:'file',snapshot:native(1),file:native(5),version:native(3),line:3};assert.equal((await client.deliver(fresh)).state,'applied');assert.deepEqual(applied,[fresh]);
});

test('manual navigation in a departing page cancels its in-flight switch even after the old request was superseded',async t=>{
 let client,counter=0;const routed=deferred(),applied=[];
 const page=createDeliveryPage({mode:'call',document:id(3),ready:()=>true,prepare:async action=>({commit(){applied.push(action);}}),send:m=>queueMicrotask(()=>client.receive(m)),navigate:()=>routed.resolve()});
 client=createDeliveryClient({makeId:()=>id(++counter),send:m=>queueMicrotask(()=>page.receive(m))});t.after(()=>{client.dispose();page.dispose();});page.connect();
 const old=client.deliver({kind:'file',snapshot:native(1),file:native(2),version:native(3),line:1});await routed.promise;const waiting=client.deliver({kind:'file',snapshot:native(1),file:native(4),version:native(3),line:2});await turn();assert.equal(page.manual(),true);
 assert.equal((await old).state,'cancelled');assert.equal((await waiting).state,'cancelled');assert.deepEqual(applied,[]);assert.equal((await client.deliver(call)).state,'applied');assert.deepEqual(applied,[call]);
});

test('malformed continuation metadata cannot hold a page pending or prepare an action',async t=>{
 const applied=[],messages=[];const page=createDeliveryPage({mode:'file',document:id(2),transition:'not a ticket',ready:()=>true,prepare:async action=>({commit(){applied.push(action);}}),send:m=>messages.push(m)});t.after(()=>page.dispose());assert.equal(page.pending(),false);
 page.receive({protocol:'vizzi-delivery',version:1,type:'prepare',id:id(1),document:id(2),transition:'not a ticket',action:{kind:'file',snapshot:native(1),file:native(2),version:native(3),line:1},expires:Date.now()+60000});await turn();assert.deepEqual(applied,[]);assert.ok(messages.some(m=>m.type==='refused'));
});

test('the latest action carried by a mode transition replaces its route visit while a fresh action pushes a visit',async t=>{
 const pages=new Map(),history=[],routed=deferred();let client,counter=0,arrival;
 const make=(mode,document,transition=null)=>{const page=createDeliveryPage({mode,document,transition,ready:()=>true,prepare:async(_action,{replaceHistory})=>({commit(){history.push(replaceHistory);}}),send:m=>queueMicrotask(()=>client.receive(m)),navigate:(_path,ticket)=>{page.depart();arrival=()=>make('file',id(4),ticket).connect();routed.resolve();}});pages.set(document,page);return page;};
 client=createDeliveryClient({makeId:()=>id(++counter),send:m=>queueMicrotask(()=>pages.get(m.document)?.receive(m))});t.after(()=>{client.dispose();for(const page of pages.values())page.dispose();});
 make('call',id(3)).connect();const action={kind:'file',snapshot:native(1),file:native(2),version:native(3),line:1};const old=client.deliver(action);await routed.promise;const latest=client.deliver({...action,line:2});client.receive({protocol:'vizzi-delivery',version:1,type:'cancelled',id:id(1),transition:id(1),document:id(9),reason:'manual_navigation'});client.receive({protocol:'vizzi-delivery',version:1,type:'cancelled',id:id(1),transition:id(8),document:id(3),reason:'manual_navigation'});arrival();assert.equal((await old).state,'cancelled');assert.equal((await latest).state,'applied');assert.deepEqual(history,[true]);assert.equal((await client.deliver({...action,line:3})).state,'applied');assert.deepEqual(history,[true,false]);
});
test('an arriving page cancels the carried request immediately even when its preparation has not arrived',async t=>{
 const pages=new Map(),held=[],routed=deferred();let client,counter=0,arrival,destination;
 const make=(mode,document,transition=null)=>{const page=createDeliveryPage({mode,document,transition,ready:()=>true,prepare:async()=>({commit(){assert.fail('cancelled request applied');}}),send:m=>queueMicrotask(()=>client.receive(m)),navigate:(_path,ticket)=>{page.depart();arrival=()=>{destination=make('file',id(4),ticket);destination.connect();};routed.resolve();}});pages.set(document,page);return page;};
 client=createDeliveryClient({makeId:()=>id(++counter),send:m=>{if(m.type==='prepare'&&m.document===id(4)){held.push(m);return;}queueMicrotask(()=>pages.get(m.document)?.receive(m));}});t.after(()=>{client.dispose();for(const page of pages.values())page.dispose();});
 make('call',id(3)).connect();const action={kind:'file',snapshot:native(1),file:native(2),version:native(3),line:1};const old=client.deliver(action);await routed.promise;const latest=client.deliver({...action,line:2});arrival();await turn();assert.equal(held.length,1);destination.manual();assert.equal((await old).state,'cancelled');assert.equal((await latest).state,'cancelled');destination.receive(held[0]);await turn();
});
test('stale transition cancellations cannot cancel a newer prepared request in the destination document',async t=>{
 const pages=new Map(),routed=deferred(),checking=deferred(),gate=deferred(),applied=[];let client,counter=0,arrival;
 const make=(mode,document,transition=null)=>{const page=createDeliveryPage({mode,document,transition,ready:()=>true,prepare:async action=>({commit(){applied.push(action);}}),send:m=>queueMicrotask(()=>client.receive(m)),navigate:(_path,ticket)=>{page.depart();arrival=()=>make('file',id(4),ticket).connect();routed.resolve();}});pages.set(document,page);return page;};
 client=createDeliveryClient({makeId:()=>id(++counter),send:m=>queueMicrotask(()=>pages.get(m.document)?.receive(m))});t.after(()=>{client.dispose();for(const page of pages.values())page.dispose();});
 make('call',id(3)).connect();const action={kind:'file',snapshot:native(1),file:native(2),version:native(3),line:1};const old=client.deliver(action);await routed.promise;const latest=client.deliver({...action,line:2},{validate:async()=>{checking.resolve();await gate.promise;}});arrival();await checking.promise;
 client.receive({protocol:'vizzi-delivery',version:1,type:'cancelled',id:id(1),transition:id(1),document:id(9),reason:'manual_navigation'});
 client.receive({protocol:'vizzi-delivery',version:1,type:'cancelled',id:id(1),transition:id(8),document:id(4),reason:'manual_navigation'});
 gate.resolve();assert.equal((await old).state,'cancelled');assert.equal((await latest).state,'applied');assert.deepEqual(applied,[{...action,line:2}]);
});

test('an unexpected transition ticket or mode cancels a request waiting behind a dispatched switch',async()=>{
 for(const arrival of [{mode:'file',transition:id(9)},{mode:'review',transition:id(1)}]){
  const routed=deferred();let client,counter=0;const page=createDeliveryPage({mode:'call',document:id(3),ready:()=>true,prepare:async()=>{assert.fail('departing document prepared a request');},send:m=>queueMicrotask(()=>client.receive(m)),navigate:()=>routed.resolve()});
  client=createDeliveryClient({makeId:()=>id(++counter),send:m=>queueMicrotask(()=>page.receive(m))});
  try{
   page.connect();const action={kind:'file',snapshot:native(1),file:native(2),version:native(3),line:1};const old=client.deliver(action);await routed.promise;const latest=client.deliver({...action,line:2});client.receive({protocol:'vizzi-delivery',version:1,type:'connected',document:id(4),...arrival});assert.equal((await old).state,'cancelled');assert.equal((await latest).state,'cancelled');
  }finally{client.dispose();page.dispose();}
 }
});

test('standalone legacy submission replaces its existing URL visit and explicit same-page submission can push',async t=>{
 const visits=[];const page=createDeliveryPage({mode:'file',document:id(2),ready:()=>true,prepare:async(_action,{replaceHistory})=>({commit(){visits.push(replaceHistory);}}),send:()=>{}});t.after(()=>page.dispose());
 const action={kind:'file',snapshot:native(1),file:native(2),version:native(3),line:1};assert.equal((await page.submit(action)).state,'applied');assert.equal((await page.submit({...action,line:2},{replaceHistory:false})).state,'applied');assert.deepEqual(visits,[true,false]);
});
