import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {writeFile} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {browser,buildHost,paint,startHost,workspace} from './support.mjs';

if(!process.argv.includes('--no-build'))buildHost();
const fixture=await workspace();
if(process.argv.includes('--debug-host'))fixture.executable=resolve('target/debug/vizzi');

function git(...args){
 const result=spawnSync('git',['-C',fixture.root,...args],{encoding:'utf8'});
 assert.equal(result.status,0,result.stderr);
 return result.stdout.trim();
}
function commit(){
 git('add','.');
 git('-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','fixture');
 return git('rev-parse','HEAD');
}
const source=(marker,helper)=>`fn target() {\n    helper();\n    let ${marker} = 1;\n${Array.from({length:180},(_,i)=>`    // retained line ${i}`).join('\n')}\n}\nfn helper() { let ${helper} = 1; }\n`;

let host,chromium;
try{
 git('init','-q');
 const file=join(fixture.root,'lib.rs');
 await writeFile(file,source('earlier_caller','earliest_helper'));
 const earlier=commit();
 await writeFile(file,source('retained_caller','historical_helper'));
 commit();
 await writeFile(join(fixture.root,'notes.txt'),'A commit that leaves the function unchanged');
 const selected=commit();
 git('branch','review-base',selected);
 await writeFile(file,source('retained_caller','current_helper'));
 const head=commit();

 host=await startHost(fixture,1);
 chromium=await browser({desktopBus:false});
 const page=await chromium.newPage({viewport:{width:1500,height:1000}}),errors=[];
 page.on('pageerror',error=>errors.push(error.message));
 await page.goto(host.url+'/workspace');
 await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
 await page.getByRole('button',{name:'Add bubble',exact:true}).click();
 await page.getByLabel('Find a function',{exact:true}).fill('target');
 await page.getByRole('button',{name:'Find functions',exact:true}).click();
 await page.locator('[data-add-symbol][data-name="target"]').click();
 const bubble=page.locator('[data-view][data-name="target"]').first(),code=bubble.locator('.code-pane');
 await code.locator('.code-line').first().waitFor();
 const geometry=()=>bubble.evaluate(node=>[node.dataset.view,node.style.cssText,document.querySelector('#workspace-world').style.transform]);
 const original=await geometry();
 const choose=bubble.getByRole('button',{name:'Choose revision…',exact:true});
 const bubbleActions=bubble.getByRole('button',{name:'Actions for target bubble',exact:true});
 async function openPicker(){
  if(await choose.isVisible())await choose.press('Enter');
  else{
   await bubbleActions.press('Enter');
   await page.getByRole('menuitem',{name:'Choose revision…',exact:true}).press('Enter');
  }
 }
 const dialog=page.getByRole('dialog',{name:'Choose revision'});
 const revision=dialog.getByLabel('Local commit ID, branch or tag');
 const label=bubble.locator('.fragment-history span');
 const atCommit=commit=>page.waitForFunction(hash=>document.querySelector('[data-name="target"] .fragment-history span').textContent===hash.slice(0,8),commit);
 const confirmed=new Set();
 page.on('request',request=>{
  if(new URL(request.url()).pathname==='/api/function-history/confirm')confirmed.add(request.postDataJSON().token);
 });

 await bubble.getByRole('button',{name:'Next section',exact:true}).click();
 await code.locator('[data-line="97"]').waitFor();
 await choose.press('Enter');
 assert.equal(await revision.evaluate(node=>node===document.activeElement),true);
 await revision.fill('missing-ref');
 await revision.press('Enter');
 const alert=dialog.getByRole('alert');
 await alert.waitFor();
 assert.match(await alert.textContent(),/not available locally/);
 assert.match(await code.textContent(),/retained line/);
 assert.deepEqual(await geometry(),original);
 await revision.fill('refs/heads/review-base');
 const [confirmation]=await Promise.all([
  page.waitForResponse(response=>new URL(response.url()).pathname==='/api/function-history/confirm'),
  revision.press('Enter'),
 ]);
 assert.ok(confirmation.ok());
 await dialog.waitFor({state:'hidden'});
 await atCommit(selected);
 assert.ok((await label.getAttribute('aria-label')).includes(selected));
 assert.equal(await choose.evaluate(node=>node===document.activeElement),true);
 await code.locator('[data-line="97"]').waitFor();
 assert.deepEqual(await geometry(),original);

 git('branch','-f','review-base',head);
 await page.reload();
 await atCommit(selected);
 await code.locator('[data-line="97"]').waitFor();
 await bubble.getByRole('button',{name:'Previous function change',exact:true}).click();
 await atCommit(earlier);
 await bubble.getByRole('button',{name:'Next function change',exact:true}).click();
 await atCommit(selected);
 await bubble.getByRole('button',{name:'Previous section',exact:true}).click();
 await bubble.getByRole('button',{name:'Actions for helper',exact:true}).click({modifiers:['Control']});
 const helper=page.locator('[data-view][data-name="helper"]');
 await helper.locator('.code-line').first().waitFor();
 assert.match(await helper.locator('.code-pane').textContent(),/historical_helper/);
 await bubble.getByRole('button',{name:'Compare with previous version',exact:true}).click();
 const before=page.locator('[data-comparison-side="older"]');
 await before.locator('.comparison-removed').first().waitFor();
 assert.match(await before.locator('.code-pane').textContent(),/earlier_caller/);
 assert.ok((await bubble.locator('.fragment-history span').getAttribute('title')).includes(selected));

 for(const lateFailure of [false,true]){
  const restoreFocus=await choose.isVisible()?choose:bubbleActions;
  await page.evaluate(failure=>{
   const fetch=window.fetch;
   window.revisionWaiting=false;
   window.fetch=async (path,options)=>{
    if(path!=='/api/function-history/select')return fetch(path,options);
    window.fetch=fetch;
    const response=await fetch(path,{...options,signal:undefined});
    window.abandonedRevision=(await response.clone().json()).token;
    await new Promise(resolve=>{window.releaseRevision=resolve;window.revisionWaiting=true;});
    window.revisionDelivered=true;
    return failure?new Response('{"message":"obsolete selection"}',{status:503}):response;
   };
  },lateFailure);
  await openPicker();
  await revision.fill(earlier);
  await revision.press('Enter');
  await page.waitForFunction(()=>window.revisionWaiting);
  const abandonedToken=await page.evaluate(()=>window.abandonedRevision);
  assert.ok(abandonedToken);
  assert.equal(confirmed.has(abandonedToken),false);
  await revision.press('Escape');
  await dialog.waitFor({state:'hidden'});
  assert.equal(await restoreFocus.evaluate(node=>node===document.activeElement),true);
  await openPicker();
  await revision.fill('review-base');
  await revision.press('Enter');
  await dialog.waitFor({state:'hidden'});
  await atCommit(head);
  await page.evaluate(()=>{window.revisionDelivered=false;window.releaseRevision();});
  await page.waitForFunction(()=>window.revisionDelivered);
  await paint(page);
  assert.equal(confirmed.has(abandonedToken),false);
  assert.equal(await label.textContent(),head.slice(0,8));
  assert.equal(await page.getByText('obsolete selection',{exact:true}).count(),0);
  assert.equal(await before.count(),1);
 }

 await bubble.getByRole('button',{name:'Return to live',exact:true}).click();
 await page.waitForFunction(()=>!document.querySelector('[data-name="target"] button[aria-label="Open in VS Code"]').disabled);
 assert.match(await helper.locator('.code-pane').textContent(),/historical_helper/);
 assert.deepEqual(errors,[]);
 console.log('function revision: keyboard selection, unchanged commit, moved refs, traversal, comparison, captured calls and cancelled replies');
}finally{
 await chromium?.close();
 await host?.close();
 await fixture.close();
}
