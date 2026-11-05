import {test,before,after} from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';import {resolve} from 'node:path';
import {browser} from './support.mjs';
import {INSTRUMENTER_CONSTANTS as stryker} from '@stryker-mutator/api/core';
let chromium;before(async()=>{chromium=await browser({desktopBus:false});});after(async()=>{await chromium.close();});
async function fixture({theme='dark',saved,denied=false}={}){
 const page=await chromium.newPage({colorScheme:theme});page.setDefaultTimeout(4000);page.testErrors=[];page.on('pageerror',e=>page.testErrors.push(e.message));
 await page.addInitScript(({saved,denied,namespace,field,active,hitCount,hitLimit})=>{
  globalThis[namespace]={[field]:active,[hitCount]:0,[hitLimit]:1000000};
  if(saved!==undefined)localStorage.setItem('vizzi.theme.v1',saved);
  if(denied)Object.defineProperty(window,'localStorage',{get(){throw new Error('denied');}});
 },{saved,denied,namespace:stryker.NAMESPACE,field:stryker.ACTIVE_MUTANT,active:process.env[stryker.ACTIVE_MUTANT_ENV_VARIABLE],hitCount:stryker.HIT_COUNT,hitLimit:stryker.HIT_LIMIT});
 await page.route('http://fixture/**',async route=>{const path=new URL(route.request().url()).pathname;if(path==='/')return route.fulfill({contentType:'text/html',body:'<meta charset="utf-8"><script src="/theme-init.js"></script><link rel="stylesheet" href="/theme.css"><link rel="stylesheet" href="/app.css"><select id="theme"><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select><div id="message" role="alert">Initial</div><div id="source"><span class="syntax-keyword">pub</span> <span class="syntax-type">Type</span> <span class="syntax-function">call</span> <span class="syntax-string">"text"</span> <span class="syntax-number">42</span> <span class="syntax-comment">// note</span></div><canvas id="map" style="display:none"></canvas>'});return route.fulfill({contentType:path.endsWith('.css')?'text/css; charset=utf-8':'text/javascript; charset=utf-8',body:await readFile(resolve('crates/vizzi/web'+path),'utf8')});});
 await page.goto('http://fixture/');page.bootChoice=await page.evaluate(()=>document.documentElement.dataset.theme ?? null);assert.deepEqual(page.testErrors,[]);await page.evaluate(async()=>{const {installAppearance,mapPalette}=await import('/appearance.js'),{createMessage}=await import('/messages.js');window.changes=[];installAppearance(document.getElementById('theme'),{onChange:mode=>changes.push(mode)});window.message=createMessage(document.getElementById('message'),{label:'Dismiss alert'});window.palette=()=>mapPalette(document.getElementById('map'));});return page;
}
test('System follows CSS/media; valid overrides persist and invalid/stored-refused settings degrade to System',async()=>{
 for(const options of [{theme:'dark'},{theme:'light'},{theme:'dark',saved:'invalid'},{theme:'dark',denied:true}]){const page=await fixture(options);try{
  assert.equal(page.bootChoice,null);assert.equal(await page.locator('#theme').inputValue(),'system');assert.equal(await page.evaluate(()=>getComputedStyle(document.documentElement).colorScheme),options.theme);
  await page.locator('#theme').selectOption('light');assert.equal(await page.evaluate(()=>getComputedStyle(document.documentElement).colorScheme),'light');await page.emulateMedia({colorScheme:'dark'});assert.equal(await page.evaluate(()=>getComputedStyle(document.documentElement).colorScheme),'light');
  await page.locator('#theme').selectOption('system');await page.emulateMedia({colorScheme:'light'});await page.waitForFunction(()=>changes.at(-1)==='light');assert.equal(await page.evaluate(()=>changes.at(-1)),'light');
 }finally{await page.close();}}
 const page=await fixture({saved:'dark',theme:'light'});try{assert.equal(page.bootChoice,'dark');assert.equal(await page.locator('#theme').inputValue(),'dark');assert.equal(await page.evaluate(()=>getComputedStyle(document.documentElement).colorScheme),'dark');await page.locator('#theme').selectOption('light');assert.equal(await page.evaluate(()=>localStorage.getItem('vizzi.theme.v1')),'light');await page.evaluate(()=>window.dispatchEvent(new StorageEvent('storage',{key:'vizzi.theme.v1',newValue:'dark'})));assert.equal(await page.locator('#theme').inputValue(),'dark');}finally{await page.close();}
});
test('dismissed messages stay closed for the same identity, new contexts reopen, and text stays literal',async()=>{
 const page=await fixture();try{
  assert.equal(await page.locator('#message').isVisible(),true);assert.equal(await page.locator('#message').evaluate(n=>getComputedStyle(n).display),'flex');await page.getByRole('button',{name:'Dismiss alert'}).click();await page.evaluate(()=>message.show('Initial'));assert.equal(await page.locator('#message').isVisible(),false);
  await page.evaluate(()=>message.show('<img src=x>','capture-a'));await page.getByRole('button',{name:'Dismiss alert'}).click();await page.evaluate(()=>message.show('<img src=x>','capture-a'));assert.equal(await page.locator('#message').isVisible(),false);
  await page.evaluate(()=>message.show('<img src=x>','capture-b'));assert.equal(await page.locator('#message').isVisible(),true);assert.equal(await page.locator('#message img').count(),0);assert.equal(await page.locator('#message [data-message-text]').textContent(),'<img src=x>');
  await page.getByRole('button',{name:'Dismiss alert'}).focus();await page.keyboard.press('Enter');assert.equal(await page.locator('#message').isVisible(),false);await page.evaluate(()=>message.show('New info','capture-b'));assert.equal(await page.locator('#message').isVisible(),true);await page.evaluate(()=>message.show(null));assert.equal(await page.locator('#message').isVisible(),false);
 }finally{await page.close();}
});
function luminance(rgb){const c=rgb.match(/[\d.]+/g).slice(0,3).map(Number).map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4);return c[0]*.2126+c[1]*.7152+c[2]*.0722;}
test('syntax hues, weight and contrast remain distinct on plain, selection and diff backgrounds in both themes',async()=>{
 const page=await fixture();try{for(const theme of ['light','dark']){
  await page.locator('#theme').selectOption(theme);
  const values=await page.evaluate(()=>{const styles=getComputedStyle(document.documentElement),probe=document.createElement('span');document.body.append(probe);const colors=['paper','active-bg','removed-bg','added-bg'].map(name=>{probe.style.background=`var(--${name})`;return getComputedStyle(probe).backgroundColor;});probe.remove();return {backgrounds:colors,syntax:[...document.querySelectorAll('#source > span')].map(n=>({colour:getComputedStyle(n).color,weight:getComputedStyle(n).fontWeight})),palette:palette()};});
  assert.equal(new Set(values.syntax.map(s=>s.colour)).size,6);assert.ok(Number(values.syntax[1].weight)>=650);assert.ok(values.palette.file.startsWith('#'));assert.notEqual(values.palette.file,values.palette.group);
  for(const span of values.syntax)for(const background of values.backgrounds){const a=luminance(span.colour),b=luminance(background),contrast=(Math.max(a,b)+.05)/(Math.min(a,b)+.05);assert.ok(contrast>=4.5,`${theme}: ${span.colour} on ${background}: ${contrast}`);}
 }}finally{await page.close();}
});

