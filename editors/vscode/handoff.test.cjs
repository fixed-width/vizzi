const {test}=require('node:test');
const assert=require('node:assert/strict');
const {createHandoffClient}=require('./handoff.cjs');
const id=n=>String(n).repeat(64);
const session={root:'/source',url:'http://127.0.0.1:7474/'};
const source={root:'/source',path:'a #%.py',text:'value = 42\n'};
function fixture(overrides={}) {
  const requests=[];let scopes=0;
  const fetch=async(url,options)=>{
    const path=new URL(url).pathname,body=JSON.parse(options.body);
    if(path.endsWith('/window'))return Response.json({schema:1,scope:id(++scopes)});
    if(path.endsWith('/cancel'))return Response.json({schema:1,sequence:body.sequence,state:'cancelled'});
    const destination=body.action.target?.kind==='call'?{kind:'call',snapshot:id(4),call:id(5)}:{kind:'file',snapshot:id(4),file:id(5),version:id(6),line:1};
    return Response.json({schema:1,sequence:body.sequence,state:'ready',destination});
  };
  const client=createHandoffClient({fetch:(url,options)=>{requests.push({path:new URL(url).pathname,body:JSON.parse(options.body)});return overrides.fetch?overrides.fetch(url,options,fetch):fetch(url,options);},pause:async()=>{}});
  return {client,requests,scopes:()=>scopes};
}
test('both actions share one attached scope and only use the native handoff interface',async()=>{
  const f=fixture();
  assert.deepEqual(await f.client.resolve(session,source,{kind:'file',line:1},f.client.begin()),{snapshot:id(4),file:id(5),version:id(6),line:1});
  assert.deepEqual(await f.client.resolve(session,source,{kind:'call',position:{line:0,character:0}},f.client.begin()),{snapshot:id(4),call:id(5)});
  assert.equal(f.scopes(),1);assert.ok(f.requests.every(r=>r.path.startsWith('/api/editor/handoff')));
  assert.equal(f.requests.find(r=>r.body.action?.kind==='start').body.action.text,source.text);
});
test('newest intent discards a late ready result and cancels its predecessor silently',async()=>{
  let complete,started;
  const waiting=new Promise(r=>{started=r;});
  const f=fixture({fetch:(url,options,next)=>{
    const body=JSON.parse(options.body);
    if(new URL(url).pathname==='/api/editor/handoff' && body.sequence===1){started();return new Promise(r=>{complete=()=>r(Response.json({schema:1,sequence:1,state:'ready',destination:{kind:'file',snapshot:id(4),file:id(5),version:id(6),line:1}}));});}
    return next(url,options);
  }});
  const old=f.client.begin(),result=f.client.resolve(session,source,{kind:'file',line:1},old);
  await waiting;const newer=f.client.begin();
  const latest=await f.client.resolve(session,source,{kind:'file',line:1},newer);complete();
  await assert.rejects(result,{name:'AbortError'});assert.equal(latest.file,id(5));assert.equal(old.current(),false);
  assert.ok(f.requests.some(r=>r.path.endsWith('/cancel')&&r.body.sequence===1));
});
test('buffer changes before presentation cancel the native handoff and report the active refusal',async()=>{
  const f=fixture();let checks=0;
  await assert.rejects(f.client.resolve(session,source,{kind:'file',line:1},f.client.begin(),{check:async()=>{if(++checks===3)throw new Error('Source changed');}}),/Source changed/);
  assert.ok(f.requests.some(r=>r.path.endsWith('/cancel')));
});
test('continuations do not upload source again or reinterpret native capture state',async()=>{
  let first=true;
  const f=fixture({fetch:(url,options,next)=>{
    const body=JSON.parse(options.body);
    if(new URL(url).pathname==='/api/editor/handoff'&&first){first=false;return Response.json({schema:1,sequence:body.sequence,state:'pending',handle:id(8),retry_after_ms:1});}
    return next(url,options);
  }});
  await f.client.resolve(session,source,{kind:'file',line:1},f.client.begin());
  const steps=f.requests.filter(r=>r.path==='/api/editor/handoff');assert.equal(steps.length,2);
  assert.deepEqual(steps[1].body.action,{kind:'continue',handle:id(8)});
});
test('a new explicit intent can reattach an expired scope but a pending handoff never restarts',async()=>{
  let expire=true;
  const f=fixture({fetch:(url,options,next)=>{
    const body=JSON.parse(options.body);
    if(new URL(url).pathname==='/api/editor/handoff'&&expire){expire=false;return Response.json({schema:1,sequence:body.sequence,state:'refused',code:'scope_expired',message:'scope expired'});}
    return next(url,options);
  }});
  await f.client.resolve(session,source,{kind:'file',line:1},f.client.begin());assert.equal(f.scopes(),2);
  const expired=fixture({fetch:(url,options,next)=>new URL(url).pathname==='/api/editor/handoff'?Response.json({schema:1,sequence:JSON.parse(options.body).sequence,state:'expired'}):next(url,options)});
  await assert.rejects(expired.client.resolve(session,source,{kind:'file',line:1},expired.client.begin()),/expired/i);assert.equal(expired.scopes(),1);
});
test('wrong roots, invalid response identities and native refusals never produce a destination',async()=>{
  const f=fixture();await assert.rejects(f.client.resolve({...session,root:'/other'},source,{kind:'file',line:1},f.client.begin()),/workspace changed/i);assert.equal(f.requests.length,0);
  for(const reply of [{schema:2,sequence:1,state:'ready'},{schema:1,sequence:2,state:'ready'},{schema:1,sequence:1,state:'ready',destination:{kind:'file',snapshot:'path',file:id(5),version:id(6),line:1}},{schema:1,sequence:1,state:'refused',code:'not_captured',message:'File is excluded'}]) {
    const g=fixture({fetch:(url,options,next)=>new URL(url).pathname==='/api/editor/handoff'?Response.json(reply):next(url,options)});
    await assert.rejects(g.client.resolve(session,source,{kind:'file',line:1},g.client.begin()),/Invalid|excluded/);
  }
});

test('an older overridden executable reports how to update the matching handoff implementation',async()=>{
  const f=fixture({fetch:async()=>new Response('',{status:404})});
  await assert.rejects(f.client.resolve(session,source,{kind:'file',line:1},f.client.begin()),/Update the extension and executable together/);
});
