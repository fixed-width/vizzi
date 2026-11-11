import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {browser,paint} from './support.mjs';

const chrome=await browser({desktopBus:false});
try {
 const page=await chrome.newPage({viewport:{width:1200,height:900},colorScheme:'light'}),errors=[];
 page.setDefaultTimeout(1500);page.on('pageerror',error=>errors.push(error.message));
 await page.addInitScript(active=>{globalThis.__stryker__={activeMutant:active};},process.env.__STRYKER_ACTIVE_MUTANT__);
 await page.route('http://fixture/**',async route=>{
  const url=new URL(route.request().url()),path=url.pathname;
  const body=path==='/'?`<link rel="stylesheet" href="/theme.css"><link rel="stylesheet" href="/app-header.css"><style>body{margin:0;font:13px sans-serif}</style><header data-app-view="${url.searchParams.get('view')}"><div data-header-controls><details id="actions" class="app-menu"><summary>Actions</summary><div><button id="action">Run action</button></div></details><span>Mode controls</span></div>${url.searchParams.get('view')==='calls'?'<template data-header-settings><label>Extra setting<input id="extra"></label></template>':''}</header><button id="outside">Outside header</button>`:await readFile('crates/vizzi/web'+path,'utf8');
  await route.fulfill({contentType:path==='/'?'text/html':path.endsWith('.css')?'text/css':'text/javascript',body});
 });
 const modes=[['calls','Call workspace','/'],['review','Review changes','/review'],['source','Source browser','/explore'],['locations','Saved locations','/investigate#saved']];
 for(const [mode,label] of modes){
  await page.goto('http://fixture/?view='+mode);
  await page.evaluate(async()=>{
   const {installAppHeader}=await import('/app-header.js');
   window.preservedControls=document.querySelector('[data-header-controls]');window.actions=0;
   document.querySelector('#action').onclick=()=>actions++;
   installAppHeader(document.querySelector('header'));
  });
  const navigation=page.getByRole('navigation',{name:'View modes',exact:true});
  assert.deepEqual(await navigation.getByRole('link').allTextContents(),modes.map(([,text])=>text));
  assert.equal(await navigation.locator('[aria-current="page"]').textContent(),label);
  for(const [,text,path] of modes)assert.equal(await navigation.getByRole('link',{name:text,exact:true}).getAttribute('href'),path);
  assert.equal(await page.getByRole('link',{name:'Vizzi home',exact:true}).getAttribute('href'),'/');
  assert.equal(await page.getByRole('link',{name:'Vizzi home',exact:true}).textContent(),'vizzi');
  assert.deepEqual(await page.evaluate(()=>{
   const brand=document.querySelector('header > div > a'),settings=document.querySelector('#workspace-settings > summary'),current=document.querySelector('[aria-current="page"]'),b=brand.getBoundingClientRect(),s=settings.getBoundingClientRect();
   return {brandSize:getComputedStyle(brand).fontSize,brandWeight:Number(getComputedStyle(brand).fontWeight),selectedBorder:parseFloat(getComputedStyle(current).borderBottomWidth)>0,background:getComputedStyle(document.querySelector('header')).backgroundColor,aligned:Math.abs(b.y+b.height/2-s.y-s.height/2)<2};
  }),{brandSize:'23px',brandWeight:750,selectedBorder:true,background:'rgb(255, 255, 255)',aligned:true});
  assert.equal(await page.evaluate(()=>document.querySelector('[data-header-controls]')===preservedControls),true);
  assert.equal(await page.getByRole('group',{name:label+' controls',exact:true}).count(),1);
  await page.locator('#actions > summary').click();await page.getByRole('button',{name:'Run action',exact:true}).click();assert.equal(await page.evaluate(()=>actions),1);
  await page.locator('#workspace-settings > summary').focus();await page.keyboard.press('Enter');await paint(page);
  assert.equal(await page.locator('#actions').getAttribute('open'),null);
  assert.equal(await page.getByLabel('Theme',{exact:true}).isVisible(),true);
  assert.deepEqual(await page.getByLabel('Theme',{exact:true}).locator('option').allTextContents(),['System','Light','Dark']);
  assert.equal(await page.getByLabel('Theme',{exact:true}).inputValue(),'system');
  assert.equal(await page.locator('#extra').count(),mode==='calls'?1:0);
  if(mode==='calls')assert.equal(await page.getByLabel('Extra setting',{exact:true}).isVisible(),true);
  await page.keyboard.press('x');assert.equal(await page.getByLabel('Theme',{exact:true}).isVisible(),true);
  await page.keyboard.press('Escape');assert.equal(await page.getByLabel('Theme',{exact:true}).isVisible(),false);
  assert.equal(await page.locator('#workspace-settings > summary').evaluate(n=>n===document.activeElement),true);
  await page.keyboard.press('Enter');await page.getByRole('button',{name:'Outside header',exact:true}).click();
  assert.equal(await page.getByLabel('Theme',{exact:true}).isVisible(),false);
  await page.locator('#actions > summary').click();await page.keyboard.press('Escape');
  assert.equal(await page.locator('#actions > summary').evaluate(n=>n===document.activeElement),true);
  await page.evaluate(async()=>{
   const {installAppHeader}=await import('/app-header.js');
   const header=document.createElement('header');header.dataset.appView='unknown';
   let rejected=false;try{installAppHeader(header);}catch(error){if(!error.message.includes('view'))throw error;rejected=true;}if(!rejected)throw Error('Unknown view was accepted');
   header.dataset.appView='calls';rejected=false;try{installAppHeader(header);}catch(error){if(!error.message.includes('controls'))throw error;rejected=true;}if(!rejected)throw Error('Missing controls were accepted');
  });
 }
 assert.deepEqual(errors,[]);
 console.log('header interface: common links/current view, retained controls, theme/extra settings, independent modes, exclusive menus, keyboard/outside dismissal and invalid input');
}finally{await chrome.close();}
