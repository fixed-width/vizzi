const fs = require('node:fs/promises');
const path = require('node:path');
const {savedSource} = require('./providers.cjs');
const opaque = value=>typeof value==='string' && /^[0-9a-f]{64}$/.test(value);

async function unchanged(selection) {
  if (selection.document.isDirty || selection.document.getText()!==selection.text) throw new Error('Source changed while opening Vizzi. Save the file and retry Explore Call.');
  if (!(await savedSource(selection.file)).equals(Buffer.from(selection.text,'utf8'))) throw new Error('Editor text differs from saved source. Synchronise the file and retry Explore Call.');
}
async function captureEditorCall(vscode,resource) {
  if (!vscode.workspace.isTrusted) throw new Error('Trust the workspace before exploring calls with Vizzi');
  if (vscode.env.remoteName) throw new Error('Explore Call currently requires a local VS Code workspace');
  const editor=vscode.window.activeTextEditor,document=editor?.document;
  if (!document || document.uri.scheme!=='file' || resource && resource.toString()!==document.uri.toString()) throw new Error('Place the cursor on a call in the source editor');
  if (document.isDirty) throw new Error('Save the source file before using Vizzi: Explore Call');
  const folder=vscode.workspace.getWorkspaceFolder(document.uri);
  if (!folder) throw new Error('Explore Call starts from a source file in an open project folder');
  const [root,file]=await Promise.all([fs.realpath(folder.uri.fsPath),fs.realpath(document.uri.fsPath)]);
  const relative=path.relative(root,file);
  if (!relative || path.isAbsolute(relative) || relative==='..' || relative.startsWith('..'+path.sep)
      || path.normalize(document.uri.fsPath)!==path.join(folder.uri.fsPath,relative)) {
    throw new Error('Open the project source path before exploring this call; source-file aliases are not captured');
  }
  const cursor=editor.selection.isEmpty?editor.selection.active:editor.selection.start;
  const position=document.getWordRangeAtPosition(cursor)?.start || cursor;
  const selection={document,uri:document.uri,root,file,path:relative.split(path.sep).join('/'),text:document.getText(),
    position:{line:position.line,character:position.character}};
  await unchanged(selection);
  return selection;
}
async function resolveEditorCall(session,selection,{fetch=globalThis.fetch,timeout=30000,pause=()=>new Promise(resolve=>setTimeout(resolve,100))}={}) {
  if (session.root!==selection.root) throw new Error('The selected Vizzi project changed; retry Explore Call');
  async function request(route,body) {
    const response=await fetch(session.url+route,body===undefined?{signal:AbortSignal.timeout(5000)}:
      {method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body),signal:AbortSignal.timeout(5000)});
    const data=await response.json().catch(()=>({message:'The native host does not support editor call navigation; update Vizzi'}));
    if (!response.ok) throw Object.assign(new Error(data.message || 'Vizzi could not open the selected call'),{status:response.status});
    return data;
  }
  await request('api/workspace/start',{});
  const deadline=Date.now()+timeout;
  do {
    if (selection.document.isDirty || selection.document.getText()!==selection.text) throw new Error('Source changed while opening Vizzi. Save the file and retry Explore Call.');
    const status=await request('api/status');
    if (status.error && !status.snapshot) throw new Error(status.error);
    if (opaque(status.snapshot) && !status.pending) {
      try {
        const data=await request('api/editor/call',{snapshot:status.snapshot,query:{kind:'position',path:selection.path,text:selection.text,position:selection.position}});
        if (data.snapshot!==status.snapshot || !opaque(data.invocation?.id) || !opaque(data.caller?.id) || data.invocation.owner!==data.caller.id) throw new Error('Vizzi returned a different editor call binding');
        await unchanged(selection);
        return {snapshot:data.snapshot,call:data.invocation.id};
      } catch (error) {if (![409,503].includes(error.status)) throw error;await unchanged(selection);}
    }
    await pause();
  } while (Date.now()<deadline);
  throw new Error('Vizzi is still capturing source. Retry Explore Call when Source ready appears.');
}
module.exports = {captureEditorCall,resolveEditorCall};
