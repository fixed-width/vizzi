import assert from 'node:assert/strict';
import {writeFile,mkdir} from 'node:fs/promises';import {join} from 'node:path';
import {buildHost,browser,startHost,workspace,paint} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const input=await workspace({files:{
 'Cargo.toml':'[package]\nname="semantic_browser"\nversion="0.1.0"\nedition="2024"\n[lib]\npath="lib.rs"\n',
 'lib.rs':'mod worker; trait Plugin { fn process(&mut self); } struct Service { meter: worker::Meter } impl Plugin for Service { fn process(&mut self) { self.meter.run(); } }',
 'worker.rs':'pub struct Meter; impl Meter { pub fn run(&mut self) {} }',
 'other.rs':'fn run() {}',
}});input.sourceArguments=['--rust-analyzer','rust-analyzer'];let host,chromium;
try{
 host=await startHost(input,1);chromium=await browser({desktopBus:false});const page=await chromium.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(host.url+'/workspace');await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
 await page.getByRole('button',{name:'Add bubble',exact:true}).click();await page.getByLabel('Find a function',{exact:true}).fill('process');await page.getByRole('button',{name:'Find functions',exact:true}).click();await page.locator('[data-add-symbol][data-name="process"]').filter({hasText:'for Service'}).click();const root=page.locator('[data-view][data-name="process"]');await root.locator('.code-line').first().waitFor();const before=await root.evaluate(n=>({left:n.style.left,top:n.style.top})),node=await root.elementHandle();
 await root.getByRole('button',{name:'Calls',exact:true}).click();await root.getByRole('button',{name:'run · line 1',exact:true}).click();const target=page.locator('[data-view][data-name="run"]');await target.locator('.code-line').first().waitFor({timeout:30000});assert.match(await target.locator('.fragment-heading').textContent(),/Meter::run/);assert.equal(await page.locator('#destination-stack').isVisible(),false);assert.equal(await page.locator('#workspace-links path.definition-call').count(),1);assert.deepEqual(await root.evaluate(n=>({left:n.style.left,top:n.style.top})),before);assert.equal(await node.evaluate(n=>n.isConnected),true);
 await page.waitForTimeout(250);await page.reload();await page.locator('#workspace-links path.definition-call').waitFor({state:'attached'});await paint(page);assert.deepEqual(errors,[]);console.log('semantic calls browser: field receiver uses project LSP definition, skips unrelated names, preserves caller and restores context');
}finally{await chromium?.close();await host?.close();await input.close();}
