const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const path=require('node:path');
const os=require('node:os');
const {captureEditorCall,resolveEditorCall}=require('./editor-call.cjs');

async function fixture(t) {
 const root=await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-editor-call-')));
 t.after(()=>fs.rm(root,{recursive:true,force:true}));
 const file=path.join(root,'a.py'),text='def caller(): leaf()\n';await fs.writeFile(file,text);
 const uri={scheme:'file',fsPath:file,toString(){return this.fsPath;}};
 const document={uri,isDirty:false,getText:()=>text,getWordRangeAtPosition:()=>({start:{line:0,character:14}})};
 const vscode={env:{},workspace:{isTrusted:true,getWorkspaceFolder:()=>({uri:{fsPath:root}})},
  window:{activeTextEditor:{document,selection:{isEmpty:true,active:{line:0,character:16}}}}};
 const requests=[];
 let reply={snapshot:'a'.repeat(64),caller:{id:'c'.repeat(64)},invocation:{id:'b'.repeat(64),owner:'c'.repeat(64)}};
 const fetch=async(url,options)=>{
  const route=new URL(url).pathname;
  if(route==='/api/workspace/start')return Response.json({},{status:202});
  if(route==='/api/status')return Response.json({snapshot:'a'.repeat(64),pending:false});
  requests.push(JSON.parse(options.body));return Response.json(reply);
 };
 return {root,file,text,uri,document,vscode,requests,fetch,setReply:value=>{reply=value;},session:{root,url:'http://127.0.0.1:7474/'}};
}
test('saved editor text and the word position are bound to the native current snapshot',async t=>{
 const f=await fixture(t),selection=await captureEditorCall(f.vscode,f.uri);
 const result=await resolveEditorCall(f.session,selection,{fetch:f.fetch});
 assert.deepEqual(result,{snapshot:'a'.repeat(64),call:'b'.repeat(64)});
 assert.deepEqual(f.requests[0].query,{kind:'position',path:'a.py',text:f.text,position:{line:0,character:14}});
});
test('dirty buffers, unrelated menu resources and changed saved bytes cannot navigate',async t=>{
 const f=await fixture(t);
 f.document.isDirty=true;await assert.rejects(captureEditorCall(f.vscode),/Save/);
 f.document.isDirty=false;await assert.rejects(captureEditorCall(f.vscode,{toString:()=>'/another/file'}),/source editor/);
 const selection=await captureEditorCall(f.vscode,f.uri);
 await fs.writeFile(f.file,'def changed(): pass\n');
 await assert.rejects(resolveEditorCall(f.session,selection,{fetch:f.fetch}),/saved source/);
});
test('a workspace switch or incorrect returned binding cannot open another call',async t=>{
 const f=await fixture(t),selection=await captureEditorCall(f.vscode);
 await assert.rejects(resolveEditorCall({...f.session,root:'/elsewhere'},selection,{fetch:f.fetch}),/project changed/);
 f.setReply({snapshot:'a'.repeat(64),caller:{id:'c'.repeat(64)},invocation:{id:'b'.repeat(64),owner:'d'.repeat(64)}});
 await assert.rejects(resolveEditorCall(f.session,selection,{fetch:f.fetch}),/different editor call binding/);
});
test('an older source publication is retried without changing editor or project settings',async t=>{
 const f=await fixture(t),selection=await captureEditorCall(f.vscode);
 let retry=true;
 const fetch=async(url,options)=>{
  if(new URL(url).pathname==='/api/editor/call'&&retry){retry=false;return Response.json({message:'capture pending'},{status:409});}
  return f.fetch(url,options);
 };
 const result=await resolveEditorCall(f.session,selection,{fetch,pause:async()=>{}});
 assert.equal(result.call,'b'.repeat(64));
 assert.equal(await fs.readFile(f.file,'utf8'),f.text);
});
