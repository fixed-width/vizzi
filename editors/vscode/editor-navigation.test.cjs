const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const os=require('node:os');
const path=require('node:path');
const crypto=require('node:crypto');
const {createEditorNavigator}=require('./editor-navigation.cjs');
const uri=fsPath=>({scheme:'file',fsPath,toString(){return this.fsPath;}});
const witness=text=>({bytes:Buffer.byteLength(text),sha256:crypto.createHash('sha256').update(text).digest('hex')});
async function fixture(t,text='fn first() {}\nfn second() {}\n') {
  const directory=await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-return-')));t.after(()=>fs.rm(directory,{recursive:true,force:true}));
  const root=path.join(directory,'project');await fs.mkdir(root);const file=path.join(root,'a #%.rs');await fs.writeFile(file,text);
  const document={uri:uri(file),isDirty:false,getText:()=>text,lineCount:text.split('\n').length,save(){throw new Error('must not save');}};
  const shown=[],selections=[],revealed=[];let onOpen=async()=>{},onShow=async()=>{};
  const editor={document,viewColumn:1,set selection(value){selections.push(value);},revealRange:range=>revealed.push(range)};
  const vscode={Uri:{file:uri},Position:class{constructor(line,character){this.line=line;this.character=character;}},Selection:class{constructor(anchor,active){this.anchor=anchor;this.active=active;}},Range:class{constructor(start,end){this.start=start;this.end=end;}},TextEditorRevealType:{InCenterIfOutsideViewport:2},
    workspace:{textDocuments:[document],openTextDocument:async target=>{await onOpen(target);return document;}},
    window:{visibleTextEditors:[editor],showTextDocument:async(doc,options)=>{shown.push({doc,options});await onShow();return editor;}}};
  const controller=new AbortController();const request={path:file,line:2,source:witness(text),expires_at_ms:Date.now()+8000};
  return {directory,root,file,text,document,editor,vscode,shown,selections,revealed,controller,request,open:createEditorNavigator(vscode,{root}),onOpen:fn=>{onOpen=fn;},onShow:fn=>{onShow=fn;}};
}
test('clean matching target is shown without a selection and selected only after revalidation',async t=>{
  const f=await fixture(t);await f.open(f.request,{signal:f.controller.signal});
  assert.equal(f.shown.length,1);assert.equal(f.shown[0].options.selection,undefined);assert.equal(f.selections.length,1);assert.equal(f.selections[0].active.line,1);assert.equal(f.revealed.length,1);assert.equal(await fs.readFile(f.file,'utf8'),f.text);
});
test('dirty target is refused while unrelated dirty documents do not block navigation',async t=>{
  const f=await fixture(t);f.document.isDirty=true;await assert.rejects(f.open(f.request,{signal:f.controller.signal}),/unsaved|dirty/i);assert.equal(f.shown.length,0);
  f.document.isDirty=false;const other=path.join(f.root,'other.rs');await fs.writeFile(other,'other');f.vscode.workspace.textDocuments.push({uri:uri(other),isDirty:true,getText:()=> 'unsaved'});
  await f.open(f.request,{signal:f.controller.signal});assert.equal(f.selections.length,1);
});
test('saved or buffer content differing from the captured witness is refused',async t=>{
  const f=await fixture(t);await fs.writeFile(f.file,f.text+'changed');await assert.rejects(f.open(f.request,{signal:f.controller.signal}),/source.*changed|match.*captur/i);
  await fs.writeFile(f.file,f.text);f.document.getText=()=>f.text+'stale';await assert.rejects(f.open(f.request,{signal:f.controller.signal}),/buffer|source/i);assert.equal(f.shown.length,0);
});
for(const phase of ['open','show'])test(`edits during asynchronous ${phase} prevent the captured-line selection`,async t=>{
  const f=await fixture(t);f[phase==='open'?'onOpen':'onShow'](async()=>{f.document.isDirty=true;});
  await assert.rejects(f.open(f.request,{signal:f.controller.signal}),/unsaved|dirty/i);assert.equal(f.selections.length,0);assert.equal(f.revealed.length,0);
});
test('disconnect or expiry during presentation prevents late selection',async t=>{
  const f=await fixture(t);f.onShow(async()=>{f.controller.abort();});await assert.rejects(f.open(f.request,{signal:f.controller.signal}),/closed|cancel|unavailable/i);assert.equal(f.selections.length,0);
  const g=await fixture(t);let now=1;const open=createEditorNavigator(g.vscode,{root:g.root,now:()=>now});g.request.expires_at_ms=100;g.onShow(async()=>{now=100;});await assert.rejects(open(g.request,{signal:g.controller.signal}),/expired|deadline/i);assert.equal(g.selections.length,0);
});
test('workspace aliases cannot bypass a dirty target and are reused for clean buffers',async t=>{
  const f=await fixture(t),alias=path.join(f.directory,'alias');await fs.symlink(f.root,alias,'dir');f.document.uri=uri(path.join(alias,path.basename(f.file)));f.document.isDirty=true;
  const open=createEditorNavigator(f.vscode,{root:f.root,editorRoot:alias});await assert.rejects(open(f.request,{signal:f.controller.signal}),/unsaved|dirty/i);
  f.document.isDirty=false;await open(f.request,{signal:f.controller.signal});assert.equal(f.shown[0].doc.uri.fsPath,f.document.uri.fsPath);
});
test('repository files larger than the provider allowance and admitted external Rust files remain usable',async t=>{
  const f=await fixture(t,'//'+ 'x'.repeat(4*1024*1024)+'\n');f.request.line=1;await f.open(f.request,{signal:f.controller.signal});assert.equal(f.selections.length,1);
  const g=await fixture(t),external=path.join(g.directory,'library.rs');await fs.writeFile(external,g.text);g.document.uri=uri(external);g.request.path=external;await g.open(g.request,{signal:g.controller.signal});assert.equal(g.selections.length,1);
});
test('verified UTF-8 BOM bytes accept the editor text representation without rewriting source',async t=>{
  const f=await fixture(t,'\ufefffn first() {}\n');f.document.getText=()=>f.text.slice(1);f.request.line=1;await f.open(f.request,{signal:f.controller.signal});assert.equal(f.selections.length,1);assert.equal(await fs.readFile(f.file,'utf8'),f.text);
});
test('edits during saved-byte verification are checked again before selection',async t=>{
  const f=await fixture(t);let reads=0;
  const open=createEditorNavigator(f.vscode,{root:f.root,read:async file=>{const bytes=await fs.readFile(file);if(++reads===1)f.document.isDirty=true;return bytes;}});
  await assert.rejects(open(f.request,{signal:f.controller.signal}),/unsaved|dirty/i);assert.equal(f.selections.length,0);
});

