import assert from 'node:assert/strict';
import {readFile,rm} from 'node:fs/promises';
import {join} from 'node:path';
import {test} from 'node:test';
import {buildHost,browser,startHost,workspace,paint} from './support.mjs';

if(!process.argv.includes('--no-build'))buildHost();
const overflowName=index=>`overflow_match_${String(index).padStart(3,'0')}`;
const overflowPath=['bulk',...Array.from({length:7},(_,index)=>index+'x'.repeat(200)),'matches.rs'].join('/');
const input=await workspace({files:{
 'lib.rs':'fn start(){ finish(); }\nfn finish(){}\n',
 'src/reader.rs':Array.from({length:53},(_,index)=>`fn retained_match_${String(index).padStart(2,'0')}() {\n    let _ = 1;\n}\n\n`).join(''),
 [overflowPath]:Array.from({length:288},(_,index)=>`fn ${overflowName(index)}() {}\n`).join(''),
}});
let host,chrome;
try {
 host=await startHost(input,1);chrome=await browser({desktopBus:false});
 const page=await chrome.newPage({viewport:{width:1440,height:1000}}),errors=[];
 page.on('pageerror',error=>errors.push(error.message));
 if(process.argv.includes('--source-assets'))await page.route('**/*',async route=>{
  const path=new URL(route.request().url()).pathname;
  const html={'/':'index.html','/workspace':'visual.html','/review':'review.html','/explore':'index.html','/investigate':'companion.html'};
  const file=html[path]??(/^\/[a-z-]+\.(?:js|css)$/.test(path)?path.slice(1):null);
  if(!file)return route.continue();
  await route.fulfill({contentType:file.endsWith('.html')?'text/html':file.endsWith('.css')?'text/css':'text/javascript',body:await readFile('crates/vizzi/web/'+file,'utf8')});
 });
 await page.goto(host.url);
 await page.waitForFunction(()=>document.querySelector('#snapshot')?.textContent.length===8);
 const modes=[['Source browser','/','source'],['Call workspace','/workspace','calls'],['Review changes','/review','review'],['Saved locations','/investigate#saved','locations']];
 async function checkHeader(mode){
  const navigation=page.getByRole('navigation',{name:'View modes',exact:true});
  await navigation.waitFor({timeout:5000});
  assert.equal(await navigation.count(),1);
  assert.deepEqual(await navigation.getByRole('link').allTextContents(),modes.map(([label])=>label));
  assert.equal(await navigation.locator('[aria-current="page"]').textContent(),mode);
  for(const [label,path] of modes){const link=navigation.getByRole('link',{name:label,exact:true});assert.equal(await link.getAttribute('href'),path);assert.equal(await link.getAttribute('target'),null);}
  assert.equal(await page.getByLabel('Theme',{exact:true}).count(),1);
  await page.locator('#workspace-settings > summary').click();
  assert.equal(await page.getByLabel('Theme',{exact:true}).isVisible(),true);
  await page.keyboard.press('Escape');
  assert.equal(await page.getByLabel('Theme',{exact:true}).isVisible(),false);
  assert.equal(await page.locator('#workspace-settings > summary').evaluate(n=>n===document.activeElement),true);
 }
 await checkHeader('Source browser');
 await page.getByRole('button',{name:'Focus directory src',exact:true}).click();
 const search=page.getByRole('searchbox',{name:'Find source'}), matches=page.getByRole('list',{name:'Source matches'});
 const matchCase=page.getByRole('checkbox',{name:'Match case',exact:true});
 const scope=page.getByRole('checkbox',{name:/In this directory/});
 const destination=matches.getByRole('button',{name:/function retained_match_29 /});
 await search.fill('retained_match');
 await matches.getByText('retained_match_23',{exact:true}).waitFor();
 await matchCase.check();
 await scope.check();
 await page.waitForFunction(()=>document.querySelector('#navigation').getAttribute('aria-busy')==='false');
 await matches.press('End');
 await destination.click();
 await page.locator('#outline [aria-current="true"]').getByText('retained_match_29',{exact:true}).waitFor();
 await page.locator('#source').evaluate(source=>source.scrollTop+=176);
 await page.getByRole('button',{name:'Zoom in',exact:true}).click();
 await paint(page);
 const originalSnapshot=await page.locator('#snapshot').getAttribute('title');
 const sourcePosition=()=>page.evaluate(()=>({
  source:document.querySelector('#source').scrollTop,
  outline:document.querySelector('#outline').scrollTop,
  matches:document.querySelector('#match-list').scrollTop,
  camera:window.vizziMetrics.lastFrame.camera,
 }));
 const reading=await sourcePosition();
 assert.ok(reading.source>0 && reading.outline>0 && reading.matches>0);
 await page.getByRole('navigation',{name:'View modes',exact:true}).getByRole('link',{name:'Call workspace',exact:true}).click();
 await page.waitForFunction(()=>document.querySelector('#visual-source-status')?.textContent.includes('ready'));
 await checkHeader('Call workspace');
 await page.getByRole('button',{name:'Add bubble',exact:true}).click();
 await page.getByLabel('Find a function',{exact:true}).fill('start');
 await page.getByRole('button',{name:'Find functions',exact:true}).click();
 await page.locator('[data-add-symbol][data-name="start"]').click();
 await page.locator('[data-view][data-name="start"] .code-line').first().waitFor();
 const before=await page.locator('[data-view]').evaluateAll(ns=>ns.map(n=>[n.dataset.view,n.style.left,n.style.top,n.style.width,n.style.height]));
 await page.locator('#workspace-settings > summary').click();
 await page.getByLabel('Theme',{exact:true}).selectOption('dark');
 await page.keyboard.press('Escape');
 for(const [label,,mode] of modes.filter(([, ,mode])=>mode!=='calls')){
  await page.getByRole('navigation',{name:'View modes',exact:true}).getByRole('link',{name:label,exact:true}).click();
  await page.locator(`header[data-app-view="${mode}"]`).waitFor();
  await checkHeader(label);
  if(mode==='source'){
   await page.locator('#file-title').getByText('src/reader.rs',{exact:true}).waitFor({timeout:5000});
   await page.locator('#outline [aria-current="true"]').getByText('retained_match_29',{exact:true}).waitFor();
   await paint(page);
   assert.equal(await search.inputValue(),'retained_match');
   assert.equal(await matchCase.isChecked(),true);
   assert.equal(await scope.isChecked(),true);
   assert.equal(await destination.getAttribute('aria-current'),'location');
   assert.equal(await matches.getByRole('button').count(),48);
   const restored=await sourcePosition();
   for(const pane of ['source','outline','matches'])assert.equal(restored[pane],reading[pane]);
   for(const key of ['x','y','scale'])assert.ok(Math.abs(restored.camera[key]-reading.camera[key])<0.01,`restored camera ${key}`);
  }
  if(mode==='locations')assert.equal(await page.getByRole('navigation',{name:'Location views'}).count(),0);
  assert.equal(await page.locator('html').getAttribute('data-theme'),'dark');
  for(const width of [1440,360]){
   await page.setViewportSize({width,height:1000});await paint(page);
   assert.equal(await page.locator('header').evaluate(n=>{const box=n.getBoundingClientRect();return box.left>=0&&box.right<=innerWidth;}),true);
   const outside=await page.locator('header').evaluate(n=>[...n.querySelectorAll('a,summary,button:not(:disabled)')].filter(e=>e.getClientRects().length&&(!e.closest('details:not([open])')||e.tagName==='SUMMARY')).flatMap(e=>{const r=e.getBoundingClientRect();return r.left<0||r.right>innerWidth?[{text:e.textContent,left:r.left,right:r.right}]:[];}));
   assert.deepEqual(outside,[],`${mode} header at ${width}px`);
   await page.locator('#workspace-settings > summary').click();
   assert.equal(await page.getByLabel('Theme',{exact:true}).evaluate(n=>{const r=n.getBoundingClientRect();return r.left>=0&&r.right<=innerWidth;}),true);
   await page.keyboard.press('Escape');
   if(mode==='locations'){
    await page.locator('#locations-actions > summary').click();
    assert.equal(await page.getByRole('button',{name:'Add a location',exact:true}).evaluate(n=>{const r=n.getBoundingClientRect();return r.left>=0&&r.right<=innerWidth;}),true);
    await page.keyboard.press('Escape');
   }
   await page.screenshot({path:`/tmp/vizzi-header-${mode}-${width}.png`});
  }
 }
 await page.setViewportSize({width:1440,height:1000});
 await page.getByRole('navigation',{name:'View modes',exact:true}).getByRole('link',{name:'Call workspace',exact:true}).click();
 await page.locator('[data-view][data-name="start"] .code-line').first().waitFor();
 await checkHeader('Call workspace');
 assert.deepEqual(await page.locator('[data-view]').evaluateAll(ns=>ns.map(n=>[n.dataset.view,n.style.left,n.style.top,n.style.width,n.style.height])),before);
 await page.screenshot({path:'/tmp/vizzi-header-workspace-dark.png'});
 await page.locator('#workspace-settings > summary').click();await page.getByLabel('Theme',{exact:true}).selectOption('light');await page.keyboard.press('Escape');
 await page.screenshot({path:'/tmp/vizzi-header-workspace-light.png'});
 await page.setViewportSize({width:360,height:1000});await paint(page);
 assert.equal(await page.locator('header').evaluate(n=>[...n.querySelectorAll('a,summary,button')].filter(e=>e.getClientRects().length).every(e=>{const r=e.getBoundingClientRect();return r.left>=0&&r.right<=innerWidth;})),true);
 await page.screenshot({path:'/tmp/vizzi-header-calls-360.png'});

 async function withSource(run){
  const sourcePage=await chrome.newPage({viewport:{width:1440,height:1000}});
  sourcePage.setDefaultTimeout(5000);
  const selectView=name=>sourcePage.getByRole('navigation',{name:'View modes',exact:true}).getByRole('link',{name,exact:true}).click();
  const ready=()=>sourcePage.waitForFunction(()=>document.querySelector('#snapshot')?.textContent.length===8);
  try{
   await sourcePage.goto(host.url);
   await ready();
   await run(sourcePage,selectView,ready);
  }finally{await sourcePage.close();}
 }

 await test('clearing a restored search keeps the browsing map revealed before search',()=>withSource(async (page,selectView,ready)=>{
  await page.getByRole('button',{name:'Collapse directory src',exact:true}).click();
  await page.getByText('Accessible file list',{exact:true}).click();
  await page.getByRole('navigation',{name:'Files',exact:true}).getByRole('button',{name:'src/reader.rs',exact:true}).click();
  await page.getByRole('button',{name:'Collapse directory src',exact:true}).waitFor();
  const search=page.getByRole('searchbox',{name:'Find source'});
  await search.fill('retained_match');
  await page.getByRole('list',{name:'Source matches'}).getByText('retained_match_23',{exact:true}).waitFor();
  await selectView('Review changes');
  await selectView('Source browser');
  await ready();
  await search.fill('');
  await page.locator('#source-matches').waitFor({state:'hidden'});
  assert.equal(await page.evaluate(()=>vizziMetrics.scene.files.some(file=>file.path==='src/reader.rs')),true);
 }));

 await test('a search that exceeds the session cache returns to its previous result',()=>withSource(async (page,selectView,ready)=>{
  const matches=page.getByRole('list',{name:'Source matches'});
  await page.getByRole('searchbox',{name:'Find source'}).fill('overflow_match');
  await matches.getByText(overflowName(23),{exact:true}).waitFor();
  for(let offset=24;offset<=240;offset+=24){
   await matches.evaluate(list=>list.scrollTop=list.scrollHeight);
   await matches.getByText(overflowName(offset),{exact:true}).waitFor();
  }
  assert.ok((await matches.getByRole('button').count()-24)*overflowPath.length>256*1024);
  const anchor=matches.getByText(overflowName(237),{exact:true});
  await anchor.evaluate(name=>name.closest('li').scrollIntoView({block:'start',behavior:'instant'}));
  const rowTop=()=>anchor.evaluate(name=>name.closest('li').getBoundingClientRect().top-name.closest('ul').getBoundingClientRect().top);
  const top=await rowTop();
  const offsets=[];
  page.on('request',request=>{
   const url=new URL(request.url());
   if(url.pathname==='/api/matches' && url.searchParams.has('results_offset'))offsets.push(Number(url.searchParams.get('results_offset')));
  });
  await selectView('Saved locations');
  await selectView('Source browser');
  await ready();
  await anchor.waitFor();
  await paint(page);
  assert.ok(Math.abs(await rowTop()-top)<=1);
  assert.equal(offsets[0],216);
  const release=Promise.withResolvers();
  const lastPage=url=>url.pathname==='/api/matches' && url.searchParams.get('results_offset')==='264';
  await page.route(lastPage,async route=>{await release.promise;await route.continue();});
  try{
   const pending=page.waitForRequest(request=>lastPage(new URL(request.url())));
   await matches.evaluate(list=>list.scrollTop=list.scrollHeight);
   await pending;
   await matches.evaluate(list=>list.scrollTop=0);
  }finally{release.resolve();}
  await matches.getByText(overflowName(287),{exact:true}).waitFor();
  await matches.getByText(overflowName(192),{exact:true}).waitFor();
  await anchor.evaluate(name=>name.closest('li').scrollIntoView({block:'start',behavior:'instant'}));
  await selectView('Review changes');
  await selectView('Source browser');
  await ready();
  await anchor.waitFor();
  assert.ok(Math.abs(await rowTop()-top)<=1);
  assert.equal(await matches.getByText(overflowName(0),{exact:true}).count(),0);
 }));

 await test('a fitted map refits to the narrower pane on return',()=>withSource(async (page,selectView,ready)=>{
  const fit=page.getByRole('button',{name:'Fit map',exact:true});
  const camera=()=>page.evaluate(()=>vizziMetrics.lastFrame.camera);
  await fit.click();
  await paint(page);
  const wide=await camera();
  await selectView('Call workspace');
  await page.setViewportSize({width:360,height:750});
  await selectView('Source browser');
  await ready();
  await paint(page);
  const restored=await camera();
  await fit.click();
  await paint(page);
  assert.ok((await camera()).scale<wide.scale);
  assert.deepEqual(restored,await camera());

  await page.getByRole('button',{name:'Zoom in',exact:true}).click();
  const manualPosition=()=>page.evaluate(()=>{
   const {camera,width,height}=vizziMetrics.lastFrame;
   return {scale:camera.scale,x:(width/2-camera.x)/camera.scale,y:(height/2-camera.y)/camera.scale};
  });
  await paint(page);
  const manual=await manualPosition();
  await selectView('Call workspace');
  await page.setViewportSize({width:920,height:900});
  await selectView('Source browser');
  await ready();
  await paint(page);
  const recentered=await manualPosition();
  assert.equal(recentered.scale,manual.scale);
  assert.ok(Math.abs(recentered.x-manual.x)<0.01 && Math.abs(recentered.y-manual.y)<0.01);
 }));

 await rm(join(input.root,'src/reader.rs'));
 await page.waitForFunction(async initial=>{
  const status=await (await fetch('/api/status')).json();
  return status.snapshot && `Captured observation ${status.snapshot}`!==initial && !status.pending;
 },originalSnapshot,{polling:100,timeout:10000});
 await page.getByRole('link',{name:'Vizzi home',exact:true}).click();
 await checkHeader('Source browser');
 await page.getByText('Previously selected file is no longer available: src/reader.rs',{exact:true}).waitFor();
 await page.locator('#file-title').getByText('Choose a file',{exact:true}).waitFor();
 assert.equal(await search.inputValue(),'retained_match');
 assert.deepEqual(errors,[]);
 console.log('shared header: four-view navigation, source and task restoration, missing source, settings, keyboard dismissal, theme persistence and narrow layout');
}finally{await chrome?.close();await host?.close();await input.close();}
