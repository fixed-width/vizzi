const vscode = require('vscode');
const {
  createWorkspaceRuntime
} = require('./workspace-runtime.cjs');
const {
  createSourceWorkspaceLifetime
} = require('./source-workspace.cjs');
const {
  createWorkspaceView
} = require('./workspace-view.cjs');
const {
  captureEditorCall,
  captureSourceFile,
  verifySource
} = require('./editor-source.cjs');
const {
  createHandoffClient
} = require('./handoff.cjs');
const comparison = require('./comparison.cjs');
let lifetime;
let editorHandoffs;
async function deactivate() {
  editorHandoffs?.dispose();
  await lifetime?.dispose();
}
function activate(context) {
  editorHandoffs = createHandoffClient();
  const output = vscode.window.createOutputChannel('Vizzi');
  const view = createWorkspaceView(vscode);
  let comparing;
  const reportedErrors = new WeakSet();
  const showTab = async ({
    session,
    signal
  }) => {
    output.appendLine(`Open workspace: ${session.url}`);
    const outcome = await view.present(session, {kind:'reveal'}, {signal});
    return outcome.state === 'applied' ? session : undefined;
  };
  lifetime = createSourceWorkspaceLifetime({
    ...createWorkspaceRuntime(vscode, context, output),
    present: showTab,
    retirePresentation: () => {
      try {view.dispose();} catch (error) {
        output.appendLine(`Could not close the Vizzi tab: ${error.message}`);
        throw error;
      }
    }
  });
  const status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left);
  status.text = '$(preview) Vizzi';
  status.tooltip = 'Open the Vizzi visual workspace';
  status.command = 'vizzi.openWorkspace';
  status.show();
  function announce() {
    output.show(true);
    output.appendLine('Starting Vizzi language-provider sharing…');
  }
  async function selectFolder(folderUri) {
    if (!vscode.workspace.isTrusted) throw new Error('Trust this workspace in VS Code before sharing language providers');
    if (vscode.env.remoteName) throw new Error('Shared language providers currently require a local VS Code workspace');
    const folders = vscode.workspace.workspaceFolders || [];
    if (!folders.length) throw new Error('Open a project folder with File → Open Folder before starting Vizzi');
    const folder = folderUri ? vscode.workspace.getWorkspaceFolder(folderUri) : folders.length === 1 ? folders[0] : await vscode.window.showWorkspaceFolderPick({
      placeHolder: 'Select the workspace to share with Vizzi'
    });
    if (!folder) {
      output.appendLine('Vizzi cancelled: no workspace folder selected.');
      return;
    }
    if (folder.uri.scheme !== 'file') throw new Error('Select a local file workspace');
    return {
      folderUri: folder.uri
    };
  }
  async function report(error) {
    if (reportedErrors.has(error)) return;
    reportedErrors.add(error);
    const message = `Vizzi: ${error.message}`;
    output.appendLine(message);
    output.show(true);
    await vscode.window.showErrorMessage(message);
  }
  async function run(action) {
    try {
      return await action();
    } catch (error) {
      await report(error);
    }
  }
  function open(folderUri, external = false) {
    announce();
    return lifetime.use(() => selectFolder(folderUri), {
      present: external ? async ({
        session,
        signal
      }) => {
        signal.throwIfAborted();
        output.appendLine(`Open workspace: ${session.url}`);
        if (!(await vscode.env.openExternal(vscode.Uri.parse(session.url)))) throw new Error(`Open ${session.url} in your browser; Vizzi is running.`);
        return session;
      } : showTab
    });
  }
  async function compare(mode, resource) {
    if (comparing && !comparing.signal?.aborted) return;
    const request = {};
    comparing = request;
    announce();
    try {
      return await lifetime.use(signal => {
        request.signal = signal;
        return comparison.selectComparison(vscode, mode, resource);
      }, {
        prepare: ({
          session,
          selection,
          signal
        }) => comparison.createComparison(session, selection, {
          signal
        }),
        present: async ({
          session,
          prepared,
          signal
        }) => {
          const outcome = await view.present(session, {kind:'review',ticket:prepared.ticket}, {signal});
          if (outcome.state !== 'applied') return;
          return {
            ...session,
            ...prepared
          };
        }
      });
    } catch (error) {
      await report(error);
    } finally {
      if (comparing === request) comparing = undefined;
    }
  }
  async function enterSource(kind, resource) {
    const intent = editorHandoffs.begin();
    announce();
    try {
      return await lifetime.use(async () => {
        const source = await (kind === 'call' ? captureEditorCall : captureSourceFile)(vscode, resource);
        const project = await selectFolder(source.uri);
        return project && {
          ...project,
          source
        };
      }, {
        signal: intent.signal,
        prepare: ({
          session,
          selection: {
            source
          },
          signal
        }) => {
          const target = kind === 'call' ? {
            kind,
            position: source.position
          } : {
            kind,
            line: source.line
          };
          return editorHandoffs.resolve(session, source, target, intent, {
            signal,
            check: ({
              final
            }) => verifySource(source, {
              saved: final
            })
          });
        },
        present: async ({
          session,
          selection: {
            source
          },
          prepared,
          signal
        }) => {
          const outcome = await view.present(session, {kind,...prepared}, {
            signal,validate:()=>verifySource(source)
          });
          if (outcome.state !== 'applied') return;
          return prepared;
        }
      });
    } catch (error) {
      if (intent.current()) await report(error);
    }
  }
  function share(folderUri) {
    announce();
    return lifetime.use(() => selectFolder(folderUri), {
      resources: 'sharing',
      refresh: true,
      present: ({
        project
      }) => {
        output.appendLine('For automatic setup, run Vizzi: Open Workspace. No connection settings are needed.');
        output.appendLine('Optional settings for an external Vizzi launch:');
        output.appendLine(`root = ${JSON.stringify(project.root)}\n\n[language-servers]\nvscode-connection = ${JSON.stringify(project.connectionFile)}`);
        output.appendLine('This connection path is stable for this project. Sharing again refreshes its session credentials.');
        return project;
      }
    });
  }
  context.subscriptions.push(output, status, view,
    vscode.commands.registerCommand('vizzi.openWorkspace', folderUri => run(() => open(folderUri))),
    vscode.commands.registerCommand('vizzi.openWorkspaceInBrowser', folderUri => run(() => open(folderUri, true))),
    vscode.commands.registerCommand('vizzi.exploreCall', resource => enterSource('call', resource)),
    vscode.commands.registerCommand('vizzi.openSourceBrowser', resource => enterSource('file', resource)),
    vscode.commands.registerCommand('vizzi.compareRevisions', resource => compare('commits', resource)),
    vscode.commands.registerCommand('vizzi.compareWorkingChanges', resource => compare('working', resource)),
    vscode.commands.registerCommand('vizzi.stopWorkspace', () => run(() => lifetime.stop())),
    vscode.commands.registerCommand('vizzi.stopSharing', () => run(() => lifetime.stop('sharing'))),
    vscode.commands.registerCommand('vizzi.shareLanguageProviders', folderUri => run(() => share(folderUri))));
}
module.exports = {
  activate,
  deactivate
};
