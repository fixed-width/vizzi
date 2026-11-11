import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir,copyFile} from 'node:fs/promises';
import {join,dirname,resolve} from 'node:path';
import {execFileSync} from 'node:child_process';
import {browser,startHost,workspace,paint,buildHost} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const project=resolve('.');
const chrome=await browser({desktopBus:false}),reports=[];
try{
 for(const kind of ['python','rust']){
  const source=join(project,'docs/tutorials',kind==='python'?'first-call-trail-python':'first-call-trail');
  const input=await workspace({source,name:kind+'-tutorial'});let host,page;if(kind==='python')input.sourceArguments.push('--investigation-file',join(dirname(input.cache),'state','locations.json'));
  try{
   if(kind==='python')assert.equal(execFileSync('python3',[join(input.root,'greeting.py')],{encoding:'utf8'}).trim(),'Hello, Ada!');
   else{
    const driver=join(input.cache,'check.rs');await mkdir(input.cache,{recursive:true});
    await writeFile(driver,`#[path = ${JSON.stringify(join(input.root,'src/lib.rs'))}] mod lesson;\nfn main(){assert_eq!(lesson::Greeter.greet("  Ada  "), "Hello, Ada!");}\n`);
    execFileSync('rustc',['--edition=2024',driver,'-o',join(input.cache,'check')]);execFileSync(join(input.cache,'check'));
   }
   host=await startHost(input,1);page=await chrome.newPage({viewport:{width:1440,height:900},colorScheme:'light'});page.setDefaultTimeout(10000);
   const errors=[];page.on('pageerror',error=>errors.push(error.message));await page.goto(host.url);await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
   await page.getByRole('button',{name:'Add bubble',exact:true}).click();await page.getByLabel('Find a function',{exact:true}).fill(kind==='python'?'greet':'Greeter::greet');await page.getByRole('button',{name:'Find functions',exact:true}).click();await page.locator('[data-add-symbol][data-name="greet"]').click();
   const caller=page.locator('[data-view][data-name="greet"]');await caller.locator('.code-line').first().waitFor();
   await caller.getByRole('button',{name:'Actions for prepare_name',exact:true}).click();await page.getByRole('menuitem',{name:'Open definition',exact:false}).click();await page.locator('[data-view][data-name="prepare_name"] .code-line').first().waitFor();
   await caller.getByRole('button',{name:'Actions for message',exact:true}).click({modifiers:['Control']});await page.locator('[data-view][data-name="message"] .code-line').first().waitFor();
   assert.equal(await page.locator('[data-view]').count(),3);assert.equal(await page.locator('#workspace-links .link-stroke').count(),2);assert.equal(await page.locator('#destination-stack').isVisible(),false);
   await page.getByRole('button',{name:'Fit workspace',exact:true}).click();
   for(const name of ['greet','prepare_name','message'])await page.locator(`[data-view][data-name="${name}"]`).getByRole('button',{name:'Fit height',exact:true}).click();
   const message=page.locator('[data-view][data-name="message"] .fragment-header');await message.focus();for(let n=0;n<12;n++)await page.keyboard.press('ArrowUp');
   await page.getByRole('button',{name:'Fit workspace',exact:true}).click();
   async function menu(name){await page.locator('#workspace-actions > summary').click();await page.getByRole('button',{name,exact:true}).click();}
   await menu('Groups');await page.getByLabel('Group name',{exact:true}).fill('Greeting');for(const member of await page.locator('#group-members input').all())await member.check();await page.getByRole('button',{name:'Save group',exact:true}).click();
   await page.getByRole('button',{name:'Collapse group Greeting',exact:true}).click();assert.equal(await caller.isVisible(),false);await page.getByRole('button',{name:'Expand group Greeting',exact:true}).click();assert.equal(await caller.isVisible(),true);
   await menu('Rename task');await page.getByLabel('Task name',{exact:true}).fill('Read the greeting');await page.getByRole('button',{name:'Save name',exact:true}).click();
   await menu('New task');assert.equal(await page.locator('[data-view]').count(),0);await menu('Task shelf');await page.getByRole('button',{name:'Read the greeting',exact:true}).click();await caller.locator('.code-line').first().waitFor();
   assert.equal(await page.locator('[data-view]').count(),3);assert.equal(await page.locator('[data-group] strong').textContent(),'Greeting');
   if(kind==='python'){
    await page.getByRole('button',{name:'Fit workspace',exact:true}).click();await paint(page);await page.screenshot({path:process.argv.includes('--update-images')?join(project,'docs/images/call-workspace.png'):'/tmp/vizzi-documentation-call-workspace.png'});
    await caller.getByRole('button',{name:'Save location',exact:true}).click();
    await caller.getByRole('button',{name:'Actions for greet bubble',exact:true}).click();await page.getByRole('menuitem',{name:'Find callers',exact:true}).click();await page.locator('[data-reference-stack] [role="status"]').filter({hasText:'Name-based candidates'}).waitFor();
   }
   await page.getByRole('navigation',{name:'View modes',exact:true}).getByRole('link',{name:'Saved locations',exact:true}).click();await page.getByRole('heading',{name:'Saved locations',exact:true}).waitFor();await page.locator('#locations-actions > summary').click();assert.equal(await page.getByRole('button',{name:'Add a location',exact:true}).isVisible(),true);assert.equal(await page.getByRole('button',{name:'Export locations',exact:true}).isVisible(),true);
   if(kind==='python'){
    await page.keyboard.press('Escape');await page.getByRole('button',{name:'Inspect location',exact:true}).click();await page.getByLabel('Your interpretation',{exact:true}).fill('A greeting task');await page.getByRole('button',{name:'Save note',exact:true}).click();
    await page.waitForFunction(()=>!document.querySelector('#note-form button[type=submit]').disabled);const state=JSON.parse(await readFile(join(dirname(input.cache),'state','locations.json'),'utf8'));assert.equal(state.evidence.length,1);assert.equal(state.evidence[0].note,'A greeting task');
    await host.close();host=await startHost(input,1);await page.goto(host.url+'/investigate#saved');await page.getByRole('button',{name:'Inspect location',exact:true}).click();assert.equal(await page.getByLabel('Your interpretation',{exact:true}).inputValue(),'A greeting task');
   }
   if(kind==='rust'&&process.argv.includes('--with-rust-analyzer')){
    await host.close();input.sourceArguments.push('--rust-analyzer','rust-analyzer');host=await startHost(input,1);
    const post=async(path,data)=>{const r=await fetch(host.url+path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(data)});assert.ok(r.ok,await r.clone().text());return r.json();};
    await post('/api/workspace/start',{});let status;
    for(let n=0;n<100;n++){status=await(await fetch(host.url+'/api/status')).json();if(status.readiness==='ready')break;await new Promise(resolve=>setTimeout(resolve,50));}
    assert.equal(status.readiness,'ready');const inventory=await post('/api/structure',{snapshot:status.snapshot,query:{kind:'inventory',area:'',search:'Greeter::prepare_name',offset:0,functions_only:true}});const references=await post('/api/references',{snapshot:status.snapshot,symbol:inventory.items[0].symbol.id,kind:'callers'});assert.equal(references.precision,'semantic');assert.equal(references.matching,1);
   }
   assert.deepEqual(errors,[]);console.log(kind+' tutorial passed');reports.push({language:kind,example_output:'Hello, Ada!',source_bubbles:3,branches:2,group_and_task_restored:true,notes_labels_checked:true});
  }finally{await page?.close();await host?.close();await input.close();}
 }
 console.log(JSON.stringify(reports));
}finally{await chrome.close();}
