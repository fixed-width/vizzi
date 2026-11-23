import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {unlink,writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {browser,buildHost,paint,startHost,workspace} from './support.mjs';

if(!process.argv.includes('--no-build'))buildHost();
const input=await workspace({files:{
 'py/caller.py':'def target():\n    return helper()\n',
 'py/worker.py':'def helper():\n    target()\n    return "historical_python"\n',
 'py/alternate.py':'def helper():\n    return "other_candidate"\n',
 'py/archived.py':'def archived():\n    pass\n',
 'py/ambiguous.py':'class First:\n    def helper(self):\n        return "first_choice"\n\nclass Second:\n    def helper(self):\n        return "second_choice"\n',
 'js/caller.js':'function targetJs() { jsHelper(); return "historical_caller"; }\n',
 'js/worker.js':'function jsHelper() { return "historical_js"; }\n',
}});
function git(...args){
 const result=spawnSync('git',['-C',input.root,...args],{encoding:'utf8'});
 assert.equal(result.status,0,result.stderr);
 return result.stdout.trim();
}
function commit(){
 git('add','.');
 git('-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','fixture');
 return git('rev-parse','HEAD');
}

let host,chromium;
try{
 git('init','-q');
 const old=commit();
 await unlink(join(input.root,'py/archived.py'));
 await writeFile(join(input.root,'py/live_only.py'),'def live_only():\n    pass\n');
 await writeFile(join(input.root,'py/worker.py'),'def helper():\n    return "live_python"\n');
 await writeFile(join(input.root,'js/caller.js'),'function targetJs() { jsHelper(); return "live_caller"; }\n');
 await writeFile(join(input.root,'js/worker.js'),'function jsHelper() { return "live_js"; }\n');
 commit();
 host=await startHost(input,1);
 chromium=await browser({desktopBus:false});
 const page=await chromium.newPage({viewport:{width:1580,height:1030}}),errors=[];
 page.on('pageerror',error=>errors.push(error.message));
 await page.goto(host.url+'/workspace');
 await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
 async function add(name,path){
  await page.getByRole('button',{name:'Add bubble',exact:true}).click();
  await page.getByLabel('Find a function',{exact:true}).fill(name);
  await page.getByLabel('Source file path (optional)',{exact:true}).fill(path);
  await page.getByRole('button',{name:'Find functions',exact:true}).click();
  await page.locator(`[data-add-symbol][data-name="${name}"]`).click();
  const bubble=page.locator(`[data-view][data-name="${name}"]`).first();
  await bubble.locator('.code-line').first().waitFor();
  return bubble;
 }
 const live=await add('helper','py/worker.py');
 assert.match(await live.locator('.code-pane').textContent(),/live_python/);
 const caller=await add('target','py/caller.py');
 await caller.getByRole('button',{name:'Choose revision…',exact:true}).click();
 const revision=page.getByLabel('Local commit ID, branch or tag');
 await revision.fill(old);
 await revision.press('Enter');
 await page.getByRole('dialog',{name:'Choose revision'}).waitFor({state:'hidden'});
 const rect=await caller.evaluate(node=>node.style.cssText);
 const zoom=await page.locator('#visual-zoom').textContent();
 const dialog=page.getByRole('dialog',{name:'Look in another file'});
 const path=dialog.getByLabel('Repository-relative file path');
 async function inline(bubble,name,keyboard=false){
  const control=bubble.getByRole('button',{name:`Actions for ${name}`,exact:true});
  const item=page.getByRole('menuitem',{name:'Look in another file…',exact:true});
  if(keyboard){await control.press('Enter');await item.press('Enter');}
  else{await control.click();await item.click();}
  assert.match(await dialog.locator('#cross-file-commit').textContent(),new RegExp(old));
 }
 async function chooseFile(file){
  await path.fill(file);
  await path.press('Enter');
 }
 await inline(caller,'helper');
 await dialog.getByRole('button',{name:'Repository root',exact:true}).click();
 await dialog.getByRole('button',{name:'Browse py',exact:true}).click();
 await dialog.getByRole('button',{name:'Find candidates in py/archived.py',exact:true}).waitFor();
 assert.equal(await dialog.getByRole('button',{name:'Find candidates in py/live_only.py',exact:true}).count(),0);
 await dialog.getByRole('button',{name:'Find candidates in py/worker.py',exact:true}).click();
 const nameCandidate=dialog.getByRole('button',{name:'Open candidate helper',exact:true});
 await nameCandidate.waitFor();
 assert.match(await dialog.getByRole('status').textContent(),/1 name candidate/);
 assert.equal(await page.locator('[data-view][data-name="helper"]').count(),1);
 const endpoint='/api/captured-functions';
 const helper=page.locator('[data-view][data-name="helper"][data-captured-file]');
 const isQuery=(request,kind)=>new URL(request.url()).pathname===endpoint&&request.postDataJSON()?.query.kind===kind;
 const holdConfirmation=Promise.withResolvers(),startConfirmation=Promise.withResolvers();
 const hold=async route=>{
  if(!isQuery(route.request(),'confirm_file'))return route.continue();
  startConfirmation.resolve();
  await holdConfirmation.promise;
  await route.continue().catch(()=>{});
 };
 await page.route('**'+endpoint,hold);
 try{
  const opening=page.waitForRequest(request=>isQuery(request,'open_file'));
  await nameCandidate.click();
  await startConfirmation.promise;
  assert.equal(await helper.count(),0,'the callee stays provisional while confirmation is pending');
  assert.equal(await page.locator('#workspace-links .link-stroke').count(),0);
  const {source,query:{call}}=(await opening).postDataJSON();
  const otherPath='py/alternate.py';
  const choices=await page.request.post(host.url+endpoint,{data:{source,query:{kind:'candidates',call,path:otherPath,offset:0}}});
  assert.ok(choices.ok(),await choices.text());
  const competing=await choices.json();
  const replacement=await page.request.post(host.url+endpoint,{data:{source,query:{kind:'open_file',call,path:otherPath,snapshot:competing.snapshot,target:competing.items[0].id}}});
  assert.ok(replacement.ok(),await replacement.text());
  const confirmation=page.waitForResponse(response=>isQuery(response.request(),'confirm_file'));
  holdConfirmation.resolve();
  assert.equal((await confirmation).status(),409);
  await dialog.getByRole('alert').waitFor();
  assert.equal(await helper.count(),0);
  assert.equal(await page.locator('#workspace-links .link-stroke').count(),0);
  await page.reload();
  await caller.locator('.code-line').first().waitFor();
  assert.equal(await helper.count(),0,'reloading restores only confirmed source');
  assert.equal(await page.locator('#workspace-links .link-stroke').count(),0);
 }finally{
  holdConfirmation.resolve();
  await page.unroute('**'+endpoint,hold);
 }

 await inline(caller,'helper',true);
 await page.keyboard.press('Escape');
 const inlineControl=caller.getByRole('button',{name:'Actions for helper',exact:true});
 assert.equal(await inlineControl.evaluate(button=>button===document.activeElement),true,'Escape returns focus to the inline call');
 const callsControl=caller.getByRole('button',{name:'Calls',exact:true});
 await callsControl.press('Enter');
 await caller.getByRole('button',{name:'Look for helper in another file',exact:true}).press('Enter');
 await dialog.getByRole('button',{name:'Close',exact:true}).press('Enter');
 assert.equal(await callsControl.evaluate(button=>button===document.activeElement),true,'Close returns focus to the Calls button');

 await inline(caller,'helper');
 await chooseFile('py/worker.py');
 const confirmed=page.waitForResponse(response=>isQuery(response.request(),'confirm_file'));
 await nameCandidate.click();
 assert.equal((await confirmed).ok(),true);
 await dialog.waitFor({state:'hidden'});
 assert.match(await helper.locator('.code-pane').textContent(),/historical_python/);
 assert.match(await live.locator('.code-pane').textContent(),/live_python/);
 assert.equal(await caller.evaluate(node=>node.style.cssText),rect);
 assert.equal(await page.locator('#visual-zoom').textContent(),zoom);
 await helper.getByRole('button',{name:'Calls',exact:true}).click();
 await helper.getByRole('button',{name:'Look for target in another file',exact:true}).click();
 await chooseFile('py/caller.py');
 await dialog.getByRole('button',{name:'Open candidate target',exact:true}).click();
 await dialog.waitFor({state:'hidden'});
 assert.equal(await caller.count(),1,'a cross-file cycle reuses the pinned caller');
 await page.reload();
 await helper.locator('.code-line').first().waitFor();
 assert.match(await helper.locator('.code-pane').textContent(),/historical_python/);
 assert.equal(await page.locator('#workspace-links .link-stroke').count(),2);

 for(const lateFailure of [false,true]){
  await page.evaluate(failure=>{
   const fetch=window.fetch;
   window.crossWaiting=false;
   window.fetch=async(path,options)=>{
    if(path!=='/api/captured-functions'||JSON.parse(options.body).query.kind!=='open_file')return fetch(path,options);
    window.fetch=fetch;
    const response=await fetch(path,{...options,signal:undefined});
    await new Promise(resolve=>{window.releaseCross=resolve;window.crossWaiting=true;});
    window.crossDelivered=true;
    return failure?new Response('{"message":"obsolete cross-file failure"}',{status:500}):response;
   };
  },lateFailure);
  await inline(caller,'helper');
  await chooseFile('py/alternate.py');
  await nameCandidate.click();
  await page.waitForFunction(()=>window.crossWaiting);
  await page.keyboard.press('Escape');
  await dialog.waitFor({state:'hidden'});
  await page.evaluate(()=>{window.crossDelivered=false;window.releaseCross();});
  await page.waitForFunction(()=>window.crossDelivered);
  await paint(page);
  assert.equal(await page.getByText('obsolete cross-file failure',{exact:true}).count(),0);
  assert.equal(await helper.count(),1);
  assert.match(await helper.locator('.code-pane').textContent(),/historical_python/);
 }

 await helper.getByRole('button',{name:'Return to live',exact:true}).click();
 await helper.waitFor({state:'detached'});
 assert.equal(await page.locator('[data-view][data-name="helper"]').count(),2);
 assert.match(await caller.locator('.code-pane').textContent(),/return helper/);
 assert.match(await page.locator('#workspace-links .link-stroke').first().getAttribute('aria-label'),/Earlier call/);

 await inline(caller,'helper');
 await chooseFile('py/ambiguous.py');
 const firstChoice=dialog.getByRole('button',{name:'Open candidate First::helper',exact:true});
 await firstChoice.waitFor();
 assert.match(await dialog.getByRole('status').textContent(),/2 name candidates/);
 assert.equal(await dialog.getByRole('button',{name:'Open candidate Second::helper',exact:true}).count(),1);
 assert.equal(await page.locator('[data-view][data-captured-file]').count(),0);
 await firstChoice.click();
 await dialog.waitFor({state:'hidden'});
 assert.match(await helper.locator('.code-pane').textContent(),/first_choice/);

 const js=await add('targetJs','js/caller.js');
 await js.getByRole('button',{name:'Compare with previous version',exact:true}).click();
 const before=page.locator('[data-comparison-side="older"]'),after=page.locator('[data-comparison-side="newer"]');
 await after.getByRole('button',{name:'Actions for jsHelper',exact:true}).click();
 await page.getByRole('menuitem',{name:'Open in captured file'}).waitFor();
 assert.equal(await page.getByRole('menuitem',{name:'Look in another file…',exact:true}).count(),0);
 await page.keyboard.press('Escape');
 await inline(before,'jsHelper');
 await chooseFile('js/worker.js');
 await dialog.getByRole('button',{name:'Open candidate jsHelper',exact:true}).click();
 await dialog.waitFor({state:'hidden'});
 assert.match(await page.locator('[data-view][data-name="jsHelper"] .code-pane').textContent(),/historical_js/);
 assert.deepEqual(errors,[]);
 console.log('historical cross-file calls: mixed languages, explicit candidates, captured paths, cycles, reload, comparison and late replies');
}finally{
 await chromium?.close();
 await host?.close();
 await input.close();
}