test('theme callbacks report the effective mode, ignore unrelated storage and retain explicit choices on media changes',async()=>{
 const page=await fixture({theme:'dark',saved:'light'});try{
  assert.equal(page.bootChoice,'light');assert.equal(await page.evaluate(()=>changes.at(-1)),'light');
  await page.locator('#theme').selectOption('dark');assert.equal(await page.evaluate(()=>changes.at(-1)),'dark');const count=await page.evaluate(()=>changes.length);
  await page.emulateMedia({colorScheme:'light'});await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));assert.equal(await page.evaluate(()=>changes.length),count);
  await page.evaluate(()=>window.dispatchEvent(new StorageEvent('storage',{key:'unrelated',newValue:'light'})));assert.equal(await page.locator('#theme').inputValue(),'dark');
  await page.locator('#theme').selectOption('system');assert.equal(await page.evaluate(()=>changes.at(-1)),'light');await page.emulateMedia({colorScheme:'dark'});await page.waitForFunction(()=>changes.at(-1)==='dark');
 }finally{await page.close();}
 const dark=await fixture({theme:'dark'});try{assert.equal(await dark.evaluate(()=>changes[0]),'dark');}finally{await dark.close();}
});
test('canvas palettes resolve every shared token in both modes',async()=>{
 const page=await fixture();try{
  const expected={light:{text:'#183044',muted:'#526477',selected:'#265dc5',onSelected:'#ffffff',file:'#c6d7eb',partial:'#ebce91',unavailable:'#cbd4df',group:'#e3eaf3',border:'#b8c7d6',focus:'#265dc5'},dark:{text:'#d9e4f0',muted:'#a9bbd0',selected:'#2863b9',onSelected:'#ffffff',file:'#294463',partial:'#67532e',unavailable:'#39495f',group:'#162438',border:'#3c536d',focus:'#8abdff'}};
  for(const mode of ['light','dark']){await page.locator('#theme').selectOption(mode);assert.deepEqual(await page.evaluate(()=>palette()),expected[mode]);}
 }finally{await page.close();}
});
test('initially empty or hidden messages remain quiet, reuse supplied actions and do not submit forms',async()=>{
 const page=await fixture();try{
  await page.evaluate(async()=>{const {createMessage}=await import('/messages.js');const form=document.createElement('form');form.innerHTML='<div id="extra" hidden><span data-message-text>Existing detail</span><button type="button" id="retry">Retry</button></div><div id="empty"></div>';document.body.append(form);window.submits=0;form.onsubmit=e=>{e.preventDefault();submits++;};window.extra=createMessage(document.getElementById('extra'));window.empty=createMessage(document.getElementById('empty'));});
  assert.equal(await page.locator('#extra').isVisible(),false);assert.equal(await page.locator('#extra [data-message-text]').textContent(),'Existing detail');assert.equal(await page.locator('#empty').isVisible(),false);
  await page.evaluate(()=>extra.show('A problem','a'));assert.equal(await page.locator('#retry').count(),1);assert.equal(await page.locator('#extra [data-message-text]').textContent(),'A problem');
  const button=page.locator('#extra').getByRole('button',{name:'Dismiss message'});assert.equal(await button.evaluate(n=>getComputedStyle(n,'::before').content),'"×"');await button.click();assert.equal(await page.evaluate(()=>submits),0);
  await page.evaluate(()=>{empty.show('');extra.show(null);});assert.equal(await page.locator('#empty').isVisible(),false);assert.equal(await page.locator('#extra').isVisible(),false);
 }finally{await page.close();}
});
