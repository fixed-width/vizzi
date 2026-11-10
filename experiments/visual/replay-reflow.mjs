// Known-source functional replay and local layout measurements; not an independent usability trial.
import assert from 'node:assert/strict';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {browser,startHost,paint} from '../browser/support.mjs';
const [root,output]=process.argv.slice(2);assert.ok(root&&output,'Use replay-reflow.mjs ROOT NEW_OUTPUT');await mkdir(output);
let host,chromium;
try {
 host=await startHost({root:resolve(root),cache:join(resolve(output),'analysis')},1);chromium=await browser({desktopBus:false});const page=await chromium.newPage({viewport:{width:1600,height:1100}}),errors=[],windows=[];page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>{if(new URL(r.url()).pathname==='/api/window')windows.push(r.url());});
 const post=async(path,body)=>{const response=await fetch(host.url+path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)});assert.ok(response.ok);return response.json();};
 await page.goto(host.url+'/workspace');await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));const status=await(await fetch(host.url+'/api/status')).json();
 await page.evaluate(async()=>{const {SourcePane}=await import('/source-pane.js'),original=SourcePane.prototype.update;window.reflowCosts=[];SourcePane.prototype.update=function(...args){const start=performance.now();try{return original.apply(this,args);}finally{window.reflowCosts.push({ms:performance.now()-start,width:this.element.closest('.fragment')?.style.width,physicalRows:this.rows.length});}};});
 const observations=[];
 async function open(area,name,parents){
  const inventory=await post('/api/structure',{snapshot:status.snapshot,query:{kind:'inventory',area,search:name,offset:0,functions_only:true}});const symbol=inventory.items.find(i=>i.symbol.name===name&&i.symbol.parents.some(p=>p.includes(parents))).symbol;
  await page.getByRole('button',{name:'Add bubble',exact:true}).click();await page.getByLabel('Find a function',{exact:true}).fill(name);const searched=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/structure'&&r.request().method()==='POST'&&r.request().postDataJSON().query.kind==='inventory'&&r.request().postDataJSON().query.search===name);await page.getByRole('button',{name:'Find functions',exact:true}).click();await searched;await paint(page);const choice=page.locator(`[data-add-symbol="${symbol.id}"]`);for(let i=0;!(await choice.count())&&i<20;i++){const next=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/structure'&&r.request().method()==='POST'&&r.request().postDataJSON().query.kind==='inventory');await page.getByRole('button',{name:'Next results',exact:true}).click();await next;await paint(page);}await choice.click();
  const pane=page.locator('[data-view]').filter({has:page.locator('.fragment-heading strong',{hasText:parents+'::'+name})});await pane.locator('.code-line').first().waitFor();return {pane,symbol};
 }
 const {pane:caller,symbol:process}=await open('minitosis/src/lib.rs','process','Minitosis');
 let offset=0,site;for(let i=0;i<16;i++){const result=await post('/api/structure',{snapshot:status.snapshot,query:{kind:'calls',symbol:process.id,area:'minitosis/src',offset}});site=result.items.find(c=>c.name==='render'&&c.binding==='rust_self_receiver');if(site||result.next===null)break;offset=result.next;}assert.ok(site);
 const identity=await caller.elementHandle(),before=await caller.evaluate(n=>({left:n.style.left,top:n.style.top}));await caller.locator(`[data-call-id="${site.id}"]`).click({modifiers:['Control']});await page.locator('[data-view][data-name="render"] .code-line').first().waitFor();assert.deepEqual(await caller.evaluate(n=>({left:n.style.left,top:n.style.top})),before);assert.equal(await identity.evaluate(n=>n.isConnected),true);
 await open('gs-meter/src/lib.rs','process','GsMeter');await page.getByRole('button',{name:'Fit workspace',exact:true}).click();await paint(page);
 for(const pane of await page.locator('[data-view]').all()){
  const physical=await pane.locator('.code-line').count(),segments=await pane.locator('.reflow-segment').count();assert.ok(physical<=96);observations.push({title:await pane.locator('.fragment-heading strong').innerText(),physicalRows:physical,displayRows:segments,calls:await pane.locator('.code-call').count(),textSha256:createHash('sha256').update(await pane.locator('.code-pane').textContent()).digest('hex')});
 }
 const requestsBefore=windows.length;await page.evaluate(()=>window.reflowCosts=[]);const grip=caller.getByRole('button',{name:'Resize bubble',exact:true});await grip.focus();
 for(const target of [320,448,720,960,448,320]){
  for(let i=0;i<50;i++){const width=await caller.evaluate(n=>parseFloat(n.style.width));if(width===target)break;await grip.press(width<target?'ArrowRight':'ArrowLeft');}assert.equal(await caller.evaluate(n=>parseFloat(n.style.width)),target);await paint(page);
 }
 assert.equal(windows.length,requestsBefore,'resizing does not fetch source');assert.deepEqual(await caller.evaluate(n=>({left:n.style.left,top:n.style.top})),before);const costs=await page.evaluate(()=>window.reflowCosts);assert.ok(costs.length>0);
 await page.getByRole('button',{name:'Fit workspace',exact:true}).click();await paint(page);await page.screenshot({path:join(output,'smart-light.png')});await page.emulateMedia({colorScheme:'dark'});await page.screenshot({path:join(output,'smart-dark.png')});
 await page.locator('#workspace-settings > summary').click();await page.getByLabel('Code layout',{exact:true}).selectOption('original');await page.locator('#workspace-settings > summary').click();await paint(page);await page.screenshot({path:join(output,'original-dark.png')});await page.locator('#workspace-settings > summary').click();await page.getByLabel('Code layout',{exact:true}).selectOption('smart');await page.locator('#workspace-settings > summary').click();assert.deepEqual(errors,[]);
 const sorted=costs.map(c=>c.ms).sort((a,b)=>a-b);const summary={samples:costs.length,medianMs:sorted[Math.floor(sorted.length/2)],p95Ms:sorted[Math.floor((sorted.length-1)*.95)],maxMs:sorted.at(-1),sourceRequestsDuringResize:windows.length-requestsBefore};
 const result={schema:1,kind:'known-source reflow functional replay and instrumented local layout observation',sourceRevision:spawnSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).stdout.trim(),binarySha256:createHash('sha256').update(await readFile('target/release/vizzi')).digest('hex'),observations,summary,costs,pageErrors:errors,limitations:'One isolated Chromium session on this host after other checks stopped. Instrumentation includes SourcePane.update and synchronous layout, not full frame/GPU time or baseline-laptop qualification. Known Rust call trail; mixed-language correctness has separate fixtures. No source edits, repository builds or editor launch.'};await writeFile(join(output,'result.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(summary));
} finally {await chromium?.close();await host?.close();}
