const vscode=require('vscode');
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const path=require('node:path');
const os=require('node:os');
const {writeSignal}=require('./test-signals.cjs');

exports.run=async()=>{
  let storage;
  const errors=[],originalError=vscode.window.showErrorMessage;
  vscode.window.showErrorMessage=async message=>{errors.push(message);};
  try {
    const root=vscode.workspace.workspaceFolders[0].uri;
    const a=await vscode.workspace.openTextDocument(vscode.Uri.joinPath(root,'a.py'));
    const b=await vscode.workspace.openTextDocument(vscode.Uri.joinPath(root,'b.py'));
    const editor=await vscode.window.showTextDocument(a,{preview:false});
    editor.selection=new vscode.Selection(130,4,130,4);editor.revealRange(new vscode.Range(130,0,130,20),vscode.TextEditorRevealType.InCenter);
    await vscode.window.showTextDocument(b,{preview:false});
    storage=await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-source-browser-ui-'));
    await fs.writeFile(path.join(root.fsPath,'vizzi.toml'),`port = 0\ncache = ${JSON.stringify(path.join(storage,'cache'))}\n`);
    await vscode.workspace.getConfiguration('vizzi').update('executablePath',process.env.VIZZI_TEST_EXTENSION_PATH?undefined:process.env.VIZZI_TEST_BINARY,vscode.ConfigurationTarget.Global);
    const first=await vscode.commands.executeCommand('vizzi.openSourceBrowser',b.uri);
    assert.ok(first?.file,`Source-browser launch failed: ${errors.join('; ')}`);
    const session=await vscode.commands.executeCommand('vizzi.openWorkspace',root);
    assert.ok(session?.url);
    await vscode.commands.executeCommand('workbench.action.closePanel');
    await vscode.commands.executeCommand('workbench.action.closeAuxiliaryBar');
    await writeSignal(process.env.VIZZI_TEST_REPORT+'.workspace',{url:session.url});
    const deadline=Date.now()+70000;let inspected;
    while(!inspected) {
      inspected=await fs.readFile(process.env.VIZZI_TEST_REPORT+'.webview','utf8').then(JSON.parse,error=>{if(error.code!=='ENOENT')throw error;});
      assert.ok(Date.now()<deadline,`Source-browser UI test did not complete: ${errors.join('; ')}`);
      if(!inspected)await new Promise(r=>setTimeout(r,100));
    }
    assert.ok(inspected.passed,inspected.error);assert.deepEqual(errors,[]);
    const tabs=vscode.window.tabGroups.all.flatMap(g=>g.tabs).filter(t=>t.input instanceof vscode.TabInputWebview && t.input.viewType.includes('vizzi.workspace'));
    assert.equal(tabs.length,1);assert.equal(a.isDirty,false);assert.equal(b.isDirty,false);
    await writeSignal(process.env.VIZZI_TEST_REPORT,{passed:true});
  } catch(error) {await writeSignal(process.env.VIZZI_TEST_REPORT,{passed:false,error:error.stack});throw error;}
  finally {
    vscode.window.showErrorMessage=originalError;
    await vscode.commands.executeCommand('vizzi.stopWorkspace');
    if(storage)await fs.rm(storage,{recursive:true,force:true});
    await vscode.commands.executeCommand('workbench.action.quit');
  }
};
