const fs=require('node:fs/promises');
const {constants}=require('node:fs');
const path=require('node:path');

async function savedSource(file) {
  const handle=await fs.open(file,constants.O_RDONLY | (constants.O_NONBLOCK || 0));
  try {
    const info=await handle.stat();
    if(!info.isFile() || info.size>4*1024*1024)throw new Error('Editor source exceeds allowance or is not a regular file');
    const buffer=Buffer.alloc(info.size+1);let length=0;
    while(length<buffer.length){const {bytesRead}=await handle.read(buffer,length,buffer.length-length,null);if(!bytesRead)break;length+=bytesRead;}
    if(length>info.size)throw new Error('Saved source changed during analysis; retry navigation');
    return buffer.subarray(0,length);
  } finally{await handle.close();}
}
function verifyBuffer(selection) {
  if(selection.document.isDirty || selection.document.getText()!==selection.text)throw new Error('Source changed while opening Vizzi. Save the file and retry the editor action.');
}
async function verifySource(selection,{saved=true}={}) {
  verifyBuffer(selection);
  if(saved && !(await savedSource(selection.file)).equals(Buffer.from(selection.text,'utf8')))throw new Error('Editor text differs from saved source. Synchronise the file and retry the editor action.');
  verifyBuffer(selection);
}
async function capture(vscode,resource,call) {
  if(!vscode.workspace.isTrusted)throw new Error('Trust the workspace before opening source in Vizzi');
  if(vscode.env.remoteName)throw new Error('Editor handoffs currently require a local VS Code workspace');
  const active=vscode.window.activeTextEditor;
  const uri=resource || active?.document.uri;
  if(call && (!active || resource && resource.toString()!==active.document.uri.toString()))throw new Error('Place the cursor on a call in the source editor');
  if(uri?.scheme!=='file')throw new Error('Select a local project file to open in Vizzi');
  const folder=vscode.workspace.getWorkspaceFolder(uri);
  if(!folder)throw new Error('Select a source file in an open project folder');
  const document=call?active.document:await vscode.workspace.openTextDocument(uri);
  if(document.isDirty)throw new Error('Save the source file before using the Vizzi editor action');
  const [root,file]=await Promise.all([fs.realpath(folder.uri.fsPath),fs.realpath(uri.fsPath)]);
  const relative=path.relative(root,file);
  if(!relative || path.isAbsolute(relative) || relative==='..' || relative.startsWith('..'+path.sep)
    || path.normalize(uri.fsPath)!==path.join(folder.uri.fsPath,relative))throw new Error('Open the project source path; source-file aliases are not captured by Vizzi');
  const editor=call?active:[active,...(vscode.window.visibleTextEditors || [])].find(editor=>editor?.document.uri.toString()===uri.toString());
  const cursor=call&&!editor.selection.isEmpty?editor.selection.start:editor?.selection.active;
  const point=call?(document.getWordRangeAtPosition(cursor)?.start || cursor):cursor;
  const selection={uri,document,root,file,path:relative.split(path.sep).join('/'),text:document.getText(),
    position:point?{line:point.line,character:point.character}:undefined,line:(point?.line ?? 0)+1};
  await verifySource(selection);return selection;
}
module.exports={savedSource,verifySource,captureEditorCall:(vscode,resource)=>capture(vscode,resource,true),captureSourceFile:(vscode,resource)=>capture(vscode,resource,false)};
