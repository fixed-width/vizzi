const {test}=require('node:test');const assert=require('node:assert/strict');const fs=require('node:fs/promises');const path=require('node:path');const os=require('node:os');const net=require('node:net');
const {startOriginEditor}=require('./origin-connection.cjs');
async function fixture(t,open=async()=>{}){const directory=await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-origin-')));t.after(()=>fs.rm(directory,{recursive:true,force:true}));const root=path.join(directory,'source');await fs.mkdir(root);const origin=await startOriginEditor({root,storage:path.join(directory,'storage'),name:'Fixture editor',open});t.after(()=>origin.dispose());const descriptor=JSON.parse(await fs.readFile(origin.file,'utf8'));return {directory,root,origin,descriptor};}
const request=d=>({protocol:'vizzi-origin',version:1,root:d.root,session:d.session,token:d.token,request:'9'.repeat(64),path:path.join(d.root,'a.rs'),line:1,source:{bytes:1,sha256:'a'.repeat(64)},expires_at_ms:Date.now()+8000});
async function call(d,body){return new Promise((resolve,reject)=>{const socket=net.createConnection(d.port,'127.0.0.1');let reply='';socket.once('error',reject);socket.on('data',data=>{reply+=data;if(reply.includes('\n')){resolve(JSON.parse(reply));socket.end();}});socket.once('connect',()=>socket.write(JSON.stringify(body)+'\n'));socket.setTimeout(2000,()=>{socket.destroy();reject(new Error('test request timeout'));});});}
test('two windows on one root have independent pinned sessions',async t=>{
  let a=0,b=0;const f=await fixture(t,async()=>{a++;});const other=await startOriginEditor({root:f.root,storage:path.join(f.directory,'storage'),name:'Other editor',open:async()=>{b++;}});t.after(()=>other.dispose());const d=JSON.parse(await fs.readFile(other.file,'utf8'));
  assert.notEqual(d.session,f.descriptor.session);assert.notEqual(d.token,f.descriptor.token);assert.equal((await call(f.descriptor,request(f.descriptor))).status,'opened');assert.equal(a,1);assert.equal(b,0);
  await f.origin.dispose();assert.equal((await call(d,request(d))).status,'opened');assert.equal(b,1);
});
test('wrong credentials, sessions, roots, commands and expired requests never open source',async t=>{
  let count=0;const f=await fixture(t,async()=>{count++;});
  for(const changed of [{token:'b'.repeat(64)},{session:'b'.repeat(64)},{root:'/other'},{command:'anything'},{expires_at_ms:0},{version:2}]){const response=await call(f.descriptor,{...request(f.descriptor),...changed});assert.equal(response.status,'error');}
  assert.equal(count,0);
});
test('disposal closes the owned endpoint and removes only its own descriptor',async t=>{
  const f=await fixture(t);await f.origin.dispose();await assert.rejects(fs.access(f.origin.file),{code:'ENOENT'});await assert.rejects(call(f.descriptor,request(f.descriptor)));
});
test('a stalled request cannot occupy the semantic listener or queue another navigation',async t=>{
  let release,notify;const gate=new Promise(r=>{release=r;}),started=new Promise(r=>{notify=r;});
  const f=await fixture(t,async(_request,{signal})=>{notify();await gate;if(signal.aborted)throw new Error('closed');});
  const first=call(f.descriptor,request(f.descriptor));await started;const second=await call(f.descriptor,{...request(f.descriptor),request:'8'.repeat(64)});assert.equal(second.code,'busy');release();assert.equal((await first).status,'opened');
});

test('expiry releases navigation while a slow preparation is still pending',async t=>{
  let release,started;const gate=new Promise(r=>{release=r;}),ready=new Promise(r=>{started=r;});let count=0;
  const f=await fixture(t,async(_request,{signal})=>{if(++count===1){started();await gate;assert.equal(signal.aborted,true);}});
  const first=call(f.descriptor,{...request(f.descriptor),expires_at_ms:Date.now()+100});await ready;
  const refused=await first;assert.equal(refused.status,'error');assert.equal(refused.code,'expired');
  assert.equal((await call(f.descriptor,request(f.descriptor))).status,'opened');release();
});

test('a replaced descriptor is retained and symbolic storage inside source is refused',async t=>{
  const f=await fixture(t);await fs.writeFile(f.origin.file,'replacement');await f.origin.dispose();assert.equal(await fs.readFile(f.origin.file,'utf8'),'replacement');
  const alias=path.join(f.directory,'alias-storage');await fs.symlink(f.root,alias,'dir');
  await assert.rejects(startOriginEditor({root:f.root,storage:alias,name:'Fixture',open:async()=>{}}),/outside captured source/);
});
test('invalid bounded locations are rejected before the editor runs',async t=>{
  let opened=0;const f=await fixture(t,async()=>{opened++;});
  for(const changed of [{path:'relative'},{line:0},{line:1.5},{source:{bytes:16*1024*1024+1,sha256:'a'.repeat(64)}},{source:{bytes:1,sha256:'bad'}},{source:{bytes:1,sha256:'a'.repeat(64),text:'forbidden'}}]){
    assert.equal((await call(f.descriptor,{...request(f.descriptor),...changed})).status,'error');
  }
  assert.equal(opened,0);
});

test('disconnect cancels pending preparation and releases the callback for the next request',async t=>{
  let release,started,signal;const ready=new Promise(r=>{started=r;}),gate=new Promise(r=>{release=r;});let count=0;
  const f=await fixture(t,async(_request,options)=>{if(++count===1){signal=options.signal;started();await gate;assert.equal(signal.aborted,true);}});
  const socket=net.createConnection(f.descriptor.port,'127.0.0.1');
  socket.on('error',()=>{});await new Promise(resolve=>socket.once('connect',resolve));socket.write(JSON.stringify(request(f.descriptor))+'\n');await ready;
  socket.destroy();const deadline=Date.now()+1000;
  while(!signal.aborted){assert.ok(Date.now()<deadline);await new Promise(r=>setTimeout(r,5));}
  assert.equal((await call(f.descriptor,request(f.descriptor))).status,'opened');release();
});
test('oversized and multiple frames cannot invoke the editor',async t=>{
  let count=0;const f=await fixture(t,async()=>{count++;});
  async function raw(frame){return new Promise((resolve,reject)=>{const socket=net.createConnection(f.descriptor.port,'127.0.0.1');let reply='';socket.on('error',reject);socket.on('data',bytes=>{reply+=bytes;});socket.once('close',()=>resolve(reply));socket.once('connect',()=>socket.write(frame));socket.setTimeout(1000,()=>{socket.destroy();reject(new Error('frame did not close'));});});}
  assert.equal(await raw('x'.repeat(64*1024+1)),'');
  const reply=JSON.parse(await raw(JSON.stringify(request(f.descriptor))+'\n{}\n'));assert.equal(reply.status,'error');
  assert.equal(count,0);
});
test('editor refusal messages stay within the UTF-8 reply allowance',async t=>{
  const f=await fixture(t,async()=>{throw new Error('界'.repeat(600));});
  const reply=await call(f.descriptor,request(f.descriptor));assert.equal(reply.status,'error');assert.ok(Buffer.byteLength(reply.message)<=512);assert.ok(reply.message.startsWith('界'));
});
