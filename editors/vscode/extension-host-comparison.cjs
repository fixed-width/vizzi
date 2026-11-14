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
    const git=await vscode.extensions.getExtension('vscode.git').activate(),api=git.getAPI(1);
    const repo=await api.openRepository(root);
    assert.ok(repo,'Git did not discover the fixture repository');
    const base=await repo.getCommit('HEAD~1'),target=await repo.getCommit('HEAD');
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(vscode.Uri.joinPath(root,'a.py')));
    storage=await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-comparison-'));
    await fs.writeFile(path.join(root.fsPath,'vizzi.toml'),`port = 0\ncache = ${JSON.stringify(path.join(storage,'cache'))}\n`);
    await vscode.workspace.getConfiguration('vizzi').update('executablePath',
      process.env.VIZZI_TEST_EXTENSION_PATH?undefined:process.env.VIZZI_TEST_BINARY,vscode.ConfigurationTarget.Global);
    const session=await vscode.commands.executeCommand('vizzi.compareWorkingChanges',{rootUri:root});
    assert.ok(session?.url,`Working changes did not start Vizzi: ${errors.join('; ')}`);
    const deadline=Date.now()+10000;
    let status;
    do {
      status=await fetch(session.url+'api/comparisons/status').then(r=>r.json());
      assert.ok(Date.now()<deadline,'Working capture timed out');
      assert.equal(status.error,null,JSON.stringify(status.error));
      if(status.pending)await new Promise(r=>setTimeout(r,50));
    } while(status.pending);
    assert.equal(status.active.resolved,target.hash);assert.equal(status.active.target,null);
    assert.equal((await fetch(session.url+'api/status').then(r=>r.json())).snapshot,null,'Review launch must not start live indexing');
    await vscode.commands.executeCommand('workbench.view.scm');
    await vscode.commands.executeCommand('workbench.action.closePanel');
    await vscode.commands.executeCommand('workbench.action.closeAuxiliaryBar');
    await writeSignal(process.env.VIZZI_TEST_REPORT+'.workspace',{url:session.url,base:base.hash,target:target.hash,token:status.active.token});
    if(process.env.VIZZI_TEST_WEBVIEW) {
      let inspected;
      const deadline=Date.now()+60000;
      while(!inspected) {
        inspected=await fs.readFile(process.env.VIZZI_TEST_REPORT+'.webview','utf8').then(JSON.parse,error=>{if(error.code!=='ENOENT')throw error;});
        assert.ok(Date.now()<deadline,'Review UI verification did not complete');
        if(!inspected)await new Promise(r=>setTimeout(r,100));
      }
      assert.ok(inspected.passed,inspected.error);
    }
    const tabs=vscode.window.tabGroups.all.flatMap(g=>g.tabs).filter(t=>t.input instanceof vscode.TabInputWebview && t.input.viewType.includes('vizzi.workspace'));
    assert.equal(tabs.length,1,'Comparisons must reuse the Vizzi editor tab');
    await writeSignal(process.env.VIZZI_TEST_REPORT,{passed:true});
  } catch(error) {
    await writeSignal(process.env.VIZZI_TEST_REPORT,{passed:false,error:error.stack});
    throw error;
  } finally {
    vscode.window.showErrorMessage=originalError;
    await vscode.commands.executeCommand('vizzi.stopWorkspace');
    if(storage)await fs.rm(storage,{recursive:true,force:true});
    await vscode.commands.executeCommand('workbench.action.quit');
  }
};
