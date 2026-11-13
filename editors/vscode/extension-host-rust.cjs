const vscode = require('vscode');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const {writeSignal}=require('./test-signals.cjs');

async function verify() {
  const root = vscode.workspace.workspaceFolders[0].uri;
  const file = vscode.Uri.joinPath(root,'a.rs');
  const document = await vscode.workspace.openTextDocument(file);
  await vscode.window.showTextDocument(document);
  const analyzer = vscode.extensions.getExtension('rust-lang.rust-analyzer');
  assert.ok(analyzer,'the isolated Rust extension is missing');
  await analyzer.activate();
  const position = new vscode.Position(1,12);
  let definitions;
  const deadline = Date.now()+60000;
  while (!definitions?.length) {
    definitions = await vscode.commands.executeCommand('vscode.executeDefinitionProvider',file,position).catch(()=>[]);
    assert.ok(Date.now()<deadline,'rust-analyzer did not resolve ok_or_else');
    if (!definitions?.length) await new Promise(resolve=>setTimeout(resolve,200));
  }
  const targetUri = definitions[0].targetUri || definitions[0].uri;
  assert.ok(targetUri.fsPath.endsWith('/core/src/option.rs'),targetUri.toString());
  const temporary = await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-rust-editor-'));
  try {
    await vscode.workspace.getConfiguration('vizzi').update('executablePath',
      process.env.VIZZI_TEST_EXTENSION_PATH?undefined:process.env.VIZZI_TEST_BINARY,vscode.ConfigurationTarget.Global);
    await fs.writeFile(path.join(root.fsPath,'vizzi.toml'),`root = "."\nport = 0\ncache = ${JSON.stringify(path.join(temporary,'cache'))}\n`);
    let workspace;
    if (process.env.VIZZI_TEST_EDITOR_CALL==='command') {
      const editor=await vscode.window.showTextDocument(document);
      editor.selection=new vscode.Selection(position,position);
      const handoff=await vscode.commands.executeCommand('vizzi.exploreCall',file);
      assert.ok(handoff?.call,'Explore Call command did not deliver the selected call');
      workspace=await vscode.commands.executeCommand('vizzi.openWorkspace');
    } else if (process.env.VIZZI_TEST_EDITOR_CALL) {
      await writeSignal(process.env.VIZZI_TEST_REPORT+'.editor-source',{callee:'ok_or_else'});
      const deadline=Date.now()+45000;
      while (!workspace) {
        workspace=await fs.readFile(process.env.VIZZI_TEST_REPORT+'.editor-opened','utf8').then(JSON.parse,error=>{if(error.code!=='ENOENT')throw error;});
        assert.ok(Date.now()<deadline,'editor context-menu action did not open Vizzi');
        if(!workspace)await new Promise(resolve=>setTimeout(resolve,100));
      }
    } else workspace=await vscode.commands.executeCommand('vizzi.openWorkspace');
    assert.ok(workspace?.url,'Vizzi did not launch');
    async function get(route) {
      const response = await fetch(workspace.url+route);
      assert.ok(response.ok,await response.clone().text());
      return response.json();
    }
    async function post(route,body) {
      const response = await fetch(workspace.url+route,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)});
      assert.ok(response.ok,await response.clone().text());
      return response.json();
    }
    await post('api/workspace/start',{});
    let status;
    do {
      status = await get('api/status');
      if (status.pending) await new Promise(resolve=>setTimeout(resolve,50));
    } while (status.pending);
    const snapshot = status.snapshot;
    const inventory = await post('api/structure',{snapshot,query:{kind:'inventory',area:'a.rs',search:'convert',offset:0}});
    const calls = await post('api/structure',{snapshot,query:{kind:'calls',symbol:inventory.items[0].symbol.id,area:'a.rs',offset:0}});
    const call = calls.items.find(call=>call.name==='ok_or_else');
    assert.ok(call,'Vizzi did not capture the written method call');
    let answer;
    const definitionDeadline = Date.now()+15000;
    do {
      answer = await post('api/call-definition',{snapshot,call:call.id});
      if (answer.state==='unique') break;
      if (Date.now()>=definitionDeadline) break;
      await new Promise(resolve=>setTimeout(resolve,100));
    } while (answer.state==='unavailable');
    assert.equal(answer.state,'unique',JSON.stringify(answer));
    const target = answer.invocation.candidates[0];
    assert.equal(target.name,'ok_or_else');
    assert.equal(target.origin.label,'core');
    const query = new URLSearchParams({snapshot:target.origin.capture,file:target.source.file,version:target.source.version,line_start:String(target.source.line),line_count:'96'});
    const source = await get('api/dependency/window?'+query);
    const body = source.lines.map(line=>line.text).join('\n');
    assert.ok(body.includes('fn ok_or_else'),body);
    assert.ok(body.includes('match self'),body);
    const tabs = ()=>vscode.window.tabGroups.all.flatMap(group=>group.tabs).filter(tab=>tab.input instanceof vscode.TabInputWebview && tab.input.viewType.includes('vizzi.workspace'));
    if (process.env.VIZZI_TEST_WEBVIEW) {
      async function inspect(suffix='') {
        const deadline = Date.now()+45000;
        for (;;) {
          const report = await fs.readFile(process.env.VIZZI_TEST_REPORT+'.webview'+suffix,'utf8').then(JSON.parse,error=>{if (error.code!=='ENOENT') throw error;});
          if (report) {assert.ok(report.passed,report.error);return;}
          assert.ok(Date.now()<deadline,'Rust webview check timed out');
          await new Promise(resolve=>setTimeout(resolve,100));
        }
      }
      await writeSignal(process.env.VIZZI_TEST_REPORT+'.workspace',{url:workspace.url,caller:'convert',callee:'ok_or_else',editorCall:Boolean(process.env.VIZZI_TEST_EDITOR_CALL)});
      await inspect();
      if(process.env.VIZZI_TEST_EDITOR_CALL) {
        const editor=await vscode.window.showTextDocument(document);
        editor.selection=new vscode.Selection(position,position);
        assert.ok((await vscode.commands.executeCommand('vizzi.exploreCall',file))?.call);
        await writeSignal(process.env.VIZZI_TEST_REPORT+'.editor-repeat',{});
        await inspect('-repeat');
      }
      await vscode.window.tabGroups.close(tabs()[0]);
      await vscode.commands.executeCommand('vizzi.openWorkspace');
      await writeSignal(process.env.VIZZI_TEST_REPORT+'.reopened',{});
      await inspect('-reopened');
    }
    console.log('Real VS Code rust-analyzer resolved ok_or_else into a captured core source bubble.');
  } finally {
    await vscode.commands.executeCommand('vizzi.stopWorkspace');
    await fs.rm(temporary,{recursive:true,force:true});
  }
}

async function run() {
  try {await verify();await fs.writeFile(process.env.VIZZI_TEST_REPORT,JSON.stringify({passed:true}));}
  catch (error) {await fs.writeFile(process.env.VIZZI_TEST_REPORT,JSON.stringify({passed:false,error:error.stack}));throw error;}
}
module.exports = {run};
