import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {buildHost,browser,startHost,workspace,paint} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const text='fn target(){}\nfn caller_one(){\n let working_version = 1;\n'+' // context\n'.repeat(48)+' target();\n}\nfn caller_two(){\n'+' // context\n'.repeat(120)+' target();\n}\nfn unrelated(){}\n';
const input=await workspace({files:{'lib.rs':text,'Cargo.toml':"[package]\nname='working_sets'\nversion='0.1.0'\nedition='2024'\n[lib]\npath='lib.rs'\n"}});let host,chrome,page;
try{
 const git=(...args)=>{const result=spawnSync('git',['-C',input.root,...args],{encoding:'utf8'});assert.equal(result.status,0,result.stderr);};
 async function commit(contents){
  await writeFile(join(input.root,'lib.rs'),contents);git('add','lib.rs','Cargo.toml');
  git('-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','fixture');
 }
 git('init','-q');await commit(text.replace('working_version = 1','earlier_version = 0'));
 await writeFile(join(input.root,'lib.rs'),text);
 host=await startHost(input,1);chrome=await browser({desktopBus:false});page=await chrome.newPage({viewport:{width:1700,height:1100}});const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.addInitScript(()=>{const NativeSource=EventSource;window.workspaceTestStreams=[];window.EventSource=class extends NativeSource{constructor(...args){super(...args);workspaceTestStreams.push(this);}};});await page.goto(host.url);await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
 async function menu(name){await page.locator('#workspace-actions>summary').click();await page.getByRole('button',{name,exact:true}).click();}
 async function open(name){await page.getByRole('button',{name:'Add bubble',exact:true}).click();await page.getByLabel('Find a function',{exact:true}).fill(name);await page.getByRole('button',{name:'Find functions',exact:true}).click();await page.locator(`[data-add-symbol][data-name="${name}"]`).click();const view=page.locator(`[data-view][data-name="${name}"]`);await view.locator('.code-line').first().waitFor();return view;}
 await page.locator('#workspace-settings > summary').click();await page.getByLabel('Module outlines',{exact:true}).check();await page.keyboard.press('Escape');
 await open('target');const target=page.locator('[data-view][data-name="target"]');await target.getByRole('button',{name:'Actions for target bubble',exact:true}).click();await page.getByRole('menuitem',{name:'Find callers',exact:true}).click();const stack=page.locator('[data-reference-stack]');await stack.getByRole('status').filter({hasText:'Name-based candidates'}).waitFor();assert.equal(await stack.locator('[data-reference-row]').count(),2);await stack.locator('[data-reference-row]').last().hover();await stack.locator('.reference-preview .is-reference-line').waitFor({timeout:3000});assert.match(await stack.locator('.reference-preview .is-reference-line').textContent(),/target/);const beforeWheel=await page.locator('#workspace-world').evaluate(n=>n.style.transform);
 await stack.locator('.reference-body>p').hover();await page.mouse.wheel(0,140);await page.waitForTimeout(100);
 assert.equal(await page.locator('#workspace-world').evaluate(n=>n.style.transform),beforeWheel);
 assert.ok(await stack.locator('.reference-body').evaluate(n=>n.scrollTop>0));await stack.locator('.reference-body').evaluate(n=>n.scrollTop=0);
 await stack.locator('[data-reference-row]').first().hover();await stack.locator('.reference-preview .code-line').first().waitFor();await stack.locator('[data-reference-row]').first().click();await page.locator('[data-view][data-name="caller_one"] .code-line').first().waitFor();assert.equal(await page.locator('#workspace-links .link-stroke').count(),1);assert.equal(await page.locator('[data-view]').count(),2);
 const callerRow=stack.locator('[data-reference-row]').first();
 assert.match(await callerRow.textContent(),/Open live source/);
 const showComparison=stack.getByRole('button',{name:'Show comparison',exact:true});
 assert.equal(await showComparison.count(),0);
 const callers=page.locator('[data-view][data-name="caller_one"]');
 await callers.getByRole('button',{name:'Compare with previous version',exact:true}).click();
 const compared=page.locator('[data-view][data-name="caller_one"][data-comparison-side]');
 const after=page.locator('[data-view][data-name="caller_one"][data-comparison-side="newer"]');
 await after.locator('.comparison-added').first().waitFor();
 await compared.locator('.code-pane').evaluateAll(panes=>{for(const pane of panes)pane.scrollTop=84;});
 const captures=()=>compared.evaluateAll(views=>views.map(view=>({
  id:view.dataset.view,token:view.dataset.comparisonToken,side:view.dataset.comparisonSide,
  geometry:[view.style.left,view.style.top,view.style.width,view.style.height],
  scroll:view.querySelector('.code-pane').scrollTop,
 })));
 const preserved=await captures();
 assert.equal(preserved.length,2);assert.ok(preserved.every(view=>view.scroll>0));
 await page.getByRole('button',{name:'Fit workspace',exact:true}).click();
 for(const {id} of preserved)await page.locator(`[data-view="${id}"]`).getByRole('button',{name:'Collapse bubble',exact:true}).click();
 await target.locator('.fragment-heading strong').click();
 await page.reload();
 await stack.getByRole('status').filter({hasText:'Name-based candidates'}).waitFor();
 for(const {id} of preserved)await page.locator(`[data-view="${id}"] .code-line`).first().waitFor({state:'attached'});
 assert.equal(await target.evaluate(view=>view.classList.contains('is-selected')),true,'another bubble is selected before revealing the comparison');
 assert.equal(await compared.locator('.code-pane:visible').count(),0,'both comparison panes start collapsed');
 const comparisonZoom=await page.locator('#visual-zoom').textContent();
 await showComparison.waitFor({timeout:5000});
 await showComparison.press('Enter');
 assert.equal(await callers.count(),2,'showing the comparison does not open a live caller');
 assert.equal(await compared.locator('.code-pane:visible').count(),2,'showing the comparison expands both retained panes');
 assert.equal(await after.evaluate(view=>view.classList.contains('is-selected')),true);
 assert.equal(await after.locator('.code-pane').evaluate(pane=>document.activeElement===pane),true,'keyboard focus moves into the revealed After pane');
 assert.equal(await page.locator('#visual-zoom').textContent(),comparisonZoom);
 assert.deepEqual(await captures(),preserved,'revealing restores the saved comparison reading positions');
 assert.equal(await page.locator('#workspace-links .link-stroke[data-state="current"]').count(),0);
 await page.getByRole('button',{name:'Fit workspace',exact:true}).click();
 await callerRow.click();
 const liveCaller=page.locator('[data-view][data-name="caller_one"]:not([data-comparison-side])');
 await liveCaller.locator('.code-line').first().waitFor();
 assert.equal(await callers.count(),3);assert.deepEqual(await captures(),preserved);
 const liveId=await liveCaller.getAttribute('data-view');
 const liveLinks=await page.locator('#workspace-links .link-stroke[data-state="current"]').evaluateAll(links=>links.map(link=>link.dataset.linkId));
 assert.equal(liveLinks.length,1);assert.ok(liveLinks[0].startsWith(liveId+'|'));
 await page.getByRole('button',{name:'Fit workspace',exact:true}).click();
 await callerRow.click();assert.equal(await callers.count(),3);
 assert.equal(await liveCaller.getAttribute('data-view'),liveId);
 await writeFile(join(input.root,'lib.rs'),text.replace('working_version = 1','working_version = 2'));
 await page.waitForFunction(()=>document.querySelector('[data-name="caller_one"]:not([data-comparison-side]) .code-pane').textContent.includes('working_version = 2'));
 await stack.getByRole('status').filter({hasText:'Repository updated'}).waitFor();
 await stack.getByRole('button',{name:'Refresh',exact:true}).click();
 await stack.getByRole('status').filter({hasText:'Name-based candidates'}).waitFor();
 await liveCaller.getByRole('button',{name:'Compare with previous version',exact:true}).click();
 await page.locator(`[data-view="${liveId}"][data-comparison-side="newer"] .code-line`).filter({hasText:'working_version = 2'}).waitFor();
 await compared.locator('.code-pane').evaluateAll(panes=>{for(const pane of panes)pane.scrollTop=84;});
 await paint(page);
 await page.getByRole('button',{name:'Fit workspace',exact:true}).click();
 const choices=stack.getByRole('button',{name:/^Show comparison/});
 await page.waitForFunction(()=>[...document.querySelectorAll('[data-reference-stack] button')].filter(button=>button.textContent.startsWith('Show comparison')).length===2);
 const labels=await choices.allTextContents();
 assert.equal(new Set(labels.map(label=>label.replace(/ · bubble \d+$/,''))).size,1,'the fixtures share the revision and reading line labels');
 assert.equal(new Set(labels).size,2,'distinct captures of the same revision and line have distinct labels');
 await choices.first().press('Enter');
 const firstAfter=page.locator(`[data-view="${preserved.find(view=>view.side==='newer').id}"]`);
 assert.equal(await firstAfter.evaluate(view=>view.classList.contains('is-selected')&&view.querySelector('.code-pane').textContent.includes('working_version = 1')),true);
 await page.getByRole('button',{name:'Fit workspace',exact:true}).click();
 await choices.last().press('Enter');
 assert.equal(await page.locator(`[data-view="${liveId}"]`).evaluate(view=>view.classList.contains('is-selected')),true);
 assert.deepEqual(await choices.allTextContents(),labels);
 await page.getByRole('button',{name:'Fit workspace',exact:true}).click();
 await callerRow.click();await liveCaller.locator('.code-line').first().waitFor();
 await page.getByRole('button',{name:'Fit workspace',exact:true}).click();
 const comparedIds=await compared.evaluateAll(views=>views.map(view=>view.dataset.view));
 for(const id of comparedIds)await page.locator(`[data-view="${id}"]`).getByRole('button',{name:'Close bubble',exact:true}).click();
 assert.equal(await showComparison.count(),0,'closing the retained pair removes the comparison action');
 await page.getByRole('button',{name:'Fit workspace',exact:true}).click();await menu('Groups');await page.getByLabel('Group name',{exact:true}).fill('Input path');for(const box of await page.locator('#group-members input').all())await box.check();await page.getByRole('button',{name:'Save group',exact:true}).click();const group=page.locator('[data-group]');const before=await target.evaluate(n=>parseFloat(n.style.left));await group.getByLabel('Move group Input path',{exact:true}).focus();await page.keyboard.press('ArrowRight');assert.equal(await target.evaluate(n=>parseFloat(n.style.left)),before+16);await group.getByRole('button',{name:'Collapse group Input path',exact:true}).click();assert.equal(await target.isVisible(),false);assert.equal(await stack.isVisible(),false);await group.getByRole('button',{name:'Expand group Input path',exact:true}).click();assert.equal(await target.isVisible(),true);assert.equal(await stack.isVisible(),true);
 await menu('Rename task');await page.getByLabel('Task name',{exact:true}).fill('Trace input');await page.getByRole('button',{name:'Save name',exact:true}).click();await menu('New task');assert.equal(await page.locator('[data-view],[data-reference-stack],[data-group]').count(),0);await open('unrelated');await menu('Rename task');await page.getByLabel('Task name',{exact:true}).fill('Other task');await page.getByRole('button',{name:'Save name',exact:true}).click();await menu('Task shelf');await page.getByRole('button',{name:'Trace input',exact:true}).click();await target.locator('.code-line').first().waitFor();assert.equal(await page.locator('[data-view]').count(),2);assert.equal(await page.locator('[data-reference-stack]').count(),1);assert.equal(await group.locator('strong').textContent(),'Input path');assert.equal(await page.locator('#task-title').textContent(),'Trace input');await page.getByRole('button',{name:'Fit workspace',exact:true}).click();await paint(page);await page.screenshot({path:'/tmp/vizzi-working-sets-light.png'});await page.emulateMedia({colorScheme:'dark'});await page.screenshot({path:'/tmp/vizzi-working-sets-dark.png'});await page.emulateMedia({colorScheme:'light'});await page.waitForTimeout(200);await page.reload();await target.locator('.code-line').first().waitFor();assert.equal(await page.locator('#task-title').textContent(),'Trace input');assert.equal(await page.locator('[data-group]').count(),1);
 await stack.getByRole('status').filter({hasText:'Name-based candidates'}).waitFor();
 const streams=await page.evaluate(()=>workspaceTestStreams.length);
 await page.evaluate(async()=>{const status=await(await fetch('/api/status')).json();const stream=workspaceTestStreams.at(-1);for(const pending of [true,false,true,false])stream.dispatchEvent(new MessageEvent('workspace',{data:JSON.stringify({status:{...status,pending}})}));});
 await paint(page);assert.equal(await page.evaluate(()=>workspaceTestStreams.length),streams);assert.deepEqual(errors,[]);assert.equal(await page.locator('#visual-notice').isVisible(),false);assert.match(await stack.getByRole('status').textContent(),/Name-based candidates/);assert.equal(await stack.locator('[data-reference-row]').first().isEnabled(),true);
 await stack.getByRole('button',{name:'Collapse',exact:true}).click();
 await page.waitForTimeout(200);await page.reload();await target.locator('.code-line').first().waitFor();
 await stack.getByRole('button',{name:'Expand',exact:true}).waitFor();await stack.getByRole('button',{name:'Expand',exact:true}).click();
 await stack.getByRole('status').filter({hasText:'Name-based candidates'}).waitFor();await writeFile(join(input.root,'lib.rs'),text.replace('caller_two','changed_caller'));await stack.getByRole('status').filter({hasText:'Repository updated'}).waitFor();await stack.getByRole('button',{name:'Refresh',exact:true}).click();await stack.getByRole('status').filter({hasText:'Name-based candidates'}).waitFor();assert.equal(await stack.locator('[data-reference-row]').count(),2);assert.deepEqual(errors,[]);
 const activeLayout=await page.evaluate(()=>{const key=Object.keys(localStorage).find(k=>k.startsWith('vizzi.visual.tasks.v1.'));const shelf=JSON.parse(localStorage[key]);return shelf.tasks.find(t=>t.id===shelf.active).layout;});
 const legacy=JSON.parse(activeLayout);delete legacy.collections;const legacyRaw=JSON.stringify(legacy);const context=await chrome.newContext();await context.addInitScript(raw=>localStorage.setItem('vizzi.visual.v1.source',raw),legacyRaw);const migrated=await context.newPage();await migrated.goto(host.url);await migrated.locator('[data-view][data-name="target"] .code-line').first().waitFor();assert.equal(await migrated.locator('[data-group],[data-reference-stack]').count(),0);assert.equal(await migrated.evaluate(()=>localStorage.getItem('vizzi.visual.v1.source')),legacyRaw);await context.close();
 await page.evaluate(()=>{
  window.taskOriginalFetch=window.fetch;
  window.fetch=async(input,options)=>{
   const held=input==='/api/references';
   const response=await taskOriginalFetch(input,held?{...options,signal:undefined}:options);
   if(held){window.referenceReplyWaiting=true;await new Promise(resolve=>window.releaseReferenceReply=resolve);window.referenceReplyDelivered=true;}
   return response;
  };
 });
 await stack.getByRole('button',{name:'Refresh',exact:true}).click();
 await page.waitForFunction(()=>window.referenceReplyWaiting);
 await menu('New task');assert.equal(await page.locator('[data-view],[data-reference-stack]').count(),0);
 await page.evaluate(()=>{window.fetch=taskOriginalFetch;releaseReferenceReply();});
 await page.waitForFunction(()=>window.referenceReplyDelivered);await paint(page);
 assert.equal(await page.locator('[data-view],[data-reference-stack]').count(),0);assert.equal(await page.locator('#task-title').textContent(),'Untitled task');
 await menu('Task shelf');await page.getByRole('button',{name:'Trace input',exact:true}).click();await target.locator('.code-line').first().waitFor();
 const countBefore=await page.locator('[data-view]').count();await page.evaluate(()=>{window.taskWrite=Storage.prototype.setItem;Storage.prototype.setItem=function(key,value){if(key.startsWith('vizzi.visual.tasks.v1.'))throw new DOMException('Task storage full','QuotaExceededError');return taskWrite.call(this,key,value);};});await menu('New task');assert.equal(await page.locator('[data-view]').count(),countBefore);assert.equal(await page.locator('#task-title').textContent(),'Trace input');await page.evaluate(()=>Storage.prototype.setItem=taskWrite);
 console.log('working sets: caller stacks, embedded previews, verified caller connection, group membership/movement/collapse, task isolation/shelf/reload, collapsed-stack persistence, stale-result refresh and late-reply isolation');
}catch(error){if(page)console.error(await page.locator('body').innerText());throw error;}finally{await chrome?.close();await host?.close();await input.close();}
