const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const vm = require('node:vm');

async function fixture(t, folders = 1) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-commands-'));
  const root = path.join(directory,'project');
  await fs.mkdir(root);
  const binary = path.join(directory,'vizzi');
  await fs.writeFile(binary,'fixture executable');
  t.after(()=>fs.rm(directory,{recursive:true,force:true}));
  const folder = {uri:{scheme:'file',fsPath:root}};
  const commands = new Map(), lines = [], errors = [], opened = [], launches = [], panels = [], connections = [], editorCalls = [];
  let shown = 0, picks = 0, disposed = 0;
  const output = {clear(){lines.length=0;},appendLine:line=>lines.push(line),
    append:line=>lines.push(line),show(){shown++;},dispose(){}};
  const vscode = {
    ExtensionMode:{Development:2},StatusBarAlignment:{Left:1},ProgressLocation:{Notification:15},ViewColumn:{Active:-1},
    Uri:{parse:value=>value},
    env:{openExternal:async uri=>{opened.push(uri);return true;}},
    window:{createOutputChannel:()=>output,
      createWebviewPanel:(type,title,column,options)=>{
        const callbacks = [];
        const messages = [];
        const panel = {type,title,column,options,messages,webview:{html:'',
          onDidReceiveMessage:callback=>{queueMicrotask(()=>callback({type:'vizzi.ready'}));return {dispose(){}};},
          postMessage:async value=>{messages.push(value);return true;}},revealed:0,closed:false,
          reveal(){this.revealed++;},onDidDispose:callback=>{callbacks.push(callback);},
          dispose(){this.closed=true;for (const callback of callbacks) callback();}};
        panels.push(panel);
        return panel;
      },
      createStatusBarItem:()=>({show(){},dispose(){}}),
      showWorkspaceFolderPick:async()=>{picks++;return undefined;},
      showErrorMessage:async message=>{errors.push(message);},
      withProgress:async(_options,task)=>task()},
    workspace:{isTrusted:true,workspaceFolders:Array(folders).fill(folder),
      getWorkspaceFolder:()=>folder,getConfiguration:()=>({get:()=>binary})},
    commands:{registerCommand:(name,callback)=>{commands.set(name,callback);return {dispose(){}};},
      executeCommand:(name,...args)=>commands.get(name)(...args)},
  };
  const context = {globalStorageUri:{fsPath:path.join(directory,'storage')},
    extensionPath:__dirname,extensionMode:2,subscriptions:[]};
  const module = {exports:{}};
  let start = async options=>{
    launches.push(options);
    return {url:'http://127.0.0.1:7474/',dispose:async()=>{disposed++;}};
  };
  const dependencies = {
    vscode,
    './providers.cjs':{createProvider:()=>({query(){},dispose(){}})},
    './bridge.cjs':{startBridge:async options=>{connections.push(options);return {dispose:async()=>{},isCurrent:async()=>true};}},
    './launcher.cjs':{startWorkspace:options=>start(options)},
    './editor-call.cjs':{
      captureEditorCall:(...args)=>require('./editor-call.cjs').captureEditorCall(...args),
      resolveEditorCall:async(session,selection)=>{editorCalls.push({session,selection});return {snapshot:'a'.repeat(64),call:'b'.repeat(64)};},
    },
  };
  const source = await fs.readFile(path.join(__dirname,'extension.cjs'),'utf8');
  vm.compileFunction(source,['require','module'],{filename:'extension.cjs'})(
    name=>dependencies[name] || require(name),module);
  module.exports.activate(context);
  t.after(()=>module.exports.deactivate());
  return {directory,root,folder,vscode,context,commands,lines,errors,opened,launches,panels,connections,editorCalls,
    shown:()=>shown,picks:()=>picks,disposed:()=>disposed,setStart:callback=>{start=callback;}};
}

test('sharing without a folder argument uses the only workspace and reveals its settings',async t=>{
  const f = await fixture(t);
  const result = await f.commands.get('vizzi.shareLanguageProviders')();
  assert.equal(result.root,await fs.realpath(f.root));
  assert.equal(f.picks(),0);
  assert.ok(f.shown()>0);
  assert.match(f.lines.join('\n'),/Sharing language providers for/);
});

