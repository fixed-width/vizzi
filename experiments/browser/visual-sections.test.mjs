import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {buildHost,browser,startHost,workspace,paint} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const boundary=['    let value = Some(\n','        tail(),\n','    );\n'];
const source='pub fn long() {\n'+Array.from({length:630},(_,i)=>boundary[i-94]??`    // context ${i}\n`).join('')+'    tail();\n}\npub fn tail() {}\n';
const input=await workspace({files:{'long.rs':source}});let host,chromium,release;
try{
 host=await startHost(input,1);chromium=await browser({desktopBus:false});const page=await chromium.newPage();let captured=false;
 await page.route('**/api/window?**',async route=>{const url=new URL(route.request().url());if(!captured&&url.searchParams.get('line_start')==='193'){const response=await route.fetch();captured=true;await new Promise(resolve=>{release=resolve;});await route.fulfill({response}).catch(()=>{});}else await route.continue();});
 await page.goto(host.url+'/workspace');await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));await page.getByRole('button',{name:'Add bubble',exact:true}).click();await page.getByLabel('Find a function',{exact:true}).fill('long');await page.getByRole('button',{name:'Find functions',exact:true}).click();await page.locator('[data-add-symbol][data-name="long"]').click();const fragment=page.locator('[data-view][data-name="long"]');await fragment.locator('.code-line').first().waitFor();assert.equal(await fragment.locator('.code-line').count(),96);assert.match(await fragment.locator('.source-section').innerText(),/Lines 1–96/);
 let releaseCollapsed;
 const collapsedGate=new Promise(resolve=>releaseCollapsed=resolve);
 const holdCollapsed=async route=>{
  if(new URL(route.request().url()).searchParams.get('line_start')!=='97')return route.fallback();
  const response=await route.fetch();await collapsedGate;await route.fulfill({response}).catch(()=>{});
 };
 await page.route('**/api/window?**',holdCollapsed);
 try{
  const requested=page.waitForRequest(request=>new URL(request.url()).searchParams.get('line_start')==='97');
  await fragment.getByRole('button',{name:'Next section',exact:true}).click();await requested;
  await fragment.getByRole('button',{name:'Collapse bubble',exact:true}).click();releaseCollapsed();
  await fragment.locator('.code-line[data-line="97"]').waitFor({state:'attached'});
  assert.match(await fragment.locator('.source-section').textContent(),/Lines 97–192/,'a collapsed jump updates the section label');
 }finally{releaseCollapsed();await page.unroute('**/api/window?**',holdCollapsed);}
 await page.reload();await fragment.getByRole('button',{name:'Expand bubble',exact:true}).click();
 await fragment.locator('.code-line[data-line="97"]').waitFor();
 assert.equal(await fragment.locator('.code-line').first().getAttribute('data-line'),'97','a collapsed jump saves the destination section');
 assert.equal(await fragment.locator('.code-line').count(),96);
 await fragment.getByRole('button',{name:'Next section',exact:true}).click();for(let i=0;!captured&&i<500;i++)await page.waitForTimeout(10);assert.equal(captured,true,'later section request was captured');await fragment.getByRole('button',{name:'Previous section',exact:true}).click();await page.waitForFunction(()=>document.querySelector('.source-section').textContent.includes('Lines 1–96'));release();await page.waitForTimeout(100);await paint(page);assert.match(await fragment.locator('.source-section').innerText(),/Lines 1–96/);assert.equal(await fragment.locator('.code-line').first().getAttribute('data-line'),'1');
 const pane=fragment.locator('.code-pane'),rows=pane.locator('.code-line');
 const row=number=>pane.locator(`[data-line="${number}"]`);
 const scrollDown=()=>pane.evaluate(element=>{element.scrollTop=element.scrollHeight-element.clientHeight-10;});
 let boundaryRow=await row(96).elementHandle();
 const selected=await row(96).locator('.code-text').evaluate(element=>{getSelection().selectAllChildren(element);return getSelection().toString();});
 await scrollDown();await row(97).waitFor({timeout:5000});
 assert.equal(await boundaryRow.evaluate(node=>node.isConnected),true,'loading a neighbour keeps the earlier boundary line mounted');
 assert.equal(await page.evaluate(()=>getSelection().toString()),selected,'an existing selection survives an adjacent read');
 const copied=await pane.evaluate(element=>{
  const range=document.createRange();range.setStartBefore(element.querySelector('[data-line="96"] .code-text'));range.setEndAfter(element.querySelector('[data-line="97"] .code-text'));
  const selection=getSelection();selection.removeAllRanges();selection.addRange(range);
  const clipboardData=new DataTransfer();element.dispatchEvent(new ClipboardEvent('copy',{clipboardData,bubbles:true}));selection.removeAllRanges();return clipboardData.getData('text/plain');
 });
 assert.match(copied,/    let value = Some\(\n        tail\(\),/);
 await row(96).evaluate(element=>{element.closest('.code-pane').scrollTop=element.offsetTop+3;});
 await page.waitForFunction(()=>Object.keys(localStorage).filter(key=>key.startsWith('vizzi.visual.tasks.v1.')).some(key=>{
  const shelf=JSON.parse(localStorage[key]),layout=JSON.parse(shelf.tasks.find(task=>task.id===shelf.active).layout);
  return layout.views.some(view=>view.symbol.name==='long'&&view.reading.anchor?.line===96);
 }));
 await page.reload();await row(97).waitFor({timeout:5000});
 await page.waitForFunction(()=>{
  const row=document.querySelector('[data-name="long"] [data-line="96"]');
  return row&&Math.abs(row.closest('.code-pane').scrollTop-row.offsetTop-3)<=1;
 });
 boundaryRow=await row(96).elementHandle();
 const call=row(97).getByRole('button',{name:'Actions for tail',exact:true});
 const originalCall=await call.getAttribute('data-call-id');
 let failedActions=0;
 const failAdjacentActions=route=>{
  const query=route.request().postDataJSON()?.query;
  if(query?.kind!=='actions'||query.line_start!==97)return route.fallback();
  failedActions++;
  return route.fulfill({status:503,contentType:'application/json',body:JSON.stringify({message:'fixture call metadata unavailable'})});
 };
 await page.route('**/api/structure',failAdjacentActions);
 try{
  await writeFile(join(input.root,'other.rs'),'fn independent() {}\n');
  await page.getByText('Live refresh is waiting. Earlier source is retained.',{exact:true}).waitFor({timeout:8000});
  assert.ok(failedActions>=6,'the neighbouring read exhausted its transport retries');
  assert.equal(await call.getAttribute('data-call-id'),originalCall,'a failed metadata read retains the previous inline control');
 }finally{await page.unroute('**/api/structure',failAdjacentActions);}
 await page.waitForFunction(id=>{
  const button=document.querySelector('[data-name="long"] [data-line="97"] .code-call');
  return button&&button.dataset.callId!==id&&button.getAttribute('aria-disabled')==='false';
 },originalCall);
 assert.equal(await boundaryRow.evaluate(node=>node.isConnected),true,'unchanged source retains existing rows through a live refresh');
 await call.click({modifiers:['Control']});await page.locator('[data-view][data-name="tail"] .code-line').first().waitFor();
 for(const end of [288,384]){await scrollDown();await row(end).waitFor();}
 await row(96).locator('.code-text').evaluate(element=>getSelection().selectAllChildren(element));
 await scrollDown();await fragment.getByText('Clear the selection to load more source.',{exact:true}).waitFor();
 assert.equal(await row(480).count(),0,'a selection protects the loaded rows at the retention limit');
 await page.evaluate(()=>getSelection().removeAllRanges());
 await pane.evaluate(element=>element.dispatchEvent(new WheelEvent('wheel',{deltaY:40})));
 await row(480).waitFor();
 for(const end of [576,633]){await scrollDown();await row(end).waitFor();assert.ok(await rows.count()<=384);}
 assert.equal(await row(96).count(),0,'off-screen rows are recycled');
 const first=Number(await rows.first().getAttribute('data-line'));
 await pane.evaluate(element=>{element.scrollTop=0;});await row(first-1).waitFor();await row(first).waitFor();
 const numbers=await rows.evaluateAll(elements=>elements.map(element=>Number(element.dataset.line)));
 assert.ok(numbers.length<=384);assert.deepEqual(numbers,Array.from({length:numbers.length},(_,i)=>numbers[0]+i));
 console.log('visual sections browser: continuous expression reading and copying, bounded rows, reverse scrolling and superseded section jumps');
}finally{release?.();await chromium?.close();await host?.close();await input.close();}
