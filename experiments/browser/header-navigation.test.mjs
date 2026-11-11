import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {buildHost,browser,startHost,workspace,paint} from './support.mjs';

if(!process.argv.includes('--no-build'))buildHost();
const input=await workspace({files:{'lib.rs':'fn start(){ finish(); }\nfn finish(){}\n'}});
let host,chrome;
try {
 host=await startHost(input,1);chrome=await browser({desktopBus:false});
 const page=await chrome.newPage({viewport:{width:1440,height:1000}}),errors=[];
 page.on('pageerror',error=>errors.push(error.message));
 if(process.argv.includes('--source-assets'))await page.route('**/*',async route=>{
  const path=new URL(route.request().url()).pathname;
  const html={'/':'visual.html','/review':'review.html','/explore':'index.html','/investigate':'companion.html'};
  const file=html[path]??(/^\/[a-z-]+\.(?:js|css)$/.test(path)?path.slice(1):null);
  if(!file)return route.continue();
  await route.fulfill({contentType:file.endsWith('.html')?'text/html':file.endsWith('.css')?'text/css':'text/javascript',body:await readFile('crates/vizzi/web/'+file,'utf8')});
 });
 await page.goto(host.url);
 await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
 const modes=[['Call workspace','/','calls'],['Review changes','/review','review'],['Source browser','/explore','source'],['Saved locations','/investigate#saved','locations']];
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
 for(const [label,,mode] of modes.slice(1)){
  await page.getByRole('navigation',{name:'View modes',exact:true}).getByRole('link',{name:label,exact:true}).click();
  await page.locator(`header[data-app-view="${mode}"]`).waitFor();
  await checkHeader(label);
  if(mode==='locations')assert.equal(await page.locator('#show-saved').evaluate(n=>getComputedStyle(n).backgroundColor!==getComputedStyle(document.querySelector('#show-discovery')).backgroundColor),true);
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
 assert.deepEqual(errors,[]);
 console.log('shared header: four-view navigation, selected mode, consistent settings, keyboard dismissal, theme persistence, narrow layout and task restoration');
}finally{await chrome?.close();await host?.close();await input.close();}
