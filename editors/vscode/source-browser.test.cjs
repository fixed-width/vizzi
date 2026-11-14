const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const os=require('node:os');
const path=require('node:path');
const {captureSourceFile,resolveSourceFile}=require('./source-browser.cjs');

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

function peer(f,{captured=f.text,lookup={snapshot:id(1),file:id(2),version:id(3),path:'a #%.py',lines:132},onSource=()=>{}}={}) {
  return async(url,options)=>{
    const request=new URL(url);
    if(request.pathname==='/api/workspace/start')return {ok:true,json:async()=>({})};
    if(request.pathname==='/api/status')return {ok:true,json:async()=>({snapshot:id(1),pending:false})};
    if(request.pathname==='/api/investigation/lookup'){
      assert.equal(request.searchParams.get('path'),'a #%.py');
      return {ok:true,json:async()=>lookup};
    }
    assert.equal(request.pathname,'/api/source');assert.equal(request.searchParams.get('snapshot'),id(1));
    assert.equal(request.searchParams.get('file'),id(2));onSource();
    return {ok:true,json:async()=>({schema:2,snapshot:id(1),file:id(2),version:id(3),path:'a #%.py',text:captured})};
  };
}

test('saved text must match pinned captured bytes before the opaque handoff is returned',async t=>{
  const f=await fixture(t),selection=await captureSourceFile(f.vscode);
  assert.deepEqual(await resolveSourceFile(f.session,selection,{fetch:peer(f)}),{snapshot:id(1),file:id(2),version:id(3),line:131});
  await assert.rejects(resolveSourceFile(f.session,selection,{fetch:peer(f,{captured:'older capture'})}),/captur.*saved|saved.*captur/i);
  await assert.rejects(resolveSourceFile({...f.session,root:'/other'},selection,{fetch:peer(f)}),/project changed/i);
});

test('source changes during capture and missing captured files refuse navigation',async t=>{
  const f=await fixture(t),selection=await captureSourceFile(f.vscode);
  await assert.rejects(resolveSourceFile(f.session,selection,{fetch:peer(f,{onSource:()=>{f.document.isDirty=true;}})}),/changed|Save/);
  f.document.isDirty=false;
  const fetch=async(url,options)=>new URL(url).pathname==='/api/investigation/lookup'
    ?{ok:false,status:422,text:async()=>JSON.stringify({message:'Path is not in the current source index'})}:peer(f)(url,options);
  await assert.rejects(resolveSourceFile(f.session,selection,{fetch}),/source index/);
});

test('captured source metadata cannot substitute another file/version; native read diagnostics are retained',async t=>{
  const f=await fixture(t),selection=await captureSourceFile(f.vscode),base=peer(f);
  for(const changed of [{schema:1},{snapshot:id(4)},{file:id(4)},{version:id(4)},{path:'other.py'}]) {
    const fetch=async(url,options)=>{
      const response=await base(url,options);
      return new URL(url).pathname==='/api/source'?{ok:true,json:async()=>({...await response.json(),...changed})}:response;
    };
    await assert.rejects(resolveSourceFile(f.session,selection,{fetch}),/different captured source binding/);
  }
  await assert.rejects(resolveSourceFile(f.session,selection,{fetch:async(url,options)=>new URL(url).pathname==='/api/source'?{ok:false,status:413,text:async()=> 'Source exceeds the bulk read allowance'}:base(url,options)}),/bulk read allowance/);
});