test('saved edits during presentation and dirty duplicate aliases refuse the line',async t=>{
  const f=await fixture(t);f.onShow(()=>fs.writeFile(f.file,f.text+'saved change'));
  await assert.rejects(f.open(f.request,{signal:f.controller.signal}),/changed|captur/i);assert.equal(f.selections.length,0);
  const g=await fixture(t),alias=path.join(g.directory,'alias');await fs.symlink(g.root,alias,'dir');
  g.vscode.workspace.textDocuments.push({...g.document,uri:uri(path.join(alias,path.basename(g.file))),isDirty:true});
  await assert.rejects(g.open(g.request,{signal:g.controller.signal}),/unsaved/i);assert.equal(g.shown.length,0);
});
test('invalid captured destinations and oversized or nonregular source are refused',async t=>{
  const f=await fixture(t);
  for(const changed of [{line:0},{line:99},{path:'relative.rs'},{source:{...f.request.source,bytes:16*1024*1024+1}},{source:{...f.request.source,sha256:'bad'}}]){
    await assert.rejects(f.open({...f.request,...changed},{signal:f.controller.signal}));
  }
  assert.equal(f.selections.length,0);
  const alias=path.join(f.root,'alias.rs');await fs.symlink(f.file,alias);await assert.rejects(f.open({...f.request,path:alias}),/path changed/i);
});

test('a target alias opened during the final saved-byte read cannot bypass dirty checks',async t=>{
  const f=await fixture(t),alias=path.join(f.directory,'alias');await fs.symlink(f.root,alias,'dir');let reads=0;
  const open=createEditorNavigator(f.vscode,{root:f.root,read:async file=>{
    const bytes=await fs.readFile(file);
    if(++reads===2)f.vscode.workspace.textDocuments.push({...f.document,uri:uri(path.join(alias,path.basename(f.file))),isDirty:true});
    return bytes;
  }});
  await assert.rejects(open(f.request,{signal:f.controller.signal}),/changed|unsaved/i);assert.equal(f.selections.length,0);
});
