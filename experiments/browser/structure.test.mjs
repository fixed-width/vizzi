import assert from 'node:assert/strict';
import {writeFile,mkdir,chmod,readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {buildHost,browser,startHost,workspace,paint} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const input=await workspace({files:{'beacon/run.rs':'pub fn start() { middle(); }\npub fn middle() { end(); }\npub fn end() { start(); }\n','beacon-old/panel.rs':'fn middle() {}\n'}});
await mkdir(input.cache);const editor=join(input.cache,'record-editor');await writeFile(editor,'#!/bin/sh\nprintf \'%s\\n\' "$@" >> "$0.args"\n');await chmod(editor,0o700);input.sourceArguments=['--editor-code',editor];
let host,chromium;
try {
 host=await startHost(input,1);chromium=await browser({desktopBus:false});const page=await chromium.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(host.url+'/investigate?trial=structure');assert.equal(await page.locator('#structure-panel').count(),1);
 await page.waitForFunction(()=>document.querySelector('#source-state').textContent.includes('ready'));
 assert.equal(await page.locator('input:visible,textarea:visible').count(),1);
 await page.getByLabel('Find a source area or name',{exact:true}).fill('middle');await page.getByRole('button',{name:'Find areas',exact:true}).click();await page.locator('button[data-area="beacon/run.rs"]').click();await page.locator('[data-structure-symbol][data-name="middle"]').waitFor();assert.equal(await page.locator('[data-structure-symbol]').count(),1);
 await page.getByLabel('Find a source area or name',{exact:true}).fill('beacon');await page.getByRole('button',{name:'Find areas',exact:true}).click();
 await page.locator('button[data-area="beacon"]').click();await page.locator('[data-structure-symbol][data-name="start"]').waitFor();
 assert.equal(await page.locator('[data-structure-symbol][data-name="middle"]').count(),1);
 await page.locator('[data-structure-symbol][data-name="start"]').getByRole('button',{name:'Follow written calls',exact:true}).click();
 await page.locator('[data-call-site]').first().waitFor();assert.match(await page.locator('#structure-results').textContent(),/middle/);
 assert.match(await page.locator('#structure-limit').textContent(),/not.*resolved/i);
 await page.getByRole('button',{name:'Follow middle',exact:true}).first().click();await page.waitForFunction(()=>document.querySelector('#structure-trail').textContent.includes('middle'));
 await paint(page);await page.screenshot({path:'/tmp/vizzi-structure-candidate-light.png'});await page.emulateMedia({colorScheme:'dark'});await paint(page);await page.screenshot({path:'/tmp/vizzi-structure-candidate-dark.png'});
 await page.getByRole('button',{name:'Inspect call',exact:true}).first().click();await page.locator('#preview-source').waitFor({state:'visible'});
 await page.locator('#keep-discovered').click();await page.waitForFunction(()=>document.querySelector('#show-saved').textContent.includes('1'));await page.locator('#preview-editor').click();await page.locator('#investigation-notice').waitFor({state:'visible'});assert.match(await readFile(editor+'.args','utf8'),/--goto\n.*beacon\/run\.rs:2:1/);
 await page.locator('#close-preview').click();await page.locator('[data-trail-index="0"]').click();await page.waitForFunction(()=>document.querySelector('#structure-trail').textContent.includes('start')&&!document.querySelector('#structure-trail').textContent.includes('middle'));
 await writeFile(join(input.root,'beacon/run.rs'),'pub fn replacement() {}\n');await page.locator('#structure-stale').waitFor({state:'visible'});assert.equal(await page.locator('#structure-results button:enabled').count(),0);
 await page.setViewportSize({width:600,height:900});await paint(page);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);assert.deepEqual(errors,[]);
 console.log('structure browser: area discovery, bounded inventory, candidate ambiguity, selected trail/back, captured call-site keep, recording editor, stale actions and narrow layout passed');
}finally{await chromium?.close();await host?.close();await input.close();}
