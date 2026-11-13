const fs = require('node:fs/promises');
const {constants} = require('node:fs');
const path = require('node:path');

const FILE_BYTES = 4 * 1024 * 1024;
async function savedSource(file) {
  const handle = await fs.open(file,constants.O_RDONLY | (constants.O_NONBLOCK || 0));
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
  async function verifyDocument(document,file,text=document.getText()) {
    if (document.isDirty) throw new Error('Save dependency and workspace buffers before using Vizzi editor analysis');
    if (!(await savedSource(file)).equals(Buffer.from(text,'utf8')) || document.getText() !== text) {
      throw new Error('Editor buffers differ from saved source; synchronise the editor before retrying');
    }
  }
  async function functionRange(document,position) {
    let symbols;
    try {symbols = await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider',document.uri);}
    catch {return;}
    if (!Array.isArray(symbols)) return;
    if (symbols.length>8192) return;
    const stack = [...symbols];
    let visited = 0;
    const before = (a,b)=>a.line<b.line || a.line===b.line && a.character<=b.character;
    while (stack.length && ++visited<=8192) {
      const symbol = stack.pop();
      if (!symbol) continue;
      if (Array.isArray(symbol.children)) {
        if (stack.length+symbol.children.length>8192) return;
        stack.push(...symbol.children);
      }
      if (![vscode.SymbolKind.Method,vscode.SymbolKind.Function].includes(symbol.kind)) continue;
      const range = symbol.range, selection = symbol.selectionRange;
      if (!range || !selection || ![range.start,range.end,selection.start,selection.end].every(p=>validPosition(document,p))) continue;
      if (selection.start.line!==selection.end.line || !before(range.start,selection.start)
          || !before(selection.end,range.end) || !before(selection.start,position)
          || !before(position,selection.end) || position.character===selection.end.character) continue;
      const name = document.lineAt(selection.start.line).text.slice(selection.start.character,selection.end.character);
      if (!name || Buffer.byteLength(name)>128) continue;
      const point = value=>({line:value.line,character:value.character});
      return {name,range:{start:point(range.start),end:point(range.end)},
        selection:{start:point(selection.start),end:point(selection.end)}};
    }
  }
  async function query(request) {
    if (disposed) throw new Error('Editor sharing stopped');
    if (!['definition','references','dependency-definition','dependency-references'].includes(request.method)) throw new Error('Unsupported editor operation');
    const dependency = request.method.startsWith('dependency-');
    const method = dependency?request.method.slice('dependency-'.length):request.method;
    if (typeof request.path !== 'string') throw new Error('Invalid editor source path');
    if (dependency) {
      if (!path.isAbsolute(request.path) || within(root,request.path) || path.extname(request.path)!=='.rs') {
        throw new Error('Dependency queries require a captured external Rust file');
      }
    } else if (request.path.includes('\\') || path.isAbsolute(request.path)
        || request.path.split('/').some(part=>!part || part === '.' || part === '..')) throw new Error('Invalid editor source path');
    if (typeof request.text !== 'string' || Buffer.byteLength(request.text) > FILE_BYTES) throw new Error('Editor source exceeds allowance');
    const file = dependency?request.path:path.join(root,request.path);
    if ((!dependency && !within(root,file)) || await fs.realpath(file) !== file) throw new Error('Editor source path escapes the workspace or uses a symlink');
    const before = epoch;
    await clean();
    const uri = vscode.Uri.file(dependency?file:path.join(editorRoot,request.path));
    const document = await vscode.workspace.openTextDocument(uri);
    await verifyDocument(document,file);
    if (document.getText() !== request.text) throw new Error('Editor query differs from captured source');
    if (!validPosition(document,request.position)) throw new Error('Invalid editor source position');
    const position = new vscode.Position(request.position.line,request.position.character);
    const command = method === 'definition' ? 'vscode.executeDefinitionProvider' : 'vscode.executeReferenceProvider';
    const raw = await vscode.commands.executeCommand(command,uri,position);
    if (!Array.isArray(raw)) throw new Error('No language provider answered the editor request');
    if (raw.length > 256) throw new Error('Editor result exceeds location allowance');
    const locations = [];
    const seen = new Set();
    let outside = 0;
    let bytes = 0;
    const witnesses = [{doc:document,file,text:request.text}];
    for (const location of raw) {
      const target = location.targetUri || location.uri;
      const start = (location.targetSelectionRange || location.targetRange || location.range)?.start;
      if (!target || !start) throw new Error('Invalid editor location');
      if (target.scheme !== 'file') {outside++; continue;}
      if (!within(root,target.fsPath) && !within(editorRoot,target.fsPath) && path.extname(target.fsPath)!=='.rs') {outside++;continue;}
      const canonical = await fs.realpath(target.fsPath);
      const external = !within(root,canonical);
      if (external && path.extname(canonical)!=='.rs') {outside++; continue;}
      if (!external) {
        const editorPath = path.join(editorRoot,path.relative(root,canonical));
        if (target.fsPath !== canonical && target.fsPath !== editorPath) throw new Error('Editor target path uses a symlink');
      }
      if (method === 'references' && canonical === file
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
      await verifyDocument(doc,canonical,text);
      const declaration = external && method==='definition' && doc.languageId==='rust' ? await functionRange(doc,start):undefined;
      locations.push({path:canonical,position:{line:start.line,character:start.character},text,...(declaration?{declaration}:{})});
      witnesses.push({doc,file:canonical,text});
    }
    await clean();
    for (const witness of witnesses) await verifyDocument(witness.doc,witness.file,witness.text);
    if (before !== epoch || disposed) throw new Error('Editor workspace or configuration changed during analysis; retry navigation');
    return {epoch,locations,outside};
  }
  return {query,dispose(){disposed=true;for(const subscription of subscriptions) subscription.dispose();}};
}

module.exports = {createProvider};
