const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const os=require('node:os');
const path=require('node:path');
const {captureSourceFile,captureEditorCall,verifySource}=require('./editor-source.cjs');

const id=n=>String(n).repeat(64);
const uri=fsPath=>({scheme:'file',fsPath,toString(){return this.fsPath;}});
async function fixture(t) {
  const root=await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-source-browser-')));
  t.after(()=>fs.rm(root,{recursive:true,force:true}));
  const file=path.join(root,'a #%.py'),text='\n'.repeat(130)+'value = 42\n',resource=uri(file);
  await fs.writeFile(file,text);
  const document={uri:resource,isDirty:false,getText:()=>text};
  const editor={document,selection:{active:{line:130,character:5},isEmpty:true}};
  const vscode={env:{},window:{activeTextEditor:editor,visibleTextEditors:[editor]},
    workspace:{isTrusted:true,getWorkspaceFolder:()=>({uri:uri(root)}),openTextDocument:async()=>document}};
  return {root,file,text,resource,document,editor,vscode,session:{root,url:'http://127.0.0.1:7474/'}};
}

test('source browsing captures a saved file and cursor without requiring a call',async t=>{
  const f=await fixture(t),selection=await captureSourceFile(f.vscode);
  assert.equal(selection.path,'a #%.py');assert.equal(selection.line,131);assert.equal(selection.text,f.text);
});

test('a tab resource wins over the active editor and opens at file start when not visible',async t=>{
  const f=await fixture(t),other={...f.document,uri:uri(path.join(f.root,'other.py'))};
  f.vscode.window.activeTextEditor={document:other,selection:{active:{line:25}}};
  f.vscode.window.visibleTextEditors=[];
  assert.equal((await captureSourceFile(f.vscode,f.resource)).line,1);
  f.vscode.window.visibleTextEditors=[f.editor];
  assert.equal((await captureSourceFile(f.vscode,f.resource)).line,131);
});

test('dirty files, divergent disk bytes and non-file/outside resources fail before launch',async t=>{
  const f=await fixture(t);f.document.isDirty=true;
  await assert.rejects(captureSourceFile(f.vscode,f.resource),/Save/);
  f.document.isDirty=false;await fs.writeFile(f.file,'changed');
  await assert.rejects(captureSourceFile(f.vscode,f.resource),/saved source/i);
  await assert.rejects(captureSourceFile(f.vscode,{scheme:'untitled'}),/local.*file/i);
  f.vscode.workspace.getWorkspaceFolder=()=>undefined;
  await assert.rejects(captureSourceFile(f.vscode,f.resource),/project folder/i);
});

test('workspace root aliases are supported while source-file aliases remain outside capture',async t=>{
  const f=await fixture(t),alias=f.root+'-alias';
  await fs.symlink(f.root,alias);t.after(()=>fs.rm(alias,{force:true}));
  f.vscode.workspace.getWorkspaceFolder=()=>({uri:uri(alias)});f.document.uri=uri(path.join(alias,path.basename(f.file)));
  assert.equal((await captureSourceFile(f.vscode,f.document.uri)).root,f.root);
  const linked=path.join(f.root,'linked.py');await fs.symlink(f.file,linked);f.document.uri=uri(linked);
  await assert.rejects(captureSourceFile(f.vscode,f.document.uri),/alias/i);
});

test('call observations retain the original word point and require the active resource',async t=>{
  const f=await fixture(t);f.document.getWordRangeAtPosition=()=>({start:{line:130,character:0}});
  const selected=await captureEditorCall(f.vscode,f.resource);
  assert.deepEqual(selected.position,{line:130,character:0});
  await assert.rejects(captureEditorCall(f.vscode,uri(path.join(f.root,'other.py'))),/source editor/);
});
test('buffer or saved-byte changes invalidate the original editor observation',async t=>{
  const f=await fixture(t),selected=await captureSourceFile(f.vscode,f.resource);
  f.document.isDirty=true;await assert.rejects(verifySource(selected),/Source changed/);
  f.document.isDirty=false;f.document.getText=()=>f.text+'changed';await assert.rejects(verifySource(selected),/Source changed/);
  f.document.getText=()=>f.text;await fs.writeFile(f.file,'changed');await assert.rejects(verifySource(selected),/saved source/);
});

test('buffer edits while the saved bytes are being read invalidate presentation',async t=>{
  const f=await fixture(t),selected=await captureSourceFile(f.vscode,f.resource),originalOpen=fs.open;
  let release,notify;const reading=new Promise(resolve=>{notify=resolve;}),gate=new Promise(resolve=>{release=resolve;});
  fs.open=async(...args)=>{
    const handle=await originalOpen(...args);if(args[0]!==f.file)return handle;
    return {stat:()=>handle.stat(),close:()=>handle.close(),read:async(...args)=>{notify();await gate;return handle.read(...args);}};
  };
  t.after(()=>{fs.open=originalOpen;release();});
  const checked=verifySource(selected);await reading;f.document.isDirty=true;release();
  await assert.rejects(checked,/Source changed/);
});