test('an empty window reports how to open a project instead of silently returning',async t=>{
  const f = await fixture(t,0);
  await f.commands.get('vizzi.shareLanguageProviders')();
  assert.match(f.errors.join('\n'),/Open.*folder/i);
  assert.ok(f.shown()>0);
});

test('cancelling a multi-folder choice leaves a visible explanation',async t=>{
  const f = await fixture(t,2);
  await f.commands.get('vizzi.shareLanguageProviders')();
  assert.equal(f.picks(),1);
  assert.match(f.lines.join('\n'),/cancelled/i);
  assert.ok(f.shown()>0);
});

test('open workspace launches once, uses project-local settings and stops its owned process',async t=>{
  const f = await fixture(t);
  const config = path.join(f.root,'vizzi.toml');
  await fs.writeFile(config,'root = "."\n');
  const open = f.commands.get('vizzi.openWorkspace');
  await open();
  await open();
  assert.equal(f.launches.length,1);
  assert.equal(f.connections.length,1);
  assert.equal(f.launches[0].configFile,config);
  assert.equal(f.launches[0].root,await fs.realpath(f.root));
  assert.equal(f.opened.length,0);
  assert.equal(f.panels.length,1);
  assert.equal(f.panels[0].revealed,1);
  assert.match(f.panels[0].webview.html,/<iframe[^>]+src="http:\/\/127\.0\.0\.1:7474\/"/);
  await f.commands.get('vizzi.stopWorkspace')();
  assert.equal(f.disposed(),1);
  assert.equal(f.panels[0].closed,true);
});

test('an unsuccessful browser open shows the running workspace address',async t=>{
  const f = await fixture(t);
  f.vscode.env.openExternal = async()=>false;
  await f.commands.get('vizzi.openWorkspaceInBrowser')();
  assert.match(f.errors.join('\n'),/http:\/\/127\.0\.0\.1:7474/);
  assert.equal(f.disposed(),0);
});

test('concurrent open commands share one launch and untrusted workspaces fail visibly',async t=>{
  const f = await fixture(t);
  const open = f.commands.get('vizzi.openWorkspace');
  await Promise.all([open(),open()]);
  assert.equal(f.launches.length,1);
  await f.commands.get('vizzi.stopWorkspace')();
  f.vscode.workspace.isTrusted = false;
  await open();
  assert.match(f.errors.join('\n'),/Trust this workspace/);
  assert.equal(f.launches.length,1);
});

test('bundled installations launch their included executable without a TOML file or path setting',async t=>{
  const f = await fixture(t);
  f.context.extensionPath = f.directory;
  await fs.mkdir(path.join(f.directory,'bin'));
  const binary = path.join(f.directory,'bin',process.platform==='win32'?'vizzi.exe':'vizzi');
  await fs.writeFile(binary,'bundled fixture');
  f.vscode.workspace.getConfiguration = ()=>({get:()=>undefined});
  await f.commands.get('vizzi.openWorkspace')();
  assert.equal(f.launches.length,1);
  assert.equal(f.launches[0].binary,binary);
  assert.equal(f.launches[0].configFile,undefined);
});

test('stopping a launch that becomes ready concurrently disposes it without opening the browser',async t=>{
  const f = await fixture(t);
  let ready, notify, disposed = 0;
  const started = new Promise(resolve=>{notify=resolve;});
  f.setStart(()=>{notify();return new Promise(resolve=>{ready=()=>resolve({
    url:'http://127.0.0.1:7474/',dispose:async()=>{disposed++;},
  });});});
  const opening = f.commands.get('vizzi.openWorkspace')();
  await started;
  const stopping = f.commands.get('vizzi.stopWorkspace')();
  ready();
  await Promise.all([opening,stopping]);
  assert.equal(disposed,1);
  assert.equal(f.opened.length,0);
  assert.equal(f.panels.length,0);
});

test('closing and reopening the editor tab reuses the server, and browser opening is explicit',async t=>{
  const f = await fixture(t);
  const open = f.commands.get('vizzi.openWorkspace');
  await open();
  f.panels[0].dispose();
  assert.equal(f.disposed(),0);
  await open();
  assert.equal(f.launches.length,1);
  assert.equal(f.panels.length,2);
  assert.equal(f.panels[1].options.retainContextWhenHidden,true);
  await f.commands.get('vizzi.openWorkspaceInBrowser')();
  assert.equal(f.launches.length,1);
  assert.equal(f.panels.length,2);
  assert.deepEqual(f.opened,['http://127.0.0.1:7474/']);
});

