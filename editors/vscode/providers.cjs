const fs = require('node:fs/promises');
const path = require('node:path');

const FILE_BYTES = 4 * 1024 * 1024;
async function savedSource(file) {
  const handle = await fs.open(file,'r');
  try {
    const info = await handle.stat();
    if (!info.isFile() || info.size > FILE_BYTES) throw new Error('Editor source exceeds allowance or is not a regular file');
    const buffer = Buffer.alloc(info.size+1);
    let length = 0;
    while (length < buffer.length) {
      const {bytesRead} = await handle.read(buffer,length,buffer.length-length,null);
      if (!bytesRead) break;
      length += bytesRead;
    }
    if (length > info.size) throw new Error('Saved source changed during analysis; retry navigation');
    return buffer.subarray(0,length);
  } finally {await handle.close();}
}
function within(root, file) {
  const relative = path.relative(root,file);
  return relative !== '' && !relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative);
}
function validPosition(document, position) {
  if (!position || !Number.isInteger(position.line) || !Number.isInteger(position.character)
      || position.line < 0 || position.line >= document.lineCount || position.character < 0) return false;
  const line = document.lineAt(position.line).text;
  const at = position.character;
  return at <= line.length && !(at > 0 && /[\uD800-\uDBFF]/.test(line[at-1]) && /[\uDC00-\uDFFF]/.test(line[at] || ''));
}

function createProvider(vscode, root, editorRoot = root) {
  let epoch = 0;
  let disposed = false;
  const changed = () => {epoch++;};
  const watcher = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(editorRoot,'**/*'));
  const subscriptions = [watcher, watcher.onDidChange(changed), watcher.onDidCreate(changed), watcher.onDidDelete(changed),
    vscode.workspace.onDidChangeTextDocument(changed), vscode.workspace.onDidChangeConfiguration(changed)];
  async function clean() {
    let files = 0;
    let total = 0;
    for (const document of vscode.workspace.textDocuments) {
      if (document.uri.scheme !== 'file') continue;
      let file = document.uri.fsPath;
      if (!within(root,file)) {
        try {file = await fs.realpath(file);} catch {continue;}
      }
      if (!within(root,file)) continue;
      if (++files > 8192) throw new Error('Editor workspace exceeds file allowance');
      if (document.isDirty) throw new Error('Save workspace buffers before using Vizzi editor analysis');
      const bytes = await savedSource(file);
      total += bytes.length;
      if (total > 128*1024*1024) throw new Error('Editor workspace exceeds byte allowance');
      if (!bytes.equals(Buffer.from(document.getText(),'utf8'))) {
        throw new Error('Editor buffers differ from saved source; synchronise the editor before retrying');
      }
    }
  }
  async function query(request) {
    if (disposed) throw new Error('Editor sharing stopped');
    if (!['definition','references'].includes(request.method)) throw new Error('Unsupported editor operation');
    if (typeof request.path !== 'string' || request.path.includes('\\') || path.isAbsolute(request.path)
        || request.path.split('/').some(part=>!part || part === '.' || part === '..')) throw new Error('Invalid editor source path');
    if (typeof request.text !== 'string' || Buffer.byteLength(request.text) > FILE_BYTES) throw new Error('Editor source exceeds allowance');
    const file = path.join(root,request.path);
    if (!within(root,file) || await fs.realpath(file) !== file) throw new Error('Editor source path escapes the workspace or uses a symlink');
    const before = epoch;
    await clean();
    const uri = vscode.Uri.file(path.join(editorRoot,request.path));
    const document = await vscode.workspace.openTextDocument(uri);
    if (document.getText() !== request.text) throw new Error('Editor query differs from captured source');
    if (!validPosition(document,request.position)) throw new Error('Invalid editor source position');
    const position = new vscode.Position(request.position.line,request.position.character);
    const command = request.method === 'definition' ? 'vscode.executeDefinitionProvider' : 'vscode.executeReferenceProvider';
    const raw = await vscode.commands.executeCommand(command,uri,position);
    if (!Array.isArray(raw)) throw new Error('No language provider answered the editor request');
    if (raw.length > 256) throw new Error('Editor result exceeds location allowance');
    const locations = [];
    const seen = new Set();
    let outside = 0;
    let bytes = 0;
    for (const location of raw) {
      const target = location.targetUri || location.uri;
      const start = (location.targetSelectionRange || location.targetRange || location.range)?.start;
      if (!target || !start) throw new Error('Invalid editor location');
      if (target.scheme !== 'file' || (!within(root,target.fsPath) && !within(editorRoot,target.fsPath))) {outside++; continue;}
      const canonical = await fs.realpath(target.fsPath);
      if (!within(root,canonical)) {outside++; continue;}
      const editorPath = path.join(editorRoot,path.relative(root,canonical));
      if (target.fsPath !== canonical && target.fsPath !== editorPath) throw new Error('Editor target path uses a symlink');
      if (request.method === 'references' && canonical === file
          && start.line === position.line && start.character === position.character) continue;
      const key = `${canonical}:${start.line}:${start.character}`;
      if (seen.has(key)) continue;
      seen.add(key);
      const doc = await vscode.workspace.openTextDocument(target);
      if (!validPosition(doc,start)) throw new Error('Invalid editor target position');
      const text = doc.getText();
      const size = Buffer.byteLength(text);
      bytes += size;
      if (size > FILE_BYTES || bytes > 8 * 1024 * 1024) throw new Error('Editor target text exceeds allowance');
      locations.push({path:canonical,position:{line:start.line,character:start.character},text});
    }
    await clean();
    if (before !== epoch || disposed) throw new Error('Editor workspace or configuration changed during analysis; retry navigation');
    return {epoch,locations,outside};
  }
  return {query,dispose(){disposed=true;for(const subscription of subscriptions) subscription.dispose();}};
}

module.exports = {createProvider};
