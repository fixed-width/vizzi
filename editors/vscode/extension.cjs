const vscode = require('vscode');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const {startBridge} = require('./bridge.cjs');
const {createProvider} = require('./providers.cjs');
const {startWorkspace} = require('./launcher.cjs');
const {createWorkspaceView} = require('./workspace-view.cjs');

let active;
let workspace;
let workspaceView;
let launchAbort;
let launchTask;
let generation = 0;
async function stop() {
  generation++;
  const current = active;
  active = undefined;
  if (current) {current.provider.dispose(); await current.bridge.dispose();}
}

async function stopWorkspace() {
  workspaceView?.dispose();
  launchAbort?.abort();
  if (launchTask) await launchTask.catch(()=>{});
  const current = workspace;
  workspace = undefined;
  if (current) await current.dispose();
}

async function deactivate() {
  generation++;
  await stopWorkspace();
  await stop();
}

function activate(context) {
  const output = vscode.window.createOutputChannel('Vizzi');
  workspaceView = createWorkspaceView(vscode);
  const status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left);
  status.text = '$(preview) Vizzi';
  status.tooltip = 'Open the Vizzi visual workspace';
  status.command = 'vizzi.openWorkspace';
  status.show();
  let starting = false;
  let opening;
  async function selectFolder(folderUri) {
    if (!vscode.workspace.isTrusted) throw new Error('Trust this workspace in VS Code before sharing language providers');
    if (vscode.env.remoteName) throw new Error('Shared language providers currently require a local VS Code workspace');
    const folders = vscode.workspace.workspaceFolders || [];
    if (!folders.length) throw new Error('Open a project folder with File → Open Folder before starting Vizzi');
    const folder = folderUri ? vscode.workspace.getWorkspaceFolder(folderUri)
      : folders.length === 1 ? folders[0]
      : await vscode.window.showWorkspaceFolderPick({placeHolder:'Select the workspace to share with Vizzi'});
    if (!folder) {output.appendLine('Vizzi cancelled: no workspace folder selected.');return;}
    if (folder.uri.scheme !== 'file') throw new Error('Select a local file workspace');
    return folder;
  }
  async function report(error) {
    const message = `Vizzi: ${error.message}`;
    output.appendLine(message);
    output.show(true);
    await vscode.window.showErrorMessage(message);
  }
  async function openWorkspace(folderUri,external) {
    try {
      const connection = await shareLanguageProviders(folderUri);
      if (!connection) return;
      const invocation = generation;
      if (workspace?.root !== connection.root) await stopWorkspace();
      if (!workspace) {
        const override = vscode.workspace.getConfiguration('vizzi').get('executablePath');
        const binary = override || path.join(context.extensionPath,'bin',process.platform === 'win32' ? 'vizzi.exe' : 'vizzi');
        await fs.access(binary).catch(()=>{throw new Error('The Vizzi executable is missing. Install a bundled VSIX for your platform, or set Vizzi: Executable Path for development.');});
        const config = path.join(connection.root,'vizzi.toml');
        const configFile = await fs.stat(config).then(()=>config,error=>{
          if (error.code !== 'ENOENT') throw error;
        });
        if (invocation !== generation) return;
        let current;
        launchAbort = new AbortController();
        launchTask = vscode.window.withProgress({location:vscode.ProgressLocation.Notification,title:'Starting Vizzi'},
          ()=>startWorkspace({binary,...connection,configFile,output,onExit:()=>{
            if (workspace === current) {workspace=undefined;workspaceView.dispose();}
          },signal:launchAbort.signal}));
        try {current = await launchTask;} finally {launchTask=undefined;launchAbort=undefined;}
        if (invocation !== generation) {await current.dispose();return;}
        workspace = current;
        workspace.root = connection.root;
      }
      output.appendLine(`Open workspace: ${workspace.url}`);
      if (!external) workspaceView.show(workspace.url,path.basename(workspace.root));
      else if (!await vscode.env.openExternal(vscode.Uri.parse(workspace.url))) {
        throw new Error(`Open ${workspace.url} in your browser; Vizzi is running.`);
      }
      return {root:workspace.root,url:workspace.url};
    } catch (error) {await report(error);}
  }
  function open(folderUri,external=false) {
    if (!opening) opening = openWorkspace(folderUri,external).finally(()=>{opening=undefined;});
    return opening;
  }
  async function shareLanguageProviders(folderUri,externalSettings=false) {
    output.show(true);
    if (starting) {output.appendLine('Vizzi sharing is already starting.');return;}
    starting = true;
    output.appendLine('Starting Vizzi language-provider sharing…');
    let invocation = ++generation;
    let provider;
    try {
      const folder = await selectFolder(folderUri);
      if (!folder) return;
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
      if (externalSettings) {
        output.appendLine('For automatic setup, run Vizzi: Open Workspace. No connection settings are needed.');
        output.appendLine('Optional settings for an external Vizzi launch:');
        output.appendLine(`root = ${JSON.stringify(root)}\n\n[language-servers]\nvscode-connection = ${JSON.stringify(connectionFile)}`);
        output.appendLine('This connection path is stable for this project. Sharing again refreshes its session credentials.');
      } else output.appendLine('Editor connection configured automatically for this launch.');
      output.appendLine('\nVS Code keeps ownership of its language servers. Save buffers before querying from Vizzi.');
      output.show(true);
      return {root,connectionFile};
    } catch (error) {
      if (provider && active?.provider !== provider) provider.dispose();
      await report(error);
    } finally {starting = false;}
  }
  context.subscriptions.push(output,status,workspaceView,
    vscode.commands.registerCommand('vizzi.openWorkspace',folderUri=>open(folderUri)),
    vscode.commands.registerCommand('vizzi.openWorkspaceInBrowser',folderUri=>open(folderUri,true)),
    vscode.commands.registerCommand('vizzi.stopWorkspace',deactivate),
    vscode.commands.registerCommand('vizzi.stopSharing',stop),
    vscode.commands.registerCommand('vizzi.shareLanguageProviders',folderUri=>shareLanguageProviders(folderUri,true)));
}

module.exports = {activate,deactivate};
