// Optimized-host request/render observations for the already-known development questions.
import assert from 'node:assert/strict';
import {mkdir, readFile, writeFile, chmod} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {cpus, platform, arch} from 'node:os';
import {join, resolve} from 'node:path';
import {browser, startHost} from '../browser/support.mjs';
const args=process.argv.slice(2), option=name=>args[args.indexOf(name)+1];
assert.ok(args.includes('--source')&&args.includes('--output'),'--source and --output are required');
const root=resolve(option('--source')),output=resolve(option('--output'));
await mkdir(output);
const editor=join(output,'record-editor');
await writeFile(editor,'#!/bin/sh\nprintf \'%s\\n\' \"$@\" >> \"$0.args\"\n');
await chmod(editor,0o700);
const cohort=JSON.parse(await readFile('experiments/qa/library/search-roles/cohort.json','utf8'));
const questions=cohort.cases.filter(c=>c.group==='maintainer');
assert.equal(questions.length,3);
let host,chromium;
try {
 host=await startHost({root,cache:join(output,'analysis'),sourceArguments:['--editor-code',editor]},1);
 const api=async(path,body)=>{
  const start=performance.now(),response=await fetch(host.url+path,body===undefined?{}:{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}),text=await response.text(),ms=performance.now()-start;
  assert.ok(response.ok,text);return{data:JSON.parse(text),ms,bytes:Buffer.byteLength(text)};
 };
 const activated=performance.now();await api('/api/workspace/start',{});
 let status;
 for(let i=0;;i++){
  status=(await api('/api/status')).data;if(status.snapshot)break;
  assert.ok(i<600&&!status.error,'source activation failed');await new Promise(done=>setTimeout(done,100));
 }
 const activationMs=performance.now()-activated;
 const cold=await api('/api/search',{snapshot:status.snapshot,question:questions[0].question,selector:'focused'});
 const observations=[];
 for(const selector of ['focused','original'])for(const question of questions){
  const samples=[],responseBytes=[];let last;
  for(let i=0;i<35;i++){
   const result=await api('/api/search',{snapshot:status.snapshot,question:question.question,selector});last=result.data;
   assert.equal(last.snapshot,status.snapshot);assert.ok(last.results.length<=8);assert.ok(last.results.reduce((n,r)=>n+Buffer.byteLength(r.excerpt),0)<=4000);
   if(i>=5){samples.push(result.ms);responseBytes.push(result.bytes);}
  }
  for(const hit of last.results)assert.ok((await readFile(join(root,hit.path),'utf8')).includes(hit.excerpt),'excerpt must be literal captured source');
  const sorted=[...samples].sort((a,b)=>a-b);
  observations.push({id:question.id,question:question.question,selector,samplesMs:samples,medianMs:sorted[15],p95Ms:sorted[28],maxMs:sorted[29],maxResponseBytes:Math.max(...responseBytes),...last});
 }
 chromium=await browser({desktopBus:false});
 const page=await chromium.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(host.url+'/investigate?trial=search');await page.waitForFunction(()=>document.querySelector('#source-state').textContent.includes('ready'));
 const browserSamples=[];
 for(let i=0;i<35;i++){
  const question=questions[i%3].question;
  const elapsed=await page.evaluate(async question=>{
   document.getElementById('source-question').value=question;
   const start=performance.now();document.getElementById('source-question-form').requestSubmit();
   await new Promise(done=>{const root=document.getElementById('search-results');const check=()=>{if(root.getAttribute('aria-busy')==='false'&&root.querySelector('[data-search-result]')){observer.disconnect();done();}};const observer=new MutationObserver(check);observer.observe(root,{attributes:true,childList:true,subtree:true});check();});
   await new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(done)));return performance.now()-start;
  },question);
  if(i>=5)browserSamples.push(elapsed);
 }
 await page.screenshot({path:join(output,'known-question.png')});
 const workflowQuestion=questions[0].question;
 await page.getByLabel('Ask about this codebase',{exact:true}).fill(workflowQuestion);
 await page.getByRole('button',{name:'Search sources',exact:true}).click();
 await page.waitForFunction(()=>document.querySelector('#search-results').getAttribute('aria-busy')==='false');
 const locations=await page.locator('[data-search-result]').evaluateAll(nodes=>nodes.map(n=>({id:n.dataset.searchResult,path:n.querySelector('.search-location').textContent})));
 const selected=[];for(const location of locations)if(!selected.some(s=>s.path.split(' · ')[0]===location.path.split(' · ')[0])){selected.push(location);if(selected.length===2)break;}
 assert.equal(selected.length,2,'known workflow should offer at least two files');
 for(const [position,location] of selected.entries()){const card=page.locator(`[data-search-result="${location.id}"]`);await card.getByRole('button',{name:'Inspect source',exact:true}).click();await page.locator('#preview-source').waitFor({state:'visible'});await card.getByRole('button',{name:'Keep this location',exact:true}).click();await page.waitForFunction(count=>document.querySelector('#show-saved').textContent.includes(String(count)),position+1);await page.locator('#close-preview').click();}
 await page.getByRole('button',{name:'Keep question',exact:true}).click();
 await page.waitForFunction(()=>document.querySelector('#search-keep-question').disabled);
 const board=(await api('/api/investigation')).data.board;assert.equal(board.question,workflowQuestion);assert.equal(board.evidence.length,2);
 await page.locator('[data-search-result]').filter({has:page.getByRole('button',{name:'Kept',exact:true})}).first().getByRole('button',{name:'Open in VS Code',exact:true}).click();
 await page.locator('#investigation-notice').waitFor({state:'visible'});const handoff=await readFile(editor+'.args','utf8');assert.ok(handoff.includes('--goto\n'));
 const workflow={question:workflowQuestion,authorship:'Known source-aware functional replay, not a fresh maintainer task',board,recordingOnlyHandoff:handoff};
 const sorted=[...browserSamples].sort((a,b)=>a-b);assert.deepEqual(errors,[]);
 const report={schema:1,createdAt:new Date().toISOString(),sourceRoot:root,sourceSnapshot:status.snapshot,environment:{os:platform(),arch:arch(),cpu:cpus()[0].model},workers:1,model:null,binarySha256:createHash('sha256').update(await readFile('target/release/vizzi')).digest('hex'),activationMs,coldQueryMs:cold.ms,scope:{availableFiles:cold.data.available_files,searchedFiles:cold.data.searched_files,omittedFiles:cold.data.omitted_files},observations,workflow,browser:{version:chromium.version(),samplesMs:browserSamples,medianMs:sorted[15],p95Ms:sorted[28],maxMs:sorted[29],errors},limitations:[
  'Known development questions only; no fresh-question relevance or maintainer acceptance claim.',
  'One optimized native host, one indexing worker, no model, no competing test/mutation cohort.',
  'Thirty warm localhost/full-body observations after five warmups per question/selector; JSON parsing follows request timing.',
  'Browser submission through two animation frames includes DOM/network/frame scheduling, rather than physical input or hardware frame-time capture.',
  'Pinned lpc-rs source scope only; cache preparation is separated from warm retrieval. No large-repository qualification.',
 ]};
 await writeFile(join(output,'report.json'),JSON.stringify(report,null,2)+'\n');
 console.log(JSON.stringify({activationMs,coldQueryMs:report.coldQueryMs,scope:report.scope,requests:observations.map(o=>({id:o.id,selector:o.selector,medianMs:o.medianMs,p95Ms:o.p95Ms})),browser:report.browser}));
}finally{await chromium?.close();await host?.close();}
