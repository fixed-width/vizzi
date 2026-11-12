const vscode = require('vscode');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const {startBridge} = require('./bridge.cjs');
const {createProvider} = require('./providers.cjs');

let active;
let generation = 0;
async function stop() {
  generation++;
  const current = active;
  active = undefined;
  if (current) {current.provider.dispose(); await current.bridge.dispose();}
}

function activate(context) {
  const output = vscode.window.createOutputChannel('Vizzi');
  let starting = false;
  context.subscriptions.push(output,
    vscode.commands.registerCommand('vizzi.stopSharing',stop),
    vscode.commands.registerCommand('vizzi.shareLanguageProviders',async(folderUri)=>{
      if (starting) return;
      starting = true;
      let invocation = ++generation;
      let provider;
      try {
        if (!vscode.workspace.isTrusted) throw new Error('Trust this workspace in VS Code before sharing language providers');
        if (vscode.env.remoteName) throw new Error('Shared language providers currently require a local VS Code workspace');
        const folder = folderUri ? vscode.workspace.getWorkspaceFolder(folderUri)
          : await vscode.window.showWorkspaceFolderPick({placeHolder:'Select the workspace to share with Vizzi'});
        if (!folder) return;
        if (folder.uri.scheme !== 'file') throw new Error('Select a local file workspace');
        const root = await fs.realpath(folder.uri.fsPath);
        if (invocation !== generation) return;
        await stop();
        invocation = generation;
        const key = crypto.createHash('sha256').update(root).digest('hex');
        const connectionFile = path.join(context.globalStorageUri.fsPath,`connections/${key}.json`);
        if (!path.relative(root,connectionFile).startsWith(`..${path.sep}`)) throw new Error('Vizzi connection storage must be outside the inspected workspace');
        provider = createProvider(vscode,root,folder.uri.fsPath);
        const bridge = await startBridge({root,connectionFile,query:provider.query});
        if (invocation !== generation) {provider.dispose(); await bridge.dispose(); return;}
        active = {bridge,provider};
        output.clear();
        output.appendLine(`Sharing language providers for ${root}`);
        output.appendLine('Add these settings to your Vizzi TOML file, then launch vizzi --config FILE:');
        output.appendLine(`root = ${JSON.stringify(root)}\n\n[language-servers]\nvscode-connection = ${JSON.stringify(connectionFile)}`);
        output.appendLine('\nVS Code keeps ownership of its language servers. Save buffers before querying from Vizzi.');
        output.show(true);
        return {root,connectionFile};
      } catch (error) {
        if (provider && active?.provider !== provider) provider.dispose();
        await vscode.window.showErrorMessage(`Vizzi: ${error.message}`);
      } finally {starting = false;}
    }));
}

module.exports = {activate,deactivate:stop};