test('changing projects closes the previous tab and launches the selected project',async t=>{
  const f = await fixture(t);
  const open = f.commands.get('vizzi.openWorkspace');
  await open();
  const other = path.join(f.directory,'other');
  await fs.mkdir(other);
  f.vscode.workspace.workspaceFolders = [{uri:{scheme:'file',fsPath:other}}];
  await open();
  assert.equal(f.disposed(),1);
  assert.equal(f.panels[0].closed,true);
  assert.equal(f.panels.length,2);
  assert.equal(f.launches[1].root,other);
  assert.equal(f.panels[1].title,'Vizzi — other');
});

test('an HTTP server on the default port can open in an editor tab',async t=>{
  const f = await fixture(t);
  f.setStart(async()=>({url:'http://127.0.0.1:80/',dispose:async()=>{}}));
  await f.commands.get('vizzi.openWorkspace')();
  assert.equal(f.panels.length,1);
  assert.equal(f.panels[0].options.portMapping[0].extensionHostPort,80);
  assert.equal(f.errors.length,0);
});

test('automatic launches supply the current connection across restarts without asking for config edits',async t=>{
  const f = await fixture(t);
  const open = f.commands.get('vizzi.openWorkspace');
  await open();
  assert.doesNotMatch(f.lines.join('\n'),/Add these settings|vscode-connection\s*=/);
  assert.equal(f.launches[0].connectionFile,f.connections[0].connectionFile);
  assert.ok(f.launches[0].connectionFile);
  await f.commands.get('vizzi.stopWorkspace')();
  await open();
  assert.equal(f.launches.length,2);
  assert.equal(f.launches[1].connectionFile,f.connections[1].connectionFile);
  assert.equal(f.launches[1].connectionFile,f.launches[0].connectionFile);
  assert.doesNotMatch(f.lines.join('\n'),/Add these settings|vscode-connection\s*=/);
  await assert.rejects(fs.access(path.join(f.root,'vizzi.toml')),{code:'ENOENT'});
});

test('advanced provider sharing points users to automatic setup and labels optional external settings',async t=>{
  const f = await fixture(t);
  await f.commands.get('vizzi.shareLanguageProviders')();
  const text = f.lines.join('\n');
  assert.match(text,/For automatic setup, run Vizzi: Open Workspace/);
  assert.match(text,/Optional settings for an external Vizzi launch/);
  assert.match(text,/vscode-connection\s*=/);
  assert.doesNotMatch(text,/Add these settings/);
});

async function sourceEditor(f) {
  const file=path.join(f.root,'a.py'),text='def caller(): leaf()\n';
  await fs.writeFile(file,text);
  const uri={scheme:'file',fsPath:file,toString(){return this.fsPath;}};
  const document={uri,isDirty:false,getText:()=>text,getWordRangeAtPosition:()=>({start:{line:0,character:14}})};
  f.vscode.window.activeTextEditor={document,selection:{isEmpty:true,active:{line:0,character:16}}};
  return {document,text,uri};
}

test('explore call captures the source cursor before focus moves and targets the existing editor tab',async t=>{
  const f=await fixture(t),source=await sourceEditor(f);
  f.setStart(async()=>{f.vscode.window.activeTextEditor=undefined;return {url:'http://127.0.0.1:7474/',dispose:async()=>{}};});
  await f.commands.get('vizzi.exploreCall')(source.uri);
  assert.equal(f.editorCalls[0].selection.text,source.text);
  assert.deepEqual(f.editorCalls[0].selection.position,{line:0,character:14});
  assert.equal(f.panels.length,1);
  assert.equal(f.panels[0].messages[0].call,'b'.repeat(64));
  assert.equal(f.panels[0].messages[0].snapshot,'a'.repeat(64));
  assert.equal(f.opened.length,0);
});

test('explore call asks for saved source before launching or changing the task',async t=>{
  const f=await fixture(t),source=await sourceEditor(f);
  source.document.isDirty=true;
  await f.commands.get('vizzi.exploreCall')(source.uri);
  assert.match(f.errors.join('\n'),/Save/);
  assert.equal(f.launches.length,0);
  assert.equal(f.panels.length,0);
});
