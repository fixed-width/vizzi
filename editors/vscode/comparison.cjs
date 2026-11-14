const path=require('node:path');

const PAGE=100;
const contains=(root,file)=>{const relative=path.relative(root,file);return relative==='' || relative!=='..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);};
function localRevision(ref) {
  if(typeof ref!=='string' || !ref.trim() || ref.length>1024 || ref.startsWith('-') || /[\x00-\x20\x7f]/.test(ref)) {
    throw new Error('Enter a local branch, tag, commit ID or revision such as HEAD~1');
  }
  return ref;
}
async function resolve(repo,ref) {
  try {
    const commit=await repo.getCommit(localRevision(ref));
    if(!/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(commit.hash))throw new Error('Invalid commit ID');
    return commit.hash;
  } catch(error) {throw new Error(`Cannot resolve ${ref} to a local commit. Check the revision and that this repository has a commit. ${error.message}`);}
}
async function pickRevision(vscode,repo,step,base) {
  let skip=0;
  for(;;) {
    const commits=await repo.log({maxEntries:PAGE+1,skip});
    const items=[{label:'HEAD',description:'Current commit',ref:'HEAD'},
      ...(repo.state.refs || []).slice(0,200).filter(ref=>ref.name && [0,1,2].includes(ref.type)).map(ref=>({
        label:ref.name,description:['Branch','Remote branch','Tag'][ref.type],
        ref:`${['refs/heads/','refs/remotes/','refs/tags/'][ref.type]}${ref.name}`,
      })),
      ...commits.slice(0,PAGE).map(commit=>({label:commit.message.split(/\r?\n/,1)[0],
        description:commit.hash.slice(0,8),detail:commit.authorName,ref:commit.hash})),
      {label:'$(edit) Enter a revision…',description:'Branch, tag, commit ID or HEAD~1',input:true,alwaysShow:true},
      ...(commits.length>PAGE?[{label:'$(history) More commits…',more:true,alwaysShow:true}]:[]),
    ];
    const picked=await vscode.window.showQuickPick(items,{title:`Vizzi: Compare Revisions (${step}/2)`,
      placeHolder:step===1?'Select the base revision (left)':`Select the target revision (right) to compare with ${base.slice(0,8)}`,
      matchOnDescription:true,matchOnDetail:true,ignoreFocusOut:true});
    if(!picked)return;
    if(picked.more){skip+=PAGE;continue;}
    const ref=picked.input?await vscode.window.showInputBox({title:'Vizzi: Enter a revision',
      prompt:step===1?'Base revision (left)':'Target revision (right)',placeHolder:'For example: main, v1.0, HEAD~1 or a commit ID',
      ignoreFocusOut:true,validateInput:value=>{try{localRevision(value);return;}catch(error){return error.message;}}}):picked.ref;
    if(ref===undefined)return;
    return resolve(repo,ref);
  }
}

async function selectComparison(vscode,mode,context) {
  if(!vscode.workspace.isTrusted)throw new Error('Trust this workspace in VS Code before comparing revisions');
  if(vscode.env.remoteName)throw new Error('Vizzi comparisons currently require a local VS Code workspace');
  const extension=vscode.extensions.getExtension('vscode.git');
  if(!extension)throw new Error('Git integration is unavailable; enable the built-in Git extension in VS Code');
  const git=await extension.activate();
  if(git.enabled===false)throw new Error('Git integration is disabled; enable Git in VS Code');
  const api=git.getAPI(1),folders=vscode.workspace.workspaceFolders || [];
  if(api.state==='uninitialized')await new Promise((resolve,reject)=>{
    let listener,timer;
    const finish=error=>{clearTimeout(timer);listener?.dispose();if(error)reject(error);else resolve();};
    listener=api.onDidChangeState(state=>{if(state==='initialized')finish();});
    timer=setTimeout(()=>finish(new Error('Git repository discovery is still starting; retry the comparison')),15000);
    if(api.state==='initialized')finish();
  });
  const folderFor=repo=>folders.find(folder=>folder.uri.scheme==='file' &&
    (contains(folder.uri.fsPath,repo.rootUri.fsPath) || contains(repo.rootUri.fsPath,folder.uri.fsPath)));
  const repositories=api.repositories.filter(repo=>repo.rootUri.scheme==='file' && folderFor(repo));
  let repo;
  const contextUri=context?.rootUri || (context?.scheme?context:undefined);
  if(contextUri) {
    const selected=api.getRepository(contextUri);
    repo=repositories.find(repository=>repository.rootUri.fsPath===selected?.rootUri.fsPath);
    if(!repo)throw new Error('Select a Git repository in an open local workspace folder');
  } else if(repositories.length===1)repo=repositories[0];
  else if(repositories.length>1)repo=(await vscode.window.showQuickPick(repositories.map(repository=>({
    label:path.basename(repository.rootUri.fsPath),description:repository.rootUri.fsPath,repository,
  })),{title:'Vizzi: Compare changes',placeHolder:'Select the Git repository to compare',ignoreFocusOut:true}))?.repository;
  else throw new Error('No Git repository found. Open a local Git project and enable Git in VS Code');
  if(!repo)return;
  const revision=mode==='working'?await resolve(repo,'HEAD'):await pickRevision(vscode,repo,1);
  if(!revision)return;
  const target=mode==='working'?undefined:await pickRevision(vscode,repo,2,revision);
  if(mode!=='working' && !target)return;
  return {rootUri:repo.rootUri,folderUri:folderFor(repo).uri,revision,...(target?{target_revision:target}:{})};
}

async function createComparison(session,selection,{fetch=globalThis.fetch,signal}={}) {
  const response=await fetch(new URL('api/comparisons',session.url).href,{method:'POST',
    headers:{'Content-Type':'application/json'},signal:signal?AbortSignal.any([signal,AbortSignal.timeout(15000)]):AbortSignal.timeout(15000),
    body:JSON.stringify({provider:'git',revision:selection.revision,...(selection.target_revision?{target_revision:selection.target_revision}:{}),scope:'.'})});
  if(!response.ok) {
    let message=await response.text();try{message=JSON.parse(message).message || message;}catch{}
    throw new Error(`Cannot start the Vizzi comparison: ${message.slice(0,512)}`);
  }
  const accepted=await response.json();
  if(accepted?.schema!==1 || typeof accepted.ticket!=='string')throw new Error('Invalid Vizzi comparison response; update the extension and executable together');
  return accepted;
}

module.exports={selectComparison,createComparison};
