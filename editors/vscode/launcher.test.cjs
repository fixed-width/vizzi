const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const {startWorkspace} = require('./launcher.cjs');

async function fixture(t) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-launch-'));
  const root = path.join(directory,'project with spaces');
  await fs.mkdir(root);
  t.after(()=>fs.rm(directory,{recursive:true,force:true}));
  const messages = [];
  const output = {appendLine:value=>messages.push(value),append:value=>messages.push(value)};
  return {directory,root,output,messages,connectionFile:path.join(directory,'connection.json')};
}

test('a project-local config launches the native workspace and shutdown closes its HTTP listener',async t=>{
  const f = await fixture(t);
  const binary = path.resolve(process.env.VIZZI_TEST_BINARY || 'target/release/vizzi');
  const configFile = path.join(f.root,'vizzi.toml');
  await fs.writeFile(path.join(f.root,'sample.py'),'def sample(): pass\n');
  await fs.writeFile(configFile,'root = "."\ncache = "../cache"\nport = 0\n');
  const workspace = await startWorkspace({...f,binary,configFile});
  t.after(()=>workspace.dispose());
  const response = await fetch(workspace.url+'api/status');
  assert.equal(response.status,200);
  assert.equal((await response.json()).name,path.basename(f.root));
  assert.match(f.messages.join('\n'),/Vizzi: http:\/\/127\.0\.0\.1:/);
  await workspace.dispose();
  await assert.rejects(fetch(workspace.url));
});

test('native configuration errors reach the user and a failed start leaves no running workspace',async t=>{
  const f = await fixture(t);
  const binary = path.resolve(process.env.VIZZI_TEST_BINARY || 'target/release/vizzi');
  const configFile = path.join(f.root,'vizzi.toml');
  await fs.writeFile(configFile,'unknown-option = true\n');
  await assert.rejects(startWorkspace({...f,binary,configFile}),/unknown.*option/i);
  assert.match(f.messages.join('\n'),/Cannot load config/);
});

test('a missing executable is a startup error',async t=>{
  const f = await fixture(t);
  await assert.rejects(startWorkspace({...f,binary:path.join(f.directory,'missing')}),/ENOENT/);
});

test('a startup timeout terminates the child process', {skip:process.platform==='win32'},async t=>{
  const f = await fixture(t);
  const binary = path.join(f.directory,'hang');
  const pidFile = path.join(f.directory,'pid');
  await fs.writeFile(binary,`#!${process.execPath}\nrequire('fs').writeFileSync(${JSON.stringify(pidFile)},String(process.pid));setInterval(()=>{},1000);\n`,{mode:0o755});
  await assert.rejects(startWorkspace({...f,binary,timeout:500}),/timed out/);
  const pid = Number(await fs.readFile(pidFile,'utf8'));
  assert.throws(()=>process.kill(pid,0),{code:'ESRCH'});
});

test('stopping during startup cancels and reaps the pending child', {skip:process.platform==='win32'},async t=>{
  const f = await fixture(t);
  const binary = path.join(f.directory,'pending');
  const pidFile = path.join(f.directory,'pid');
  await fs.writeFile(binary,`#!${process.execPath}\nrequire('fs').writeFileSync(${JSON.stringify(pidFile)},String(process.pid));setInterval(()=>{},1000);\n`,{mode:0o755});
  const controller = new AbortController();
  const pending = startWorkspace({...f,binary,signal:controller.signal});
  const rejected = assert.rejects(pending,/cancelled/);
  let pid;
  for (let n=0;n<100;n++) {
    pid = await fs.readFile(pidFile,'utf8').catch(()=>undefined);
    if (pid) break;
    await new Promise(resolve=>setTimeout(resolve,10));
  }
  assert.ok(pid,'child process started');
  controller.abort();
  await rejected;
  assert.throws(()=>process.kill(Number(pid),0),{code:'ESRCH'});
});
