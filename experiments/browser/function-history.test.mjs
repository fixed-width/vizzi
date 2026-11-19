import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {writeFile} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {buildHost,browser,startHost,workspace,paint} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const source=(version,other='other_current')=>`fn target() {\n    other();\n    let ${version} = 1;\n${Array.from({length:110},(_,i)=>`    // retained line ${i}`).join('\n')}\n}\nfn other() { let ${other} = 1; }\n`;
const input=await workspace({files:{'lib.rs':source('old_version'),'Cargo.toml':"[package]\nname='history_fixture'\n[lib]\npath='lib.rs'\n"}});
if(process.argv.includes('--debug-host'))input.executable=resolve('target/debug/vizzi');
const git=(...args)=>{const out=spawnSync('git',['-C',input.root,...args],{encoding:'utf8'});assert.equal(out.status,0,out.stderr);return out.stdout.trim();};
async function commit(text){await writeFile(join(input.root,'lib.rs'),text);git('add','lib.rs');git('-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','fixture');return git('rev-parse','HEAD');}
let host,chromium;
try{
 git('init','-q');git('add','Cargo.toml');const old=await commit(source('old_version'));const middle=await commit(source('middle_version'));await commit(source('middle_version','unrelated'));await commit(source('newest_version','unrelated'));
 host=await startHost(input,1);chromium=await browser({desktopBus:false});const page=await chromium.newPage({viewport:{width:1500,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(host.url+'/workspace');await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
 await page.getByRole('button',{name:'Add bubble',exact:true}).click();await page.getByLabel('Find a function',{exact:true}).fill('target');await page.getByRole('button',{name:'Find functions',exact:true}).click();await page.locator('[data-add-symbol][data-name="target"]').click();const root=page.locator('[data-view][data-name="target"]');await root.locator('.code-line').first().waitFor();
 await root.getByRole('button',{name:'Actions for other',exact:true}).click({modifiers:['Control']});const other=page.locator('[data-view][data-name="other"]');await other.locator('.code-line').first().waitFor();const otherNode=await other.locator('.code-text').elementHandle();
 await page.locator('#workspace-settings > summary').click();
 await page.getByLabel('Module outlines',{exact:true}).check();
 await page.keyboard.press('Escape');
 const outline=page.locator('#module-outlines .module-outline');
 await outline.waitFor();
 const moduleId=await other.getAttribute('data-module-id');
 const geometry=(view=root)=>view.evaluate(n=>({id:n.dataset.view,left:n.style.left,top:n.style.top,width:n.style.width,height:n.style.height}));const before=await geometry();
 await root.getByRole('button',{name:'Previous function change',exact:true}).click();await page.waitForFunction(()=>document.querySelector('[data-name="target"] .code-pane').textContent.includes('middle_version'));
 const historyRow=root.locator('.fragment-history'),historyReturn=historyRow.getByRole('button',{name:'Return to live',exact:true});
 assert.equal(await root.locator('.fragment-history span').textContent(),middle.slice(0,8));
 assert.equal(await root.locator('.module-tag').textContent(),'history_fixture');
 assert.equal(await root.getAttribute('data-module-id'),moduleId);
 await outline.waitFor();
 assert.equal(await historyReturn.count(),1,'the history row offers a way to continue to live source');
 assert.equal(await historyReturn.isEnabled(),true);
 assert.equal(await root.getByRole('button',{name:'Return to live',exact:true}).count(),1,'only one return control is visible');
 assert.equal(await root.getByRole('button',{name:'Calls',exact:true}).isDisabled(),false);assert.equal(await root.getByRole('button',{name:'Open in VS Code'}).isDisabled(),true);assert.ok(await root.locator('.code-call').count()>0);assert.deepEqual(await geometry(),before);assert.equal(await otherNode.evaluate(n=>n.isConnected),true);assert.match(await page.locator('#workspace-links path.link-stroke').getAttribute('aria-label'),/Earlier call/);
 await paint(page);await page.screenshot({path:'/tmp/vizzi-function-history.png'});
 const [savedSection]=await Promise.all([page.waitForResponse(response=>{const url=new URL(response.url());return url.pathname==='/api/function-history/window'&&url.searchParams.get('line_start')==='97'&&response.status()===200;}),root.getByRole('button',{name:'Next section',exact:true}).click()]);await page.waitForFunction(()=>document.querySelector('[data-name="target"] .source-section').textContent.includes('97–'));
 const captured=new URL(savedSection.url()).searchParams,pin=Object.fromEntries(['token','snapshot','file','version'].map(key=>[key,captured.get(key)]));
 const displaced=await page.request.post(host.url+'/api/function-history/step',{data:{token:pin.token,index:1}});assert.equal(displaced.ok(),true);assert.equal((await displaced.json()).commit,old);
 const busySections=new Set(['1','97']),restoredSections=new Set(),readPins=[];
 const restoredRead=response=>{const url=new URL(response.url());if(url.pathname==='/api/function-history/window'&&response.status()===200)restoredSections.add(url.searchParams.get('line_start'));};page.on('response',restoredRead);
 await page.route('**/api/function-history/window?*',async route=>{const query=new URL(route.request().url()).searchParams;readPins.push(Object.fromEntries(Object.keys(pin).map(key=>[key,query.get(key)])));if(busySections.delete(query.get('line_start')))await route.fulfill({status:503,contentType:'application/json',body:JSON.stringify({message:'Function history is busy; retry'})});else await route.continue();});
 await page.reload();await Promise.race([root.locator('.code-line').first().waitFor(),root.getByText('Source for this bubble is unavailable.',{exact:true}).waitFor().then(()=>{throw Error('Retained history did not recover from busy concurrent reads');})]);assert.equal(busySections.size,0);assert.deepEqual([...restoredSections].sort(),['1','97']);for(const readPin of readPins)assert.deepEqual(readPin,pin);assert.match(await root.locator('.source-section').textContent(),/97–/);assert.equal(await root.locator('.fragment-history span').textContent(),middle.slice(0,8));assert.deepEqual(await geometry(),before);await page.unroute('**/api/function-history/window?*');page.off('response',restoredRead);
 await root.getByRole('button',{name:'Previous function change',exact:true}).click();await page.waitForFunction(hash=>document.querySelector('[data-name="target"] .fragment-history span').textContent===hash,old.slice(0,8));
 await root.getByRole('button',{name:'Next function change',exact:true}).click();await page.waitForFunction(hash=>document.querySelector('[data-name="target"] .fragment-history span').textContent===hash,middle.slice(0,8));
 await writeFile(join(input.root,'lib.rs'),source('saved_working_version','other_saved'));await page.waitForFunction(()=>document.querySelector('[data-name="other"] .code-pane').textContent.includes('other_saved'));assert.equal(await root.locator('.fragment-history span').textContent(),middle.slice(0,8));
 // A failed preparation retains the last accepted version and its source.
 await page.route('**/api/function-history/step',route=>route.fulfill({status:503,contentType:'application/json',body:JSON.stringify({message:'fixture history unavailable'})}));const retained=await root.locator('.code-pane').textContent();await root.getByRole('button',{name:'Previous function change',exact:true}).click();await page.getByText('fixture history unavailable',{exact:true}).waitFor();assert.equal(await root.locator('.code-pane').textContent(),retained);await page.unroute('**/api/function-history/step');
 // Returning live cancels an older in-flight reply, including after the server prepared it.
 let release,arrived;const waiting=new Promise(resolve=>arrived=resolve),gate=new Promise(resolve=>release=resolve);await page.route('**/api/function-history/step',async route=>{const response=await route.fetch();arrived();await gate;await route.fulfill({response}).catch(()=>{});});await root.getByRole('button',{name:'Previous function change',exact:true}).click();await waiting;
 await historyReturn.press('Enter');release();await page.waitForFunction(()=>!document.querySelector('[data-name="target"] button[aria-label="Open in VS Code"]').disabled);
 const compare=historyRow.getByRole('button',{name:'Compare with previous version',exact:true});
 assert.equal(await compare.evaluate(button=>document.activeElement===button),true,'keyboard focus continues to Compare');
 assert.match(await root.locator('.source-section').textContent(),/97–/);await root.getByRole('button',{name:'Previous section',exact:true}).click();await page.waitForFunction(()=>document.querySelector('[data-name="target"] .code-pane').textContent.includes('saved_working_version'));assert.deepEqual(await geometry(),before);assert.equal(await root.getByRole('button',{name:'Return to live',exact:true}).count(),0);
 await compare.press('Enter');
 const after=page.locator('[data-comparison-side="newer"]'),beforeView=page.locator('[data-comparison-side="older"]');
 await beforeView.locator('.comparison-removed').first().waitFor();
 assert.match(await beforeView.locator('.code-pane').textContent(),/newest_version/);
 assert.match(await after.locator('.code-pane').textContent(),/saved_working_version/);
 for(const side of [after,beforeView]){
  assert.equal(await side.locator('.module-tag').textContent(),'history_fixture');
  assert.equal(await side.getAttribute('data-module-id'),moduleId);
 }
 await outline.waitFor();
 assert.deepEqual(await geometry(after),before);
 await beforeView.getByRole('button',{name:'Close bubble',exact:true}).click();
 await after.getByRole('button',{name:'Return to live',exact:true}).click();
 await page.waitForFunction(()=>!document.querySelector('[data-name="target"] button[aria-label="Open in VS Code"]').disabled);
 await page.unroute('**/api/function-history/step');await root.getByRole('button',{name:'Previous function change',exact:true}).click();await page.waitForFunction(()=>document.querySelector('[data-name="target"] .code-pane').textContent.includes('newest_version'));
 await writeFile(join(input.root,'lib.rs'),source('saved_working_version','other_missing').replace('fn target()','fn renamed_target()'));await page.waitForFunction(()=>document.querySelector('[data-name="other"] .code-pane').textContent.includes('other_missing'));await root.getByRole('button',{name:'Return to live',exact:true}).click();await page.waitForFunction(()=>document.querySelector('[data-name="target"] .fragment-footer').textContent.includes('Function unavailable'));
 assert.equal(await page.locator('#visual-stale').isVisible(),true,'an unresolved result reports unavailable source');
 assert.match(await root.locator('.code-pane').textContent(),/newest_version/);assert.equal(await root.locator('.fragment-history span').textContent(),git('rev-parse','HEAD').slice(0,8));
 const previous=historyRow.getByRole('button',{name:'Previous function change',exact:true});
 let releaseRestoredRefresh;
 const restoredRefreshGate=new Promise(resolve=>releaseRestoredRefresh=resolve);
 await page.route('**/api/visual/refresh',async route=>{
  await restoredRefreshGate;
  await route.fulfill({status:500,contentType:'application/json',body:JSON.stringify({message:'fixture live refresh failed'})}).catch(()=>{});
 });
 try{
  const restoredRefresh=page.waitForRequest(request=>new URL(request.url()).pathname==='/api/visual/refresh');
  await page.reload();await restoredRefresh;
  await root.locator('.code-line').first().waitFor();
  assert.equal(await root.getByText('Waiting for live source…',{exact:true}).isVisible(),true,'restored intent is waiting rather than an active return');
  assert.match(await root.locator('.code-pane').textContent(),/newest_version/);
  assert.equal(await previous.isEnabled(),true,'background refresh leaves restored history navigable');
 }finally{releaseRestoredRefresh();}
 await page.getByText('Live refresh is waiting. Earlier source is retained.',{exact:true}).waitFor();
 assert.equal(await previous.isEnabled(),true,'restored history remains navigable while refresh fails');
 assert.equal(await historyReturn.isEnabled(),true,'restored history can retry the live lookup');
 await previous.click();
 await page.waitForFunction(()=>document.querySelector('[data-name="target"] .code-pane').textContent.includes('middle_version'));
 assert.equal(await root.getByText('Waiting for live source…',{exact:true}).count(),0,'history navigation clears the wait for live source');
 await historyRow.getByRole('button',{name:'Next function change',exact:true}).click();
 await historyReturn.press('Enter');
 await root.getByText('Waiting for live source…',{exact:true}).waitFor();
 assert.equal(await historyReturn.isEnabled(),true,'a failed refresh allows another manual return');
 assert.equal(await page.locator('#visual-stale').isVisible(),false,'transport failure does not report an unresolved declaration');
 await page.unroute('**/api/visual/refresh');
 await root.getByText(/^Function unavailable/).waitFor();
 assert.equal(await historyReturn.evaluate(button=>button===document.activeElement),true);
 await writeFile(join(input.root,'lib.rs'),source('saved_working_version','other_saved'));
 await page.waitForFunction(()=>!document.querySelector('[data-name="target"] button[aria-label="Open in VS Code"]').disabled);
 assert.match(await root.locator('.code-pane').textContent(),/saved_working_version/);
 assert.equal(await compare.evaluate(button=>button===document.activeElement),true,'keyboard focus continues to Compare after later recovery');
 assert.equal(await root.getByRole('button',{name:'Return to live',exact:true}).count(),0);assert.deepEqual(await geometry(),before);const grip=root.getByRole('button',{name:'Resize bubble',exact:true});for(let i=0;i<13;i++)await grip.press('ArrowUp');for(let i=0;i<8;i++)await grip.press('ArrowLeft');const outer=await root.boundingBox(),footer=await root.locator('.fragment-footer').boundingBox();assert.ok(footer.y+footer.height<=outer.y+outer.height+1,'the minimum bubble height keeps revision and source controls visible');await root.getByRole('button',{name:'Previous function change',exact:true}).click();await root.getByRole('button',{name:'Return to live',exact:true}).waitFor();const returning=await root.getByRole('button',{name:'Return to live',exact:true}).boundingBox();assert.ok(returning.y+returning.height<=outer.y+outer.height+1,'return to live remains visible at the minimum size');assert.deepEqual(errors,[]);
 await paint(page);console.log('function history: back/forward, busy restoration, paging/reload, stable bubbles, failed live refresh and retry, later recovery with keyboard focus, and supersession');
}finally{await chromium?.close();await host?.close();await input.close();}
