const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const os=require('node:os');
const path=require('node:path');
const crypto=require('node:crypto');
const {startOriginEditor}=require('./origin-connection.cjs');
const {startWorkspace}=require('./launcher.cjs');

test('native return navigation pins its launch, verifies saved bytes and never falls back',async t=>{
  const directory=await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-origin-native-')));
  t.after(()=>fs.rm(directory,{recursive:true,force:true}));
  const root=path.join(directory,'source');await fs.mkdir(root);
  const bytes=Buffer.from('pub fn entry() {}\n');const file=path.join(root,'lib.rs');await fs.writeFile(file,bytes);
  const configFile=path.join(root,'vizzi.toml');
  await fs.writeFile(configFile,'root="."\nport=0\neditor-code="/no/fallback/editor"\n');
  const received=[];let origin;
  const workspace=await startWorkspace({binary:path.resolve(process.env.VIZZI_TEST_BINARY||'target/release/vizzi'),root,configFile,
    connectionFile:path.join(directory,'unused-semantic.json'),output:{append(){},appendLine(){}},
    createOrigin:async()=>origin=await startOriginEditor({root,storage:path.join(directory,'origins'),name:'Fixture editor',open:async request=>{received.push(request);}})});
  t.after(()=>workspace.dispose());
  const get=async route=>{const response=await fetch(workspace.url+route);assert.equal(response.status,200,await response.clone().text());return response.json();};
  await fetch(workspace.url+'api/workspace/start',{method:'POST',headers:{'Content-Type':'application/json'},body:'{}'});
  const deadline=Date.now()+5000;
  while((await get('api/status')).pending){assert.ok(Date.now()<deadline);await new Promise(r=>setTimeout(r,10));}
  const target=await get('api/investigation/lookup?path=lib.rs');
  const open=()=>fetch(workspace.url+'api/editor/open',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({snapshot:target.snapshot,file:target.file,version:target.version,line:1})});
  assert.equal((await get('api/investigation')).editor,'Fixture editor');
  const descriptor=JSON.parse(await fs.readFile(origin.file,'utf8'));
  await fs.writeFile(origin.file,JSON.stringify({...descriptor,port:1,session:'0'.repeat(64)}));
  let response=await open();assert.equal(response.status,200,await response.clone().text());
  assert.equal(received.length,1);assert.equal(received[0].path,file);
  assert.deepEqual(received[0].source,{bytes:bytes.length,sha256:crypto.createHash('sha256').update(bytes).digest('hex')});
  assert.equal(received[0].session,descriptor.session);
  await fs.writeFile(file,'pub fn changed() {}\n');response=await open();assert.equal(response.status,409);assert.equal(received.length,1);
  await fs.writeFile(file,bytes);await origin.dispose();response=await open();assert.equal(response.status,502);
  assert.match(await response.text(),/originating editor/);assert.equal(received.length,1);
  await workspace.dispose();await assert.rejects(fetch(workspace.url));
});
