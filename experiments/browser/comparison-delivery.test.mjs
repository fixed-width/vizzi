import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {buildHost,browser,startHost,workspace,paint} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const input=await workspace({files:{'lib.rs':'fn leaf() {}\nfn caller() {}\n'}});let host,chrome;
const git=(...args)=>{const result=spawnSync('git',['-C',input.root,...args],{encoding:'utf8'});assert.equal(result.status,0,result.stderr);};
function commit(){git('add','lib.rs');git('-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','fixture');}
try{
 git('init','-q');commit();await writeFile(join(input.root,'lib.rs'),'fn leaf() {}\nfn caller() { leaf(); }\n');commit();
 host=await startHost(input,1);chrome=await browser({desktopBus:false});
 for(const scenario of [{ignoreAbort:false,failure:false},{ignoreAbort:true,failure:false},{ignoreAbort:true,failure:true}]){
  const page=await chrome.newPage({viewport:{width:1500,height:1100}}),errors=[];let release;
  try{
   page.setDefaultTimeout(10000);page.on('pageerror',error=>errors.push(error.message));
   if(scenario.ignoreAbort)await page.addInitScript(()=>{const request=globalThis.fetch.bind(globalThis);globalThis.fetch=(path,options)=>String(path).split('?')[0]==='/api/function-comparisons'?request(path,{...options,signal:undefined}):request(path,options);});
   await page.goto(host.url+'/workspace');await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
   await page.getByRole('button',{name:'Add bubble',exact:true}).click();await page.getByLabel('Find a function',{exact:true}).fill('caller');await page.getByRole('button',{name:'Find functions',exact:true}).click();await page.locator('[data-add-symbol][data-name="caller"]').click();const caller=page.locator('[data-view][data-name="caller"]');await caller.locator('.code-line').first().waitFor();
   const status=await page.request.get(host.url+'/api/status').then(response=>response.json());
   const inventory=await page.request.post(host.url+'/api/structure',{data:{snapshot:status.snapshot,query:{kind:'inventory',area:'lib.rs',search:'caller',offset:0,functions_only:true}}}).then(response=>response.json());const symbol=inventory.items.find(row=>row.symbol.name==='caller').symbol;
   const calls=await page.request.post(host.url+'/api/structure',{data:{snapshot:status.snapshot,query:{kind:'calls',symbol:symbol.id,area:'lib.rs',offset:0}}}).then(response=>response.json());const call=calls.items[0].id;
   let entered,returned,settled;const waiting=new Promise(resolve=>entered=resolve),gate=new Promise(resolve=>release=resolve),finished=new Promise(resolve=>returned=resolve),network=new Promise(resolve=>settled=resolve);
   page.on('response',response=>{if(new URL(response.url()).pathname==='/api/function-comparisons')void response.finished().then(settled,settled);});page.on('requestfailed',request=>{if(new URL(request.url()).pathname==='/api/function-comparisons')settled();});
   await page.route('**/api/function-comparisons',async route=>{const response=await route.fetch();assert.equal(response.ok(),true);entered();await gate;try{await route.fulfill(scenario.failure?{status:500,contentType:'application/json',body:JSON.stringify({message:'Obsolete comparison failed'})}:{response});}catch{}finally{returned();}});
   await caller.getByRole('button',{name:'Compare with previous version',exact:true}).click();await waiting;
   await page.evaluate(hash=>{location.hash=hash;},'#editor-call='+call+'&snapshot='+status.snapshot);const leaf=page.locator('[data-view][data-name="leaf"]');await leaf.locator('.code-line').first().waitFor();
   const camera=await page.locator('#workspace-world').getAttribute('style');release();await finished;await network;await paint(page);
   assert.equal(await page.locator('[data-comparison-side]').count(),0,JSON.stringify(scenario));assert.equal(await page.locator('[data-view]').count(),2);assert.equal(await leaf.evaluate(node=>node.classList.contains('is-selected')),true);assert.equal(await page.locator('#workspace-world').getAttribute('style'),camera);assert.equal(await caller.getByRole('button',{name:'Compare with previous version',exact:true}).isEnabled(),true);assert.equal(await caller.getByRole('button',{name:'Open in VS Code',exact:true}).isEnabled(),true);assert.equal(await page.locator('#visual-error').isVisible(),false);assert.deepEqual(errors,[]);
  }finally{release?.();await page.close();}
 }
 console.log('comparison delivery: newer Explore Call cancels comparison preparation; late success/failure cannot pin its caller, add a pair, steal focus or leave loading controls');
}finally{await chrome?.close();await host?.close();await input.close();}
