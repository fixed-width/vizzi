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
  const doc = {uri,languageId:'python',version:1,isDirty:false,getText:()=>text, lineCount:4,
    lineAt:line=>({text:text.split('\n')[line]})};
  let execute = async()=>[{uri,range:{start:new Position(1,4)}}];
  const vscode = {Position,SymbolKind:{Method:5,Function:11},RelativePattern:class{},Uri:{file:()=>uri},
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

async function rustDependency(t,f) {
  const directory = await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-provider-rust-')));
  t.after(()=>fs.rm(directory,{recursive:true,force:true}));
  const file = path.join(directory,'option.rs');
  const text = 'pub const fn ok_or_else<E, F>(self, err: F) -> Result<T, E>\nwhere F: [const] FnOnce() -> E + [const] Destruct {\n    match self { Some(v) => Ok(v), None => Err(err()), }\n}\n';
  await fs.writeFile(file,text);
  const uri = {scheme:'file',fsPath:file,toString(){return this.fsPath;}};
  const doc = {uri,languageId:'rust',version:1,isDirty:false,getText:()=>text,lineCount:5,
    lineAt:line=>({text:text.split('\n')[line]})};
  f.vscode.workspace.openTextDocument = async value=>value.fsPath===file?doc:f.doc;
  f.vscode.Uri.file = value=>value===file?uri:f.uri;
  const declaration = {kind:5,name:'ok_or_else',range:{start:{line:0,character:0},end:{line:3,character:1}},
    selectionRange:{start:{line:0,character:13},end:{line:0,character:23}},children:[]};
  return {file,text,uri,doc,declaration};
}

test('external Rust definitions include saved source and exact editor function ranges',async t=>{
  const f = await fixture(t), dependency = await rustDependency(t,f);
  f.setExecute(async command=>command==='vscode.executeDocumentSymbolProvider'?[dependency.declaration]:[
    {uri:dependency.uri,range:{start:{line:0,character:13}}},
  ]);
  const answer = await f.provider.query(f.request);
  assert.equal(answer.outside,0);
  assert.equal(answer.locations[0].path,dependency.file);
  assert.equal(answer.locations[0].text,dependency.text);
  assert.equal(answer.locations[0].declaration.name,'ok_or_else');
  assert.deepEqual(answer.locations[0].declaration.range,dependency.declaration.range);
  dependency.doc.isDirty = true;
  await assert.rejects(f.provider.query(f.request),/Save/);
  dependency.doc.isDirty = false;
  dependency.doc.getText = ()=>dependency.text+'// divergent\n';
  await assert.rejects(f.provider.query(f.request),/saved source/);
});

test('dependency queries use their captured canonical Rust path and reject stale source',async t=>{
  const f = await fixture(t), dependency = await rustDependency(t,f);
  f.setExecute(async(command,uri)=>{
    assert.equal(command,'vscode.executeDefinitionProvider');
    assert.equal(uri.fsPath,dependency.file);
    return [{uri:f.uri,range:{start:{line:1,character:4}}}];
  });
  const request = {...f.request,method:'dependency-definition',path:dependency.file,text:dependency.text,
    position:{line:2,character:20}};
  assert.equal((await f.provider.query(request)).locations[0].path,f.uri.fsPath);
  f.setExecute(async(command,uri)=>{
    assert.equal(command,'vscode.executeReferenceProvider');
    assert.equal(uri.fsPath,dependency.file);
    return [{uri:f.uri,range:{start:{line:2,character:11}}}];
  });
  assert.equal((await f.provider.query({...request,method:'dependency-references'})).locations[0].path,f.uri.fsPath);
  await assert.rejects(f.provider.query({...request,text:'older bytes'}),/captured/);
  await assert.rejects(f.provider.query({...request,path:f.uri.fsPath}),/external Rust/);
});

test('dependency text changing while function ranges are queried is rejected',async t=>{
  const f = await fixture(t), dependency = await rustDependency(t,f);
  f.setExecute(async command=>{
    if (command==='vscode.executeDocumentSymbolProvider') {
      const changed = dependency.text+'// saved during lookup\n';
      await fs.writeFile(dependency.file,changed);
      dependency.doc.getText = ()=>changed;
      return [dependency.declaration];
    }
    return [{uri:dependency.uri,range:{start:{line:0,character:13}}}];
  });
  await assert.rejects(f.provider.query(f.request),/saved source/);
});

test('nonregular external source is rejected without waiting for a FIFO writer', {skip:process.platform==='win32'},async t=>{
  const {execFileSync} = require('node:child_process');
  const f = await fixture(t), dependency = await rustDependency(t,f);
  await fs.rm(dependency.file);
  execFileSync('mkfifo',[dependency.file]);
  f.setExecute(async()=>[{uri:dependency.uri,range:{start:{line:0,character:13}}}]);
  let release;
  const deadline = new Promise((_,reject)=>{
    release = setTimeout(()=>{
      reject(new Error('External source waited for a FIFO writer'));
      fs.open(dependency.file,'w').then(handle=>handle.close()).catch(()=>{});
    },2000);
  });
  try {await assert.rejects(Promise.race([f.provider.query(f.request),deadline]),/regular file/);}
  finally {clearTimeout(release);}
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
