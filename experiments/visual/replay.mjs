// Source-informed visual replay; this is not an unseen task or a usability evaluation.
import assert from 'node:assert/strict';
import {mkdir,writeFile,readFile} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {browser,startHost,paint} from '../browser/support.mjs';
const [root,output]=process.argv.slice(2);assert.ok(root&&output,'Use replay.mjs TRACT_PLUGIN_PACK NEW_OUTPUT');await mkdir(output);
const sourceRevision=spawnSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).stdout.trim(),binarySha256=createHash('sha256').update(await readFile('target/release/vizzi')).digest('hex');
let host,chromium;
try{
 host=await startHost({root:resolve(root),cache:join(resolve(output),'analysis')},1);chromium=await browser({desktopBus:false});const page=await chromium.newPage({viewport:{width:1680,height:1100}}),errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(host.url+'/workspace?demo=minitosis');await page.locator('[data-view][data-name="process"] .code-line').first().waitFor();
 const steps=[];
 const getView=name=>page.locator(`[data-view][data-name="${name}"]`);
 async function follow(from,name,path,parent){
  await page.locator(`#overview-map rect:has(title:text-matches("${from}"))`).click();const caller=getView(from),node=await caller.elementHandle(),before=await caller.evaluate(n=>({left:n.style.left,top:n.style.top,topScroll:n.querySelector('.code-pane').scrollTop,leftScroll:n.querySelector('.code-pane').scrollLeft}));
  await caller.getByRole('button',{name:'Calls',exact:true}).click();const call=page.getByRole('button',{name:new RegExp('^'+name+' · line ')});
  for(let i=0;!(await call.count())&&i<9;i++){const more=caller.getByRole('button',{name:'More written calls',exact:true});await more.waitFor({timeout:5000});await more.click();await page.waitForTimeout(40);}
  await call.first().click();const stack=page.locator('#destination-stack');let candidates=0;if(await stack.isVisible()){const choices=stack.locator('[data-candidate]');candidates=await choices.count();let candidate=stack.locator(`[data-candidate][data-path="${path}"]`);if(parent)candidate=candidate.filter({hasText:parent});assert.equal(await candidate.count(),1);await candidate.click();}
  if(from==='process'&&name==='render')assert.equal(candidates,0,'known Minitosis self receiver opens without choices');const destination=getView(name);await destination.locator('.code-line').first().waitFor();assert.equal(await node.evaluate(n=>n.isConnected),true);assert.deepEqual(await caller.evaluate(n=>({left:n.style.left,top:n.style.top,topScroll:n.querySelector('.code-pane').scrollTop,leftScroll:n.querySelector('.code-pane').scrollLeft})),before);steps.push({from,to:name,candidatesShown:candidates,callerRetained:true});
 }
 await follow('process','render','minitosis/src/lib.rs');await getView('render').locator('.code-pane').evaluate(n=>{n.scrollTop=850;});await follow('render','process_sample','minitosis/src/engine.rs','Processor');await follow('render','begin_block','minitosis/src/engine.rs');await follow('process_sample','next','minitosis/src/engine.rs','DryDelay');
 assert.equal(await page.locator('[data-view]').count(),5);assert.equal(await page.locator('#workspace-links path').count(),4);await page.getByRole('button',{name:'Fit workspace',exact:true}).click();await paint(page);await page.screenshot({path:join(output,'working-set-light.png')});await page.emulateMedia({colorScheme:'dark'});await paint(page);await page.screenshot({path:join(output,'working-set-dark.png')});
 await page.locator('#overview-map rect:has(title:text-matches("render"))').click();await page.getByRole('button',{name:'Zoom in',exact:true}).click();await paint(page);await page.screenshot({path:join(output,'reading-dark.png')});
 await page.waitForTimeout(200);const saved=await page.evaluate(()=>Object.keys(localStorage).filter(k=>k.startsWith('vizzi.visual.v1.')).map(k=>JSON.parse(localStorage.getItem(k))));assert.equal(saved.length,1);await writeFile(join(output,'workspace.json'),JSON.stringify(saved[0],null,2)+'\n');
 await page.reload();await page.waitForFunction(()=>document.querySelectorAll('[data-view]').length===5);await page.waitForFunction(()=>document.querySelector('[data-view][data-name="render"] .code-pane').scrollTop>0);assert.deepEqual(errors,[]);
 const frames=process.argv.includes('--functional-only')?[]:await page.evaluate(async()=>{const frames=[];let last=performance.now();for(let i=0;i<90;i++){await new Promise(r=>requestAnimationFrame(r));const now=performance.now();frames.push(now-last);last=now;const c=document.querySelector('#workspace-canvas');c.dispatchEvent(new WheelEvent('wheel',{deltaX:i%2?8:-8,deltaY:0,bubbles:true,cancelable:true}));}return frames;});
 await writeFile(join(output,'result.json'),JSON.stringify({schema:1,kind:'source-informed visual functional replay',sourceRevision,binarySha256,steps,views:5,candidateLinks:saved[0].links.filter(l=>l.kind==='candidate').length,selfLinks:saved[0].links.filter(l=>l.kind==='self').length,reloadRestored:true,pageErrors:errors,frameIntervalsMs:frames,limitations:'Single headless browser, known source and explicit candidate choices; not human comprehension, task time, general type resolution, hardware GPU or large-working-set qualification.'},null,2)+'\n');console.log(JSON.stringify({steps,views:5,links:4,reloadRestored:true,pageErrors:errors}));
}finally{await chromium?.close();await host?.close();}
