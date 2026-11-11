import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {buildHost,browser,startHost,workspace} from './support.mjs';

if(!process.argv.includes('--no-build'))buildHost();
const chrome=await browser({desktopBus:false});
const python='from worker import Meter as Selected\nclass Other:\n    def process(self, value): return -1\nclass Service:\n    def __init__(self): self.meter=Selected()\n    def run(self):\n        self.meter.process(1)\n        callback=self.meter.process\n';
const cpp='struct Other { int process(int value){return -value;} };\nstruct Meter { int process(int value){return value;} };\nint run(Meter& meter){return meter.process(2);}\n';
const input=await workspace({files:{'app.py':python,'worker.py':'class Meter:\n    def process(self, value): return value\n','app.cpp':cpp}});
let host;
try {
 await writeFile(join(input.root,'compile_commands.json'),JSON.stringify([{directory:input.root,file:'app.cpp',arguments:['clang++','-std=c++20','-c','app.cpp']}]));
 input.sourceArguments.push('--pyright',process.env.VIZZI_PYRIGHT??'pyright-langserver','--c-family-root','.','--clangd',process.env.VIZZI_CLANGD??'clangd');
 host=await startHost(input,1);
 for(const language of ['python','cpp']){
  let page;
  try{
   page=await chrome.newPage({viewport:{width:1600,height:1100}});page.setDefaultTimeout(30000);
   const errors=[];page.on('pageerror',error=>errors.push(error.message));await page.goto(host.url);await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
   await page.getByRole('button',{name:'Add bubble',exact:true}).click();await page.getByLabel('Find a function',{exact:true}).fill(language==='python'?'Service::run':'run');await page.getByRole('button',{name:'Find functions',exact:true}).click();await page.locator('[data-add-symbol][data-name="run"]').filter({hasText:language==='python'?'app.py':'app.cpp'}).click();
   const caller=page.locator('[data-view][data-name="run"]');await caller.locator('.code-line').first().waitFor();await caller.getByRole('button',{name:'Actions for process',exact:true}).click({modifiers:['Control']});
   const target=page.locator('[data-view][data-name="process"]');await target.locator('.code-line').first().waitFor();assert.match(await target.locator('.fragment-heading strong').textContent(),/Meter::process/);assert.equal(await page.locator('#destination-stack').isVisible(),false);assert.equal(await page.locator('[data-view]').count(),2);assert.equal(await page.locator('#workspace-links .link-stroke').getAttribute('data-kind'),'definition');
   await target.getByRole('button',{name:'Actions for process bubble',exact:true}).click();await page.getByRole('menuitem',{name:'Find references',exact:true}).click();const stack=page.locator('[data-reference-stack]');await stack.getByRole('status').filter({hasText:'Language references'}).waitFor();assert.equal(await stack.locator('[data-reference-row]').count(),language==='python'?2:1);
   if(language==='python'){
    await writeFile(join(input.root,'worker.py'),'class Meter:\n    def process(self, value): return value + 1\n');await target.locator('.code-pane').filter({hasText:'value + 1'}).waitFor();await stack.getByRole('status').filter({hasText:'Repository updated'}).waitFor();await stack.getByRole('button',{name:'Refresh',exact:true}).click();await stack.getByRole('status').filter({hasText:'Language references'}).waitFor();assert.equal(await stack.locator('[data-reference-row]').count(),2);
   }
   assert.deepEqual(errors,[]);console.log(`${language} language server: typed receiver selected over same-name method, semantic link/references, no name chooser${language==='python'?', incremental captured-source refresh':''}`);
  }finally{await page?.close();}
 }
}finally{await host?.close();await input.close();await chrome.close();}
