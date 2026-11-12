const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const {createProvider} = require('./providers.cjs');

async function fixture(t) {
  const root = await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(), 'vizzi-providers-')));
  t.after(()=>fs.rm(root,{recursive:true,force:true}));
  const text = '# 😀\ndef leaf(): pass\ndef run(): leaf()\n';
  await fs.writeFile(path.join(root,'a.py'),text);
  const events = new Map();
  const event = name => callback => {events.set(name,callback);return {dispose(){}};};
  class Position { constructor(line,character){this.line=line;this.character=character;} }
  const uri = {scheme:'file',fsPath:path.join(root,'a.py'),toString(){return this.fsPath;}};
  const doc = {uri,version:1,isDirty:false,getText:()=>text, lineCount:4,
    lineAt:line=>({text:text.split('\n')[line]})};
  let execute = async()=>[{uri,range:{start:new Position(1,4)}}];
  const vscode = {Position,RelativePattern:class{},Uri:{file:()=>uri},
    workspace:{textDocuments:[doc],openTextDocument:async()=>doc,
      onDidChangeTextDocument:event('text'),onDidChangeConfiguration:event('config'),
      createFileSystemWatcher:()=>({onDidChange:event('fs'),onDidCreate:event('create'),onDidDelete:event('delete'),dispose(){}})},
    commands:{executeCommand:async(...args)=>execute(...args)}};
  const provider = createProvider(vscode,root);
  t.after(()=>provider.dispose());
  const request = {method:'definition',path:'a.py',text,position:{line:2,character:11}};
  return {root,text,events,doc,provider,request,setExecute:fn=>{execute=fn;},vscode,uri};
}

test('definitions and references query existing providers with UTF-16 positions and target text', async t => {
  const f = await fixture(t);
  f.setExecute(async(command,uri,position)=>{
    assert.equal(command,'vscode.executeDefinitionProvider');
    assert.equal(uri.fsPath,f.uri.fsPath);
    assert.deepEqual([position.line,position.character],[2,11]);
    return [{targetUri:uri,targetSelectionRange:{start:{line:1,character:4}}}];
  });
  const result = await f.provider.query(f.request);
  assert.equal(result.locations[0].text,f.text);
  assert.deepEqual(result.locations[0].position,{line:1,character:4});
  f.setExecute(async(command)=>{assert.equal(command,'vscode.executeReferenceProvider');return [];});
  assert.deepEqual((await f.provider.query({...f.request,method:'references'})).locations,[]);
});

test('dirty buffers, divergent query bytes, invalid paths and concurrent edits are rejected', async t => {
  const f = await fixture(t);
  f.doc.isDirty = true;
  await assert.rejects(f.provider.query(f.request), /Save/);
  f.doc.isDirty = false;
  await assert.rejects(f.provider.query({...f.request,text:'different'}), /captured/);
  await assert.rejects(f.provider.query({...f.request,path:'../other'}), /path/);
  for (const event of ['text','config','fs']) {
    f.setExecute(async()=>{f.events.get(event)({document:f.doc});return [];});
    await assert.rejects(f.provider.query(f.request), /changed/);
  }
});

test('external and non-file targets are reported outside and large results fail explicitly', async t => {
  const f = await fixture(t);
  f.setExecute(async()=>[{uri:{scheme:'virtual'},range:{start:{line:0,character:0}}},
    {uri:{scheme:'file',fsPath:path.join(f.root,'..','outside.py')},range:{start:{line:0,character:0}}}]);
  const answer = await f.provider.query(f.request);
  assert.equal(answer.outside,2);
  assert.deepEqual(answer.locations,[]);
  f.setExecute(async()=>Array(257).fill({}));
  await assert.rejects(f.provider.query(f.request), /allowance/);
});

test('a workspace opened through an alias uses its editor URI and returns canonical native paths', {skip:process.platform==='win32'}, async t => {
  const f = await fixture(t);
  const alias = path.join(f.root,'alias');
  await fs.symlink(f.root,alias,'dir');
  const editorFile = path.join(alias,'a.py');
  const uri = {scheme:'file',fsPath:editorFile,toString(){return editorFile;}};
  f.doc.uri = uri;
  f.vscode.Uri.file = file => {assert.equal(file,editorFile);return uri;};
  f.setExecute(async(_command,queryUri)=>{assert.equal(queryUri.fsPath,editorFile);return [{uri,range:{start:{line:1,character:4}}}];});
  const provider = createProvider(f.vscode,f.root,alias);
  t.after(()=>provider.dispose());
  const result = await provider.query(f.request);
  assert.equal(result.locations[0].path,path.join(f.root,'a.py'));
});

test('oversized saved source reports the resource limit before requesting analysis', async t => {
  const f = await fixture(t);
  await fs.writeFile(f.uri.fsPath,'x'.repeat(4*1024*1024+1));
  await assert.rejects(f.provider.query(f.request), /source exceeds allowance/);
});
