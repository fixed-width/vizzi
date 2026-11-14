const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const {savedSource} = require('./editor-source.cjs');

const failure = (code,message)=>Object.assign(new Error(message),{code});
function contains(root,file) {
  const relative = path.relative(root,file);
  return relative === '' || relative !== '..' && !relative.startsWith('..'+path.sep) && !path.isAbsolute(relative);
}

function createEditorNavigator(vscode,{root,editorRoot=root,read=savedSource,now=Date.now}) {
  return async function open(request,{signal}={}) {
    function alive() {
      if (signal?.aborted) throw failure('origin_unavailable','Originating editor request was cancelled or its connection closed');
      if (now() >= request.expires_at_ms) throw failure('expired','Originating editor navigation deadline expired');
    }
    alive();
    if (typeof request.path !== 'string' || !path.isAbsolute(request.path)
        || !Number.isSafeInteger(request.line) || request.line < 1
        || !Number.isSafeInteger(request.source?.bytes) || request.source.bytes < 0
        || request.source.bytes > 16*1024*1024 || !/^[0-9a-f]{64}$/.test(request.source.sha256)) {
      throw failure('invalid_request','Invalid verified editor location');
    }
    const canonical = await fs.realpath(request.path).catch(()=>{
      throw failure('target_unavailable','The verified target file is unavailable');
    });
    alive();
    if (canonical !== request.path) throw failure('source_changed','The target source path changed; reopen it in Vizzi');
    const fileDocuments = ()=>vscode.workspace.textDocuments.filter(doc=>doc.uri.scheme === 'file');
    function unchanged(documents) {
      const current = fileDocuments();
      if (current.length !== documents.length || current.some(doc=>!documents.includes(doc))) {
        throw failure('source_changed','Open editor documents changed during navigation; retry');
      }
    }
    async function matching() {
      const documents = fileDocuments();
      if (documents.length > 8192) throw failure('target_unavailable','Editor document allowance exceeded');
      const found = await Promise.all(documents.map(async doc=>
        (await fs.realpath(doc.uri.fsPath).catch(()=>undefined)) === canonical ? doc : undefined));
      alive();
      unchanged(documents);
      return {matches:found.filter(Boolean),documents};
    }
    function clean(documents) {
      for (const doc of documents) {
        if (doc.isDirty) throw failure('dirty_target','The target buffer has unsaved changes; save it before returning from Vizzi');
      }
    }
    const {matches} = await matching();
    clean(matches);
    let target = matches[0]?.uri;
    if (!target) {
      const useWorkspaceSpelling = contains(root,canonical) && await fs.realpath(editorRoot).catch(()=>undefined) === root;
      alive();
      target = vscode.Uri.file(useWorkspaceSpelling ? path.join(editorRoot,path.relative(root,canonical)) : canonical);
    }
    const document = await vscode.workspace.openTextDocument(target);
    alive();
    async function verify() {
      alive();
      const {matches,documents} = await matching();
      clean([...matches,document]);
      if (await fs.realpath(document.uri.fsPath).catch(()=>undefined) !== canonical) {
        throw failure('source_changed','The editor target path changed');
      }
      alive();
      const bytes = await read(canonical,16*1024*1024);
      alive();
      unchanged(documents);
      clean([...matches,document]);
      if (bytes.length !== request.source.bytes || crypto.createHash('sha256').update(bytes).digest('hex') !== request.source.sha256) {
        throw failure('source_changed','Saved source changed or does not match the captured source');
      }
      const text = bytes.toString('utf8');
      if (!Buffer.from(text,'utf8').equals(bytes)) throw failure('source_changed','The target is not captured UTF-8 source');
      const represented = text.startsWith('\ufeff') ? text.slice(1) : text;
      for (const doc of new Set([...matches,document])) {
        if (doc.getText() !== represented && doc.getText() !== text) {
          throw failure('source_changed','The target editor buffer differs from captured source');
        }
      }
      if (request.line > document.lineCount) throw failure('source_changed','The captured line is outside the target editor buffer');
    }
    await verify();
    const visible = vscode.window.visibleTextEditors?.find(editor=>editor.document === document);
    const editor = await vscode.window.showTextDocument(document,{preview:false,preserveFocus:false,
      ...(visible?.viewColumn ? {viewColumn:visible.viewColumn} : {})});
    alive();
    if (editor.document !== document) throw failure('source_changed','The editor displayed a different target document');
    await verify();
    alive();
    const point = new vscode.Position(request.line-1,0);
    editor.selection = new vscode.Selection(point,point);
    editor.revealRange(new vscode.Range(point,point),vscode.TextEditorRevealType.InCenterIfOutsideViewport);
  };
}
module.exports = {createEditorNavigator};
