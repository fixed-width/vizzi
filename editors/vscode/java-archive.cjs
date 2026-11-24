const fs = require('node:fs/promises');
const path = require('node:path');
const ARCHIVE_BYTES = 256 * 1024 * 1024;

function isJavaArchive(uri) {
  return uri?.scheme === 'jdt' && uri.authority === 'contents' && !uri.fragment
    && Buffer.byteLength(uri.toString()) <= 16 * 1024;
}

async function javaAttachment(vscode, uri) {
  if (!isJavaArchive(uri)) return;
  const providerUri = uri.toString();
  try {
    const result = await vscode.commands.executeCommand('java.execute.workspaceCommand',
      'java.project.resolveSourceAttachment', JSON.stringify({classFileUri:providerUri}));
    if (!result?.attributes || result.errorMessage) return;
    const {sourceAttachmentPath,jarPath} = result.attributes;
    const local = value=>typeof value === 'string' && Buffer.byteLength(value) <= 4096 && path.isAbsolute(value);
    if (!local(sourceAttachmentPath) || !local(jarPath)
        || !['.jar','.zip'].includes(path.extname(sourceAttachmentPath)) || path.extname(jarPath) !== '.jar') return;
    const [archive, binary] = await Promise.all([fs.realpath(sourceAttachmentPath),fs.realpath(jarPath)]);
    if (!local(archive) || !local(binary)) return;
    const [sourceInfo,binaryInfo] = await Promise.all([fs.stat(archive),fs.stat(binary)]);
    if (!sourceInfo.isFile() || sourceInfo.size > ARCHIVE_BYTES || !binaryInfo.isFile()) return;
    return {path:archive,archiveBinary:binary,providerUri};
  } catch {return;}
}

async function verifyJavaDocument(vscode, document, witness, text) {
  const attachment = await javaAttachment(vscode, document.uri);
  if (!attachment || attachment.path !== witness.path || attachment.archiveBinary !== witness.archiveBinary
      || attachment.providerUri !== witness.providerUri) {
    throw new Error('Java source attachment changed or is unavailable; reopen it from its caller');
  }
  if (document.isDirty || document.languageId !== 'java' || document.getText() !== text) {
    throw new Error('Java archive document differs from captured source');
  }
}

module.exports = {isJavaArchive,javaAttachment,verifyJavaDocument};
