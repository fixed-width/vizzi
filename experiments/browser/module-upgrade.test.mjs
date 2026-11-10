// Opt-in: supply --previous /path/to/pre-module-vizzi to exercise an actual host upgrade.
import assert from 'node:assert/strict';
import {browser,startHost,workspace,paint} from './support.mjs';

const previous=process.argv[process.argv.indexOf('--previous')+1];
assert.ok(process.argv.includes('--previous')&&previous,'Supply --previous with a pre-module viewer binary');
const input=await workspace({files:{'Cargo.toml':"[package]\nname='audio'\n",'src/lib.rs':'struct Meter; impl Meter { fn start(&self){self.render();} fn render(&self){} }\n'}});
let host,chrome;
try{
 input.executable=previous;host=await startHost(input,1);chrome=await browser({desktopBus:false});const page=await chrome.newPage({viewport:{width:1600,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(host.url+'/workspace');await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
 await page.getByRole('button',{name:'Add bubble',exact:true}).click();await page.getByLabel('Find a function',{exact:true}).fill('start');await page.getByRole('button',{name:'Find functions',exact:true}).click();await page.locator('[data-add-symbol][data-name="start"]').click();const start=page.locator('[data-view][data-name="start"]');await start.locator('.code-line').first().waitFor();await start.getByRole('button',{name:'Actions for render',exact:true}).click({modifiers:['Control']});await page.locator('[data-view][data-name="render"] .code-line').first().waitFor();await page.getByRole('button',{name:'Fit workspace',exact:true}).click();
 const state=()=>page.evaluate(()=>{const key=Object.keys(localStorage).find(k=>k.startsWith('vizzi.visual.v1.')&&!k.endsWith('.previous'));return JSON.parse(localStorage.getItem(key));});
 await page.waitForFunction(()=>Object.keys(localStorage).some(k=>k.startsWith('vizzi.visual.v1.')&&JSON.parse(localStorage[k]).views?.length===2));await paint(page);const before=await state();assert.ok(before.views.every(v=>!Object.hasOwn(v.symbol,'module')));assert.equal(before.links.length,1);
 const origin=host.url,port=Number(new URL(origin).port);await host.close();delete input.executable;host=await startHost(input,1,port);assert.equal(host.url,origin);await page.reload();
 await page.waitForFunction(()=>document.querySelectorAll('[data-view] .module-tag:not([hidden])').length===2);await page.waitForFunction(()=>Object.keys(localStorage).some(k=>k.startsWith('vizzi.visual.v1.')&&JSON.parse(localStorage[k]).views?.every(v=>v.symbol.module?.label==='audio')));
 const after=await state(),layout=s=>({camera:s.camera,views:s.views.map(v=>[v.id,v.x,v.y,v.w,v.h,v.collapsed])});assert.deepEqual(layout(after),layout(before));assert.ok(after.views.every(v=>v.state==='current'&&v.symbol.source.context!==before.views.find(b=>b.id===v.id).symbol.source.context));assert.equal(after.links.length,1);assert.equal(after.links[0].state,'current');assert.deepEqual(errors,[]);
 await start.getByRole('button',{name:'Actions for render',exact:true}).click({modifiers:['Control']});assert.equal(await page.locator('[data-view]').count(),2);console.log('actual pre-module → current host upgrade: same origin/cache, layout/camera/view IDs retained, exact native context recovery, labels hydrated, relationship reverified, navigation usable');
}finally{await chrome?.close();await host?.close();await input.close();}
