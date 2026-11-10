import assert from 'node:assert/strict';
import {join} from 'node:path';
import {buildHost,browser,startHost,workspace} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const input=await workspace({files:{'lib.rs':'fn entry() {}\n'}});
input.sourceArguments=['--investigation-file',join(input.cache,'..','saved.json')];
let host,chromium;
try{
 host=await startHost(input,1);chromium=await browser({desktopBus:false});
 const board=async()=>{const response=await fetch(host.url+'/api/investigation');assert.ok(response.ok);return (await response.json()).board;};
 const legacy='Previously saved investigation question';
 const seeded=await fetch(host.url+'/api/investigation',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({action:'question',revision:(await board()).revision,text:legacy})});assert.ok(seeded.ok);
 const before=await board(),page=await chromium.newPage(),errors=[],requests=[];
 page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>requests.push(new URL(r.url()).pathname));
 for(const path of ['/','/investigate','/investigate?trial=search','/investigate?trial=search#saved','/investigate?trial=structure']){
  await page.goto('about:blank');await page.goto(host.url+path);await page.waitForFunction(()=>document.querySelector('#save-mode').textContent);
  assert.equal(await page.locator('#question-search-panel,#source-question-form,#question-form,#question').count(),0,`${path}: retired question controls are absent`);
  assert.equal(await page.locator('a[href*="trial=search"]').count(),0);
  assert.equal(await page.getByRole('heading',{name:legacy,exact:true}).count(),0);
  if(path.includes('#saved'))assert.equal(await page.getByRole('heading',{name:'Saved locations',exact:true}).isVisible(),true);
  else if(!path.includes('trial=structure'))await page.locator('#discovery-panel').waitFor({state:'visible'});
 }
 await page.goto(host.url+'/workspace');await page.getByRole('button',{name:'Add bubble',exact:true}).waitFor();assert.equal(await page.locator('#source-question-form,#question-form').count(),0);
 for(const path of ['/question-search.js','/question-state.js'])assert.equal((await fetch(host.url+path)).status,404);
 assert.equal(requests.includes('/question-search.js'),false);assert.equal(requests.includes('/question-state.js'),false);assert.equal(requests.includes('/api/search'),false);
 assert.deepEqual(await board(),before,'opening retired URLs preserves all saved investigation data');assert.deepEqual(errors,[]);
 console.log('retired question UI: no controls, links or assets; legacy URLs browse normally; saved data and visual entry point preserved');
}finally{await chromium?.close();await host?.close();await input.close();}
