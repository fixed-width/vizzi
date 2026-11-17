import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {buildHost,browser,startHost,workspace,paint} from './support.mjs';

if(!process.argv.includes('--no-build'))buildHost();
const source=phase=>[
 'fn leaf() {}','fn caller() {','    leaf();',
 ...Array.from({length:280},(_,i)=>i===130||i===250?`    let edit_${i}=${phase};`:`    // context ${i}`),
 '}','',
].join('\n');
const input=await workspace({files:{'lib.rs':source(1)}});
let host,chrome;
const git=(...args)=>{
 const result=spawnSync('git',['-C',input.root,...args],{encoding:'utf8'});
 assert.equal(result.status,0,result.stderr);
 return result.stdout.trim();
};
function commit(){
 git('add','lib.rs');
 git('-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','fixture');
}
try{
 git('init','-q');commit();
 await writeFile(join(input.root,'lib.rs'),source(2));commit();
 host=await startHost(input,1);chrome=await browser({desktopBus:false});
 for(const scenario of ['current','editor','return-live','close-origin','close-peer']){
  const page=await chrome.newPage({viewport:{width:1700,height:1200}}),errors=[];
  let release;
  try{
   page.setDefaultTimeout(10000);
   page.on('pageerror',error=>errors.push(error.message));
   await page.addInitScript(()=>{
    const request=globalThis.fetch.bind(globalThis);
    globalThis.fetch=(path,options)=>['/api/function-comparisons/window','/api/captured-functions'].includes(String(path).split('?')[0])
     ?request(path,{...options,signal:undefined}):request(path,options);
   });
   await page.goto(host.url+'/workspace');
   await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
   await page.getByRole('button',{name:'Add bubble',exact:true}).click();
   await page.getByLabel('Find a function',{exact:true}).fill('caller');
   await page.getByRole('button',{name:'Find functions',exact:true}).click();
   await page.locator('[data-add-symbol][data-name="caller"]').click();
   const caller=page.locator('[data-view][data-name="caller"]');
   await caller.locator('.code-line').first().waitFor();
   const status=await page.request.get(host.url+'/api/status').then(response=>response.json());
   const inventory=await page.request.post(host.url+'/api/structure',{data:{snapshot:status.snapshot,query:{kind:'inventory',area:'lib.rs',search:'caller',offset:0,functions_only:true}}}).then(response=>response.json());
   const symbol=inventory.items.find(row=>row.symbol.name==='caller').symbol;
   const calls=await page.request.post(host.url+'/api/structure',{data:{snapshot:status.snapshot,query:{kind:'calls',symbol:symbol.id,area:'lib.rs',offset:0}}}).then(response=>response.json());
   await caller.getByRole('button',{name:'Compare with previous version',exact:true}).click();
   const newer=page.locator('[data-comparison-side="newer"]'),older=page.locator('[data-comparison-side="older"]');
   await older.locator('.comparison-removed').first().waitFor();
   const newerId=await newer.getAttribute('data-view'),olderId=await older.getAttribute('data-view');
   const original=page.locator(`[data-view="${newerId}"]`),peer=page.locator(`[data-view="${olderId}"]`);
   const firstEdit=await newer.locator('.comparison-added').getAttribute('data-line');
   let entered,returned;
   const waiting=new Promise(resolve=>entered=resolve),gate=new Promise(resolve=>release=resolve),finished=new Promise(resolve=>returned=resolve);
   const failure=scenario==='current'?'Current comparison edit failed':'Obsolete comparison edit failed';
   const settled=page.waitForResponse(response=>new URL(response.url()).pathname==='/api/function-comparisons/window'&&response.status()===500);
   await page.route('**/api/function-comparisons/window?*',async route=>{
    if(new URL(route.request().url()).searchParams.get('side')!=='newer')return route.continue();
    entered();await gate;
    try{await route.fulfill({status:500,contentType:'application/json',body:JSON.stringify({message:failure})});}finally{returned();}
   });
   await newer.getByRole('button',{name:'Next changed block',exact:true}).click();await waiting;
   if(scenario==='editor'){
    await page.evaluate(hash=>{location.hash=hash;},'#editor-call='+calls.items[0].id+'&snapshot='+status.snapshot);
    await page.locator('[data-view][data-name="leaf"] .code-line').first().waitFor();
   }else if(scenario==='return-live'){
    await newer.getByRole('button',{name:'Return to live',exact:true}).click();
   }else if(scenario==='close-origin')await newer.getByRole('button',{name:'Close bubble',exact:true}).click();
   else if(scenario==='close-peer')await older.getByRole('button',{name:'Close bubble',exact:true}).click();
   const camera=await page.locator('#workspace-world').getAttribute('style');
   const selected=await page.locator('[data-view].is-selected').getAttribute('data-view');
   release();await finished;await (await settled).finished();await paint(page);
   if(scenario==='current'){
    assert.equal(await page.locator('#visual-error').isVisible(),true);
    assert.match(await page.locator('#visual-error').textContent(),/Current comparison edit failed/);
   }else assert.equal(await page.locator('#visual-error').isVisible(),false,scenario+' must suppress superseded failures');
   if(scenario==='return-live')await page.waitForFunction(id=>!document.querySelector(`[data-view="${id}"] button[aria-label="Open in VS Code"]`).disabled,newerId);
   assert.equal(await page.locator('#workspace-world').getAttribute('style'),camera,scenario);
   assert.equal(await page.locator('[data-view].is-selected').getAttribute('data-view'),selected,scenario);
   if(scenario!=='close-origin'&&scenario!=='return-live')assert.equal(await original.locator('.comparison-added').getAttribute('data-line'),firstEdit);
   for(const view of [original,peer])if(await view.count()){
    assert.doesNotMatch(await view.locator('.fragment-footer').textContent(),/Loading comparison/);
    if(await view.getAttribute('data-comparison-side'))assert.equal(await view.getByRole('button',{name:'Next changed block',exact:true}).isEnabled(),true,scenario);
   }
   assert.deepEqual(errors,[]);
   console.log('comparison edit '+scenario+' passed');
  }finally{release?.();await page.close();}
 }
 console.log('comparison edits: current failures remain visible; uncancellable failures after editor navigation, Return to live or closing either side stay silent and preserve newer state');
}finally{await chrome?.close();await host?.close();await input.close();}
