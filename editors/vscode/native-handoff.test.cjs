const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const os=require('node:os');
const path=require('node:path');
const {spawn}=require('node:child_process');
const readline=require('node:readline');
const {createHandoffClient}=require('./handoff.cjs');

test('real HTTP and finite stdio transports resolve the same saved source and call without VS Code',{timeout:15000},async t=>{
  const directory=await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-handoff-wire-')));
  t.after(()=>fs.rm(directory,{recursive:true,force:true}));
  const root=path.join(directory,'source');await fs.mkdir(root);
  const filename='a #%.py',text='def caller(): leaf()\ndef leaf(): pass\n';await fs.writeFile(path.join(root,filename),text);
  const binary=path.resolve(process.env.VIZZI_TEST_BINARY || 'target/release/vizzi');
  const children=[];
  t.after(async()=>{await Promise.all(children.map(async child=>{
    if(child.exitCode!==null)return;
    const closed=new Promise(r=>child.once('close',r)),timer=setTimeout(()=>child.kill('SIGKILL'),2000);
    child.kill('SIGINT');try{await closed;}finally{clearTimeout(timer);}
  }));});
  async function start(name,stdio) {
    const config=path.join(directory,name+'.toml');
    await fs.writeFile(config,`root = ${JSON.stringify(root)}\ncache = ${JSON.stringify(path.join(directory,name+'-cache'))}\nworkers = 1\nport = 0\n`);
    const child=spawn(binary,['--config',config,...(stdio?['--stdio']:[])],{stdio:['pipe','pipe','pipe']});children.push(child);
    let diagnostics='';child.stderr.on('data',data=>{diagnostics=(diagnostics+data).slice(-8192);});
    child.on('error',()=>{});
    if(stdio)return child;
    return new Promise((resolve,reject)=>{
      const timer=setTimeout(()=>reject(new Error('HTTP startup timed out: '+diagnostics)),10000);
      child.stderr.on('data',()=>{const match=diagnostics.match(/Vizzi: (http:\/\/127\.0\.0\.1:\d+)/);if(match){clearTimeout(timer);resolve({child,url:match[1]+'/'});}});
      child.once('error',error=>{clearTimeout(timer);reject(error);});
      child.once('close',code=>{clearTimeout(timer);reject(new Error(`Native exit ${code}: ${diagnostics}`));});
    });
  }
  const stdio=await start('stdio',true);let sequence=0;const pending=new Map();
  readline.createInterface({input:stdio.stdout}).on('line',line=>{const response=JSON.parse(line),entry=pending.get(response.id);if(entry){pending.delete(response.id);clearTimeout(entry.timer);entry.resolve(response);}});
  async function request(uri,body) {
    const id=++sequence,response=await new Promise((resolve,reject)=>{
      const timer=setTimeout(()=>{pending.delete(id);reject(new Error('Finite stdio request timed out: '+uri));},5000);
      pending.set(id,{resolve,reject,timer});stdio.stdin.write(JSON.stringify({id,uri,method:'POST',body})+'\n');
    });
    assert.equal(response.status,200,response.body);return JSON.parse(response.body);
  }
  const scope=(await request('/api/editor/handoff/window',{root})).scope;
  async function resolve(sequence,target) {
    let action={kind:'start',path:filename,text,target};
    for(;;) {
      const response=await request('/api/editor/handoff',{scope,sequence,action});
      if(response.state==='ready'){const {kind,...destination}=response.destination;return destination;}
      assert.equal(response.state,'pending',JSON.stringify(response));
      await new Promise(r=>setTimeout(r,response.retry_after_ms));action={kind:'continue',handle:response.handle};
    }
  }
  const http=await start('http',false),client=createHandoffClient();t.after(()=>client.dispose());
  const observation={root,path:filename,text};
  for(const [sequence,target] of [[1,{kind:'file',line:2}],[2,{kind:'call',position:{line:0,character:14}}]]) {
    const expected=await resolve(sequence,target);
    const actual=await client.resolve({root,url:http.url},observation,target,client.begin());
    assert.deepEqual(actual,expected);
  }
  assert.equal((await request('/api/editor/handoff/cancel',{scope,sequence:3})).state,'cancelled');
  assert.equal((await request('/api/editor/handoff',{scope,sequence:2,action:{kind:'start',path:filename,text,target:{kind:'file',line:1}}})).state,'superseded');
});
