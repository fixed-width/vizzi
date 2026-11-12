// Run in an isolated VS Code extension host with a temporary fixture workspace.
const vscode = require('vscode');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const readline = require('node:readline');
const {spawn} = require('node:child_process');

async function verify() {
  const root = vscode.workspace.workspaceFolders[0].uri;
  const file = vscode.Uri.joinPath(root,'a.py');
  const document = await vscode.workspace.openTextDocument(file);
  const target = new vscode.Location(file,new vscode.Position(1,4));
  let definitions = 0;
  const registrations = [
    vscode.languages.registerDefinitionProvider({language:'python',scheme:'file'}, {
      provideDefinition(){definitions++;return [target];},
    }),
    vscode.languages.registerReferenceProvider({language:'python',scheme:'file'}, {
      provideReferences(){return [new vscode.Location(file,new vscode.Position(2,11))];},
    }),
  ];
  const storage = await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-extension-native-'));
  let child;
  try {
    const connection = await vscode.commands.executeCommand('vizzi.shareLanguageProviders',root);
    assert.ok(connection?.connectionFile,'sharing command did not publish a connection');
    const config = path.join(storage,'vizzi.toml');
    await fs.writeFile(config,`root = ${JSON.stringify(connection.root)}\ncache = ${JSON.stringify(path.join(storage,'cache'))}\nworkers = 1\n[language-servers]\nvscode-connection = ${JSON.stringify(connection.connectionFile)}\n`);
    child = spawn(process.env.VIZZI_TEST_BINARY,['--config',config,'--stdio'],{stdio:['pipe','pipe','pipe']});
    let error = '';
    child.stderr.on('data',data=>{error+=data;});
    let next = 0;
    const pending = new Map();
    readline.createInterface({input:child.stdout}).on('line',line=>{
      const response = JSON.parse(line);
      const entry = pending.get(response.id);
      if (entry) {pending.delete(response.id);clearTimeout(entry.timer);entry.resolve(response);}
    });
    child.on('exit',code=>{for(const entry of pending.values()){clearTimeout(entry.timer);entry.reject(new Error(`native exit ${code}: ${error}`));}pending.clear();});
    async function request(uri,body) {
      const id = ++next;
      const response = await new Promise((resolve,reject)=>{
        const timer = setTimeout(()=>{pending.delete(id);reject(new Error(`native request timed out: ${uri}: ${error}`));},15000);
        pending.set(id,{resolve,reject,timer});
        child.stdin.write(JSON.stringify({id,uri,method:body?'POST':'GET',body})+'\n');
      });
      assert.equal(response.status,200,response.body);
      return JSON.parse(response.body);
    }
    let status;
    const deadline = Date.now()+10000;
    do {
      status = await request('/api/status');
      assert.ok(Date.now()<deadline,'initial capture did not finish');
      if(status.pending) await new Promise(resolve=>setTimeout(resolve,20));
    } while(status.pending);
    const snapshot = status.snapshot;
    const inventory = await request('/api/structure',{snapshot,query:{kind:'inventory',area:'a.py',search:'run',offset:0}});
    const calls = await request('/api/structure',{snapshot,query:{kind:'calls',symbol:inventory.items[0].symbol.id,area:'a.py',offset:0}});
    const definitionRequest = {snapshot,call:calls.items[0].id};
    async function stableDefinition() {
      const deadline = Date.now()+5000;
      for (;;) {
        const answer = await request('/api/call-definition',definitionRequest);
        if (answer.state !== 'unavailable' || Date.now() >= deadline) return answer;
        if (!/changed during analysis|busy/.test(answer.detail || '')) return answer;
        await new Promise(resolve=>setTimeout(resolve,100));
      }
    }
    const answer = await stableDefinition();
    assert.equal(answer.state,'unique',JSON.stringify(answer));
    assert.equal(answer.invocation.candidates[0].name,'leaf');
    assert.ok(definitions>0);
    const references = await request('/api/references',{snapshot,symbol:answer.invocation.candidates[0].id,kind:'references'});
    assert.equal(references.precision,'semantic',JSON.stringify(references));
    assert.equal(references.matching,1);
    const edit = new vscode.WorkspaceEdit();
    edit.insert(file,new vscode.Position(0,0),'# unsaved\n');
    await vscode.workspace.applyEdit(edit);
    assert.ok(document.isDirty);
    assert.equal((await request('/api/call-definition',definitionRequest)).state,'unavailable');
    const revert = new vscode.WorkspaceEdit();
    revert.delete(file,new vscode.Range(0,0,1,0));
    await vscode.workspace.applyEdit(revert);
    await document.save();
    await vscode.commands.executeCommand('vizzi.stopSharing');
    assert.equal((await request('/api/call-definition',definitionRequest)).state,'unavailable');
    const replacement = await vscode.commands.executeCommand('vizzi.shareLanguageProviders',root);
    assert.equal(replacement.connectionFile,connection.connectionFile);
    assert.equal((await stableDefinition()).state,'unique');
    console.log('Vizzi extension host: native TOML launch, shared definitions/references, dirty-buffer rejection, disconnect and reconnect passed.');
  } finally {
    child?.kill();
    await vscode.commands.executeCommand('vizzi.stopSharing');
    for(const registration of registrations) registration.dispose();
    await fs.rm(storage,{recursive:true,force:true});
  }
}

async function run() {
  try {
    await verify();
    if (process.env.VIZZI_TEST_REPORT) await fs.writeFile(process.env.VIZZI_TEST_REPORT,JSON.stringify({passed:true}));
  } catch (error) {
    if (process.env.VIZZI_TEST_REPORT) await fs.writeFile(process.env.VIZZI_TEST_REPORT,JSON.stringify({passed:false,error:error.stack}));
    throw error;
  }
}

module.exports = {run};
