const {test}=require('node:test');
const assert=require('node:assert/strict');
const {selectComparison,createComparison}=require('./comparison.cjs');

const hash=n=>String(n).repeat(40);
const uri=fsPath=>({scheme:'file',fsPath});
function fixture() {
  const rootUri=uri('/project'),calls=[],picks=[];
  const repo={rootUri,state:{refs:[{type:0,name:'main'},{type:2,name:'v1'}]},
    getCommit:async ref=>{calls.push(ref);if(ref==='missing')throw new Error('bad revision');return {hash:ref==='HEAD'?hash(2):ref.startsWith('refs/')?hash(3):ref,message:ref};},
    log:async()=>[{hash:hash(1),message:'First change',authorName:'Reader'}]};
  const api={repositories:[repo],getRepository:u=>u.fsPath.startsWith('/project')?repo:undefined};
  const vscode={env:{},workspace:{isTrusted:true,workspaceFolders:[{uri:rootUri}]},
    extensions:{getExtension:()=>({activate:async()=>({getAPI:()=>api})})},
    window:{showQuickPick:async(items,options)=>{items=await items;picks.push({items,options});return items.find(item=>item.ref===hash(1));}}};
  return {repo,api,vscode,calls,picks};
}

test('two successive revision picks pin base and target before launch',async()=>{
  const f=fixture();
  let step=0;
  f.vscode.window.showQuickPick=async(items,options)=>{
    items=await items;f.picks.push({items,options});
    const ref=step++===0?hash(1):'HEAD';
    return items.find(item=>item.ref===ref);
  };
  const result=await selectComparison(f.vscode,'commits');
  assert.deepEqual(result,{rootUri:f.repo.rootUri,folderUri:f.repo.rootUri,revision:hash(1),target_revision:hash(2)});
  assert.deepEqual(f.calls,[hash(1),'HEAD']);
  assert.match(f.picks[0].options.placeHolder,/base.*left/i);
  assert.match(f.picks[1].options.placeHolder,/target.*right/i);
});

test('cancelling either picker leaves no comparison request',async()=>{
  for(const cancelAt of [0,1]) {
    const f=fixture();let step=0;
    f.vscode.window.showQuickPick=async items=>step++===cancelAt?undefined:(await items).find(item=>item.ref===hash(1));
    assert.equal(await selectComparison(f.vscode,'commits'),undefined);
    assert.equal(f.calls.length,cancelAt);
  }
});

test('working comparison pins current HEAD without a revision picker or buffer saves',async()=>{
  const f=fixture();
  f.vscode.workspace.textDocuments=[{isDirty:true,save(){throw new Error('must not save');}}];
  assert.deepEqual(await selectComparison(f.vscode,'working'),{rootUri:f.repo.rootUri,folderUri:f.repo.rootUri,revision:hash(2)});
  assert.deepEqual(f.calls,['HEAD']);assert.equal(f.picks.length,0);
});

test('Source Control context wins over the active editor and supports a workspace subfolder',async()=>{
  const f=fixture(),other={...f.repo,rootUri:uri('/other')};
  f.api.repositories.push(other);
  f.vscode.workspace.workspaceFolders=[{uri:uri('/project/sub')},{uri:uri('/other')}];
  f.vscode.window.activeTextEditor={document:{uri:uri('/other/a.rs')}};
  const result=await selectComparison(f.vscode,'working',{rootUri:f.repo.rootUri});
  assert.equal(result.rootUri,f.repo.rootUri);assert.equal(result.folderUri.fsPath,'/project/sub');
});

test('Git API repository wrappers need not retain object identity across lookups',async()=>{
  const f=fixture();f.api.getRepository=()=>({...f.repo});
  const result=await selectComparison(f.vscode,'working',{rootUri:f.repo.rootUri});
  assert.equal(result.rootUri,f.repo.rootUri);
});

test('the first command waits for Git repository discovery',async()=>{
  const f=fixture();f.api.state='uninitialized';f.api.repositories=[];
  let disposed=false;
  f.api.onDidChangeState=callback=>{
    queueMicrotask(()=>{f.api.state='initialized';f.api.repositories=[f.repo];callback('initialized');});
    return {dispose(){disposed=true;}};
  };
  const result=await selectComparison(f.vscode,'working');
  assert.equal(result.revision,hash(2));assert.ok(disposed);
});

test('ambiguous repositories require a choice and cancellation stops',async()=>{
  const f=fixture();f.api.repositories.push({...f.repo,rootUri:uri('/project/nested')});
  f.vscode.window.showQuickPick=async(items,options)=>{assert.match(options.placeHolder,/repository/i);return undefined;};
  assert.equal(await selectComparison(f.vscode,'working'),undefined);assert.deepEqual(f.calls,[]);
});

test('missing Git, untrusted/remote workspaces and unborn HEAD fail actionably',async()=>{
  const f=fixture();
  f.vscode.workspace.isTrusted=false;
  await assert.rejects(selectComparison(f.vscode,'working'),/Trust/);
  f.vscode.workspace.isTrusted=true;f.vscode.env.remoteName='ssh';
  await assert.rejects(selectComparison(f.vscode,'working'),/local/);
  f.vscode.env.remoteName=undefined;f.repo.getCommit=async()=>{throw new Error('unborn');};
  await assert.rejects(selectComparison(f.vscode,'working'),/HEAD.*commit/i);
  f.vscode.extensions.getExtension=()=>undefined;
  await assert.rejects(selectComparison(f.vscode,'working'),/Git.*enable/i);
});

test('explicit revisions and further history pages resolve through the selected repository',async()=>{
  const f=fixture();const pages=[];let step=0;
  f.repo.log=async options=>{pages.push(options.skip);return Array.from({length:options.skip?1:101},(_,i)=>({hash:hash(options.skip?4:1),message:'change '+i}));};
  f.vscode.window.showQuickPick=async items=>{
    items=await items;
    return step++===0?items.find(i=>i.more):step===2?items.find(i=>i.ref===hash(4)):items.find(i=>i.input);
  };
  f.vscode.window.showInputBox=async()=> 'refs/tags/v1';
  const result=await selectComparison(f.vscode,'commits');
  assert.equal(result.revision,hash(4));assert.equal(result.target_revision,hash(3));
  assert.deepEqual(pages,[0,100,0]);
});

test('native comparison request preserves direction, scope and server diagnostics',async()=>{
  let request;
  const fetch=async(url,options)=>{request={url,options};return {ok:true,status:202,json:async()=>({schema:1,ticket:'capture'})};};
  await createComparison({url:'http://127.0.0.1:7474/'},{revision:hash(1),target_revision:hash(2)},{fetch});
  assert.equal(request.url,'http://127.0.0.1:7474/api/comparisons');
  assert.deepEqual(JSON.parse(request.options.body),{provider:'git',revision:hash(1),target_revision:hash(2),scope:'.'});
  await assert.rejects(createComparison({url:'http://127.0.0.1:7474/'},{revision:hash(1)},{fetch:async()=>({ok:false,text:async()=>JSON.stringify({message:'Repository unavailable'})})}),/Repository unavailable/);
});
