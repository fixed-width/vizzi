const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const {
  startBridge
} = require('./bridge.cjs');
const {
  createProvider
} = require('./providers.cjs');
const {
  startWorkspace
} = require('./launcher.cjs');
const {
  startOriginEditor
} = require('./origin-connection.cjs');
const {
  createEditorNavigator
} = require('./editor-navigation.cjs');
function createWorkspaceRuntime(vscode, context, output) {
  async function resolveProject(selection) {
    const editorRoot = (selection.rootUri || selection.folderUri).fsPath;
    const root = await fs.realpath(editorRoot);
    const key = crypto.createHash('sha256').update(root).digest('hex');
    const connectionFile = path.join(context.globalStorageUri.fsPath, `connections/${key}.json`);
    const relative = path.relative(root, connectionFile);
    if (relative === '' || relative !== '..' && !relative.startsWith('..' + path.sep) && !path.isAbsolute(relative)) {
      throw new Error('Vizzi connection storage must be outside the inspected workspace');
    }
    return {
      root,
      editorRoot,
      connectionFile
    };
  }
  async function createSharing(project, {
    signal
  }) {
    signal.throwIfAborted();
    const provider = createProvider(vscode, project.root, project.editorRoot);
    let bridge;
    try {
      bridge = await startBridge({
        ...project,
        query: provider.query
      });
      signal.throwIfAborted();
      output.clear();
      output.appendLine(`Sharing language providers for ${project.root}`);
      output.appendLine('Editor connection configured automatically for this launch.');
      output.appendLine('\nVS Code keeps ownership of its language servers. Save buffers before querying from Vizzi.');
      return {
        ...project,
        isCurrent: () => bridge.isCurrent(),
        dispose: async () => {
          provider.dispose();
          await bridge.dispose();
        }
      };
    } catch (error) {
      provider.dispose();
      await bridge?.dispose();
      throw error;
    }
  }
  async function launch(project, {
    signal,
    onExit
  }) {
    const override = vscode.workspace.getConfiguration('vizzi').get('executablePath');
    const binary = override || path.join(context.extensionPath, 'bin', process.platform === 'win32' ? 'vizzi.exe' : 'vizzi');
    await fs.access(binary).catch(() => {
      throw new Error('The Vizzi executable is missing. Install a bundled VSIX for your platform, or set Vizzi: Executable Path for development.');
    });
    const config = path.join(project.root, 'vizzi.toml');
    const configFile = await fs.stat(config).then(() => config, error => {
      if (error.code !== 'ENOENT') throw error;
    });
    signal.throwIfAborted();
    let workspace;
    async function verifyArchive(capture,handOffSignal) {
      if (!workspace) throw new Error('The Java archive verifier is unavailable');
      const response = await fetch(new URL('/api/dependency/verify-archive',workspace.url),{
        method:'POST',headers:{'Content-Type':'application/json'},
        body:JSON.stringify({capture}),signal:handOffSignal,redirect:'error'
      });
      if (response.status !== 204) throw new Error('Saved Java archive changed or is unavailable; reopen it from its caller');
    }
    workspace = await vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: 'Starting Vizzi'
    }, () => startWorkspace({
      binary,
      ...project,
      configFile,
      output,
      signal,
      onExit,
      createOrigin: () => startOriginEditor({
        root: project.root,
        storage: path.join(context.globalStorageUri.fsPath, 'origin-connections'),
        name: 'VS Code',
        javaArchives: true,
        open: createEditorNavigator(vscode, {
          root: project.root,
          editorRoot: project.editorRoot,
          verifyArchive
        })
      })
    }));
    return workspace;
  }
  return {
    resolveProject,
    createSharing,
    launch
  };
}
module.exports = {
  createWorkspaceRuntime
};
