const fs=require('node:fs/promises');
const path=require('node:path');
const {savedSource}=require('./providers.cjs');
const opaque=value=>typeof value==='string' && /^[0-9a-f]{64}$/.test(value);

async function unchanged(selection) {
  if(selection.document.isDirty || selection.document.getText()!==selection.text)throw new Error('Source changed while opening Vizzi. Save the file and retry Open in Source Browser.');
  if(!(await savedSource(selection.file)).equals(Buffer.from(selection.text,'utf8')))throw new Error('Editor text differs from saved source. Synchronise the file and retry Open in Source Browser.');
}
async function captureSourceFile(vscode,resource) {
  if(!vscode.workspace.isTrusted)throw new Error('Trust the workspace before opening source in Vizzi');
  if(vscode.env.remoteName)throw new Error('Open in Source Browser currently requires a local VS Code workspace');
  const uri=resource || vscode.window.activeTextEditor?.document.uri;
  if(uri?.scheme!=='file')throw new Error('Select a local project file to open in the Vizzi source browser');
  const folder=vscode.workspace.getWorkspaceFolder(uri);
  if(!folder)throw new Error('Select a source file in an open project folder');
  const document=await vscode.workspace.openTextDocument(uri);
  if(document.isDirty)throw new Error('Save the source file before using Vizzi: Open in Source Browser');
  const [root,file]=await Promise.all([fs.realpath(folder.uri.fsPath),fs.realpath(uri.fsPath)]);
  const relative=path.relative(root,file);
  if(!relative || path.isAbsolute(relative) || relative==='..' || relative.startsWith('..'+path.sep)
    || path.normalize(uri.fsPath)!==path.join(folder.uri.fsPath,relative))throw new Error('Open the project source path; source-file aliases are not captured by Vizzi');
  const editor=[vscode.window.activeTextEditor,...(vscode.window.visibleTextEditors || [])]
    .find(editor=>editor?.document.uri.toString()===uri.toString());
  const selection={uri,document,root,file,path:relative.split(path.sep).join('/'),text:document.getText(),line:(editor?.selection.active.line ?? 0)+1};
  await unchanged(selection);
  return selection;
}
async function resolveSourceFile(session,selection,{fetch=globalThis.fetch,timeout=30000,pause=()=>new Promise(resolve=>setTimeout(resolve,100))}={}) {
  if(session.root!==selection.root)throw new Error('The selected Vizzi project changed; retry Open in Source Browser');
  async function request(route,body) {
    const response=await fetch(new URL(route,session.url).href,{signal:AbortSignal.timeout(5000),
      ...(body===undefined?{}:{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})});
    if(!response.ok) {
      let message=await response.text();try{message=JSON.parse(message).message || message;}catch{}
      throw Object.assign(new Error(message.slice(0,512) || 'Vizzi could not open the selected file'),{status:response.status});
    }
    return response.json();
  }
  await request('api/workspace/start',{});
  const deadline=Date.now()+timeout;
  do {
    await unchanged(selection);
    const status=await request('api/status');
    if(status.error && !status.snapshot)throw new Error(status.error);
    if(opaque(status.snapshot) && !status.pending)try {
      const match=await request('api/investigation/lookup?'+new URLSearchParams({path:selection.path}));
      if(match.snapshot!==status.snapshot){await pause();continue;}
      if(!opaque(match.file) || !opaque(match.version) || match.path!==selection.path || !Number.isSafeInteger(match.lines) || match.lines<0)throw new Error('Vizzi returned an invalid source binding');
      const source=await request('api/source?'+new URLSearchParams({snapshot:match.snapshot,file:match.file}));
      if(source.schema!==2 || source.snapshot!==match.snapshot || source.file!==match.file || source.version!==match.version || source.path!==match.path)throw new Error('Vizzi returned a different captured source binding');
      if(source.text!==selection.text)throw new Error('The Vizzi capture differs from saved source. Retry Open in Source Browser after Source ready appears.');
      await unchanged(selection);
      return {snapshot:match.snapshot,file:match.file,version:match.version,line:Math.min(selection.line,Math.max(1,match.lines))};
    } catch(error) {if(![409,429,503].includes(error.status))throw error;}
    await pause();
  } while(Date.now()<deadline);
  throw new Error('Vizzi is still capturing source. Retry Open in Source Browser when Source ready appears.');
}

module.exports={captureSourceFile,resolveSourceFile};
