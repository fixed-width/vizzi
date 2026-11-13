const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const net = require('node:net');
const {startBridge} = require('./bridge.cjs');

async function request(port, value) {
  return new Promise((resolve, reject) => {
    const socket = net.connect(port, '127.0.0.1');
    let text = '';
    socket.on('error', reject);
    socket.on('connect', () => socket.write(JSON.stringify(value) + '\n'));
    socket.on('data', data => { text += data; if (text.includes('\n')) { socket.destroy(); resolve(JSON.parse(text)); } });
    socket.on('end', () => { if (!text.includes('\n')) reject(new Error('connection closed')); });
  });
}

test('a local connection authenticates queries, preserves results and expires when stopped', async t => {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'vizzi-bridge-'));
  t.after(() => fs.rm(directory, {recursive:true, force:true}));
  const file = path.join(directory, 'connection.json');
  let calls = 0;
  const bridge = await startBridge({root:directory, connectionFile:file, query:async value => {
    calls++; return {epoch:3, locations:[{path:'target', text:value.text, position:value.position}], outside:0};
  }});
  t.after(() => bridge.dispose());
  const manifest = JSON.parse(await fs.readFile(file, 'utf8'));
  assert.equal(manifest.version, 1);
  assert.equal(manifest.token.length, 64);
  if (process.platform !== 'win32') assert.equal((await fs.stat(file)).mode & 0o777, 0o600);
  const query = {...manifest, method:'definition', path:'a.py', text:'😀', position:{line:0,character:2}};
  delete query.port;
  assert.equal((await request(manifest.port, {...query, token:'x'.repeat(64)})).status, 'error');
  assert.equal((await request(manifest.port, {...query, root:'/other'})).status, 'error');
  assert.equal((await request(manifest.port, {...query, version:2})).status, 'error');
  assert.equal(calls, 0);
  const result = await request(manifest.port, query);
  assert.equal(result.status, 'ok');
  assert.equal(result.locations[0].text, '😀');
  assert.equal(result.epoch, 3);
  assert.equal(calls, 1);
  for (const method of ['dependency-definition','dependency-references']) {
    assert.equal((await request(manifest.port,{...query,method,path:'/captured/dependency.rs'})).status,'ok');
  }
  await bridge.dispose();
  assert.equal(JSON.parse(await fs.readFile(file,'utf8')).port,manifest.port);
  await assert.rejects(request(manifest.port,query), {code:'ECONNREFUSED'});
});

test('replacing a connection does not let the previous bridge remove it', async t => {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'vizzi-bridge-'));
  t.after(() => fs.rm(directory, {recursive:true, force:true}));
  const file = path.join(directory, 'connection.json');
  const one = await startBridge({root:directory, connectionFile:file, query:async()=>({})});
  assert.equal(await one.isCurrent(),true);
  const two = await startBridge({root:directory, connectionFile:file, query:async()=>({})});
  assert.equal(await one.isCurrent(),false);
  assert.equal(await two.isCurrent(),true);
  t.after(() => two.dispose());
  await one.dispose();
  assert.equal(JSON.parse(await fs.readFile(file, 'utf8')).port, two.port);
});
