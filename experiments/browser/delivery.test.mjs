import assert from 'node:assert/strict';
import {buildHost,browser,startHost,workspace} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const input=await workspace({files:{'a.py':'def first_leaf(): pass\ndef first(): first_leaf()\ndef second_leaf(): pass\ndef second(): second_leaf()\n'}});let host,chrome;

async function verifyShallowReveal() {
 const tab=await chrome.newPage({viewport:{width:1300,height:500}});
 try {
  const fixture=host.url+'/editor-fixture';
  await tab.route(fixture,route=>route.fulfill({contentType:'text/html',body:'<iframe title="Vizzi" width="1250" height="120"></iframe>'}));
  for(const path of ['/present','/explore'])await tab.route(host.url+path,async route=>{
   const response=await route.fetch();
   const headers=response.headers();
   headers['content-security-policy']=headers['content-security-policy'].replace(/; frame-ancestors [^;]+/,"; frame-ancestors 'self'");
   await route.fulfill({response,headers});
  });
  await tab.addInitScript(()=>document.addEventListener('DOMContentLoaded',()=>{
   const source=document.getElementById('source');
   if(source)source.style.height='0';
  }));
  await tab.goto(fixture);
  await tab.evaluate(async()=>{
   const {createDeliveryClient}=await import('/workspace-delivery.mjs');
   const frame=document.querySelector('iframe');
   let port;
   const client=createDeliveryClient({send:message=>port.postMessage(message),timeout:4000});
   window.addEventListener('message',event=>{
    const {protocol,version,type,document}=event.data ?? {};
    if(event.source!==frame.contentWindow || event.origin!==location.origin || protocol!=='vizzi-delivery-connect' || type!=='hello')return;
    port?.close();
    const channel=new MessageChannel();
    port=channel.port1;
    port.onmessage=message=>client.receive(message.data);
    frame.contentWindow.postMessage({protocol,version,type:'attach',document,challenge:crypto.randomUUID()},location.origin,[channel.port2]);
   });
   window.revealed=client.deliver({kind:'reveal'}).catch(error=>({error:error.message}));
   frame.src='/present';
  });
  const frame=tab.frameLocator('iframe');
  const selected=frame.getByRole('link',{name:'Source browser',exact:true});
  await selected.waitFor({timeout:3000});
  assert.equal(await selected.getAttribute('aria-current'),'page');
  assert.equal(await frame.locator('#source').evaluate(source=>source.clientHeight),0);
  assert.deepEqual(await tab.evaluate(()=>window.revealed),{state:'applied'});
 } finally {await tab.close();}
}

try {
 host=await startHost(input,1);chrome=await browser({desktopBus:false});
 await verifyShallowReveal();
 const post=async(path,body)=>{const response=await fetch(host.url+path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)});assert.ok(response.ok,await response.clone().text());return response.json();};
 await post('/api/workspace/start',{});let status;const deadline=Date.now()+10000;
 do{status=await fetch(host.url+'/api/status').then(r=>r.json());assert.ok(Date.now()<deadline);if(status.pending)await new Promise(r=>setTimeout(r,20));}while(status.pending);
 async function call(name){const functions=await post('/api/structure',{snapshot:status.snapshot,query:{kind:'inventory',area:'a.py',search:name,offset:0,functions_only:true}});const symbol=functions.items.map(x=>x.symbol).find(s=>s.name===name);assert.ok(symbol);const result=await post('/api/structure',{snapshot:status.snapshot,query:{kind:'calls',symbol:symbol.id,area:'a.py',offset:0}});return result.items[0].id;}
 const first=await call('first'),second=await call('second');
 const link=call=>'#editor-call='+call+'&snapshot='+status.snapshot;
 const direct=await chrome.newPage(),directErrors=[];
 direct.on('pageerror',error=>directErrors.push(error.message));
 await direct.goto(host.url+'/'+link(first));
 assert.deepEqual(directErrors,[]);
 await direct.locator('[data-name="first_leaf"] .code-line').first().waitFor({timeout:10000});
 assert.equal(new URL(direct.url()).pathname,'/workspace');
 assert.equal(new URL(direct.url()).hash,'');
 assert.deepEqual(directErrors,[]);
 await direct.close();
 async function page(){const value=await chrome.newPage();value.setDefaultTimeout(10000);value.errors=[];value.on('pageerror',error=>value.errors.push(error.message));await value.goto(host.url+'/workspace');await value.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));return value;}
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
 console.log('browser delivery: shallow editor reveal, newest legacy action and manual navigation cancellation');
} finally {await chrome?.close();await host?.close();await input.close();}
