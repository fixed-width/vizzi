import assert from 'node:assert/strict';
import {buildHost,browser,startHost,workspace} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const input=await workspace({files:{'a.py':'def first_leaf(): pass\ndef first(): first_leaf()\ndef second_leaf(): pass\ndef second(): second_leaf()\n'}});let host,chrome;
try {
 host=await startHost(input,1);chrome=await browser({desktopBus:false});
 const post=async(path,body)=>{const response=await fetch(host.url+path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)});assert.ok(response.ok,await response.clone().text());return response.json();};
 await post('/api/workspace/start',{});let status;const deadline=Date.now()+10000;
 do{status=await fetch(host.url+'/api/status').then(r=>r.json());assert.ok(Date.now()<deadline);if(status.pending)await new Promise(r=>setTimeout(r,20));}while(status.pending);
 async function call(name){const functions=await post('/api/structure',{snapshot:status.snapshot,query:{kind:'inventory',area:'a.py',search:name,offset:0,functions_only:true}});const symbol=functions.items.map(x=>x.symbol).find(s=>s.name===name);assert.ok(symbol);const result=await post('/api/structure',{snapshot:status.snapshot,query:{kind:'calls',symbol:symbol.id,area:'a.py',offset:0}});return result.items[0].id;}
 const first=await call('first'),second=await call('second');
 const link=call=>'#editor-call='+call+'&snapshot='+status.snapshot;
 async function page(){const value=await chrome.newPage();value.setDefaultTimeout(10000);value.errors=[];value.on('pageerror',error=>value.errors.push(error.message));await value.goto(host.url);await value.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));return value;}
 const newest=await page();let held,release;const blocked=new Promise(r=>{held=r;});
 await newest.route('**/api/editor/call',async route=>{if(route.request().postDataJSON().query.call===first){await new Promise(r=>{release=r;held();});await route.fulfill({status:500,body:'obsolete failure'}).catch(()=>{});}else await route.continue();});
 await newest.evaluate(hash=>{location.hash=hash;},link(first));await blocked;
 await newest.evaluate(hash=>{location.hash=hash;},link(second));await newest.locator('[data-name="second_leaf"] .code-line').first().waitFor();release();
 await newest.waitForTimeout(100);assert.equal(await newest.locator('[data-name="first"]').count(),0);assert.equal(await newest.locator('#visual-error').isVisible(),false);assert.deepEqual(newest.errors,[]);await newest.close();
 const manual=await page();let manualRelease,manualStarted;const waiting=new Promise(r=>{manualStarted=r;});
 await manual.route('**/api/editor/call',async route=>{await new Promise(r=>{manualRelease=r;manualStarted();});await route.fulfill({status:500,body:'cancelled lookup failed'}).catch(()=>{});});
 await manual.evaluate(hash=>{location.hash=hash;},link(first));await waiting;
 await manual.getByRole('button',{name:'Add bubble',exact:true}).click();await manual.getByLabel('Find a function',{exact:true}).fill('second');await manual.getByRole('button',{name:'Find functions',exact:true}).click();await manual.locator('[data-add-symbol][data-name="second"]').click();manualRelease();
 await manual.locator('[data-name="second"] .code-line').first().waitFor();await manual.waitForTimeout(100);assert.equal(await manual.locator('[data-name="first"]').count(),0);assert.equal(await manual.locator('#visual-error').isVisible(),false);assert.deepEqual(manual.errors,[]);await manual.close();
 console.log('browser delivery: newest legacy action and newer manual navigation cancel stale preparation and its failure');
} finally {await chrome?.close();await host?.close();await input.close();}
