import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {buildHost,browser,startHost,workspace} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const types=Array.from({length:30},(_,i)=>`struct process_${i};`).join('');
const prefixes=Array.from({length:30},(_,i)=>`fn process_${String(i).padStart(2,'0')}() {}`).join('');
const padding=Object.fromEntries(Array.from({length:4094},(_,i)=>[`za-padding/${i}.rs`,'// padding\n']));
const scopedPath='zz/scoped.rs',scopedText='fn omitted_start() { omitted_finish(); process(); }\nfn omitted_finish() {}\n';
const input=await workspace({files:{
 ...padding,'a.rs':types+prefixes+' struct LufsMeter; impl LufsMeter { fn process_sample(&self) {} }',
 'z.rs':'fn process() {}',[scopedPath]:scopedText,'zz/second.rs':'fn another_omitted() {}\n',
}});let host,chromium;
try{
 host=await startHost(input,1);chromium=await browser({desktopBus:false});const page=await chromium.newPage();await page.goto(host.url+'/workspace');await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));await page.getByRole('button',{name:'Add bubble',exact:true}).click();
 const search=async query=>{await page.getByLabel('Find a function',{exact:true}).fill(query);const result=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/structure'&&r.request().method()==='POST'&&r.request().postDataJSON().query.kind==='inventory');await page.getByRole('button',{name:'Find functions',exact:true}).click();const response=await result;await page.waitForFunction(()=>document.querySelector('#fragment-search-status').textContent.includes('matching functions'));return response.json();};
 const ranked=await search('process');assert.equal(ranked.matching,32);assert.equal(ranked.items[0].symbol.name,'process');await page.waitForFunction(()=>document.querySelector('[data-add-symbol]')?.dataset.name==='process');assert.equal(await page.locator('[data-add-symbol]').count(),24);assert.equal(await page.getByRole('button',{name:'Next results',exact:true}).isVisible(),true);
 const abbreviated=await search('ps');assert.deepEqual(abbreviated.items.map(item=>item.symbol.name),['process_sample']);await page.locator('[data-add-symbol][data-name="process_sample"]').waitFor();
 const qualified=await search('LufsMeter::process_sample');assert.equal(qualified.matching,1);await page.waitForFunction(()=>document.querySelectorAll('[data-add-symbol]').length===1);assert.equal(await page.getByRole('button',{name:'Next results',exact:true}).isVisible(),false);await page.locator('[data-add-symbol][data-name="process_sample"]').click();await page.locator('[data-view][data-name="process_sample"] .code-line').first().waitFor();assert.match(await page.locator('.fragment-heading strong').textContent(),/LufsMeter::process_sample/);

 await page.getByRole('button',{name:'Add bubble',exact:true}).click();
 const global=await search('omitted_start');
 assert.equal(global.matching,0);
 assert.equal(global.coverage.omitted_files,2);
 const path=page.getByLabel('Source file path (optional)',{exact:true});
 await path.fill('zz/second.rs');
 assert.equal((await search('another_omitted')).matching,1);
 await path.fill(scopedPath);
 const scoped=await search('omitted_start');
 assert.equal(scoped.matching,1);
 assert.deepEqual(scoped.coverage,global.coverage);
 assert.equal(scoped.items[0].symbol.source.path,scopedPath);
 await page.locator('[data-add-symbol][data-name="omitted_start"]').click();
 const start=page.locator('[data-view][data-name="omitted_start"]');
 await start.getByRole('button',{name:'Actions for omitted_finish',exact:true}).waitFor();
 await page.getByRole('button',{name:'Add bubble',exact:true}).click();
 await path.fill('zz/second.rs');
 await search('another_omitted');
 await page.getByRole('button',{name:'Close',exact:true}).click();
 await start.getByRole('button',{name:'Actions for omitted_finish',exact:true}).click({modifiers:['Control']});
 const finish=page.locator('[data-view][data-name="omitted_finish"]');
 await finish.locator('.code-line').first().waitFor();
 await start.getByRole('button',{name:'Calls',exact:true}).click();
 await start.getByRole('button',{name:'process · line 1',exact:true}).click();
 await page.locator('[data-view][data-name="process"] .code-line').first().waitFor();
 await page.getByRole('button',{name:'Fit workspace',exact:true}).click();
 await finish.getByRole('button',{name:'Actions for omitted_finish bubble',exact:true}).click();
 await page.getByRole('menuitem',{name:'Find callers',exact:true}).click();
 const references=page.locator('[data-reference-stack]');
 await references.getByRole('status').filter({hasText:'Name-based candidates'}).waitFor();
 assert.equal(await references.locator('[data-reference-row]').count(),1);
 assert.match(await references.locator('[data-reference-row]').textContent(),/omitted_start/);
 await writeFile(join(input.root,scopedPath),scopedText.replace('omitted_finish();','let updated = 2; omitted_finish();'));
 await page.waitForFunction(()=>document.querySelector('[data-name="omitted_start"] .code-pane')?.textContent.includes('updated'));
 await page.waitForFunction(()=>document.querySelectorAll('#workspace-links path.link-stroke[data-state="current"]').length===2);
 console.log('fragment search: ranking, scoped lookup beyond the global file cap, call navigation and live refresh');
}finally{await chromium?.close();await host?.close();await input.close();}
