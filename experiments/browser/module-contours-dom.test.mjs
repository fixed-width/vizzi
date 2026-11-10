import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {browser} from './support.mjs';

const chrome=await browser({desktopBus:false});
try{
 const page=await chrome.newPage();await page.addInitScript(active=>{globalThis.__stryker__={activeMutant:active};},process.env.__STRYKER_ACTIVE_MUTANT__);
 let control;
 if(process.argv.includes('--activation-control')){
  const {Instrumenter}=await import('@stryker-mutator/instrumenter'),logger={debug(){},info(){},warn(){},trace(){},error(){},isDebugEnabled:()=>false};const result=await new Instrumenter(logger).instrument([{name:'control.js',mutate:true,content:'export const matches = a => a === 1;'}],{plugins:null,excludedMutations:[],ignorers:[]});control={source:result.files[0].content,id:result.mutants.find(m=>m.replacement.includes('!==')).id};
 }
 await page.route('http://fixture/**',async route=>{const path=new URL(route.request().url()).pathname;await route.fulfill({contentType:path==='/'?'text/html':'text/javascript',body:path==='/'?'<svg id="outlines" width="1000" height="1000"></svg>':path==='/control.js'?control.source:await readFile('crates/vizzi/web'+path,'utf8')});});await page.goto('http://fixture/');
 if(control){assert.deepEqual(await page.evaluate(async id=>{const {matches}=await import('/control.js');const before=matches(1);globalThis.__stryker__.activeMutant=id;const after=matches(1);delete globalThis.__stryker__.activeMutant;return {before,after};},control.id),{before:true,after:false});console.log('browser mutation activation control: unmodified comparison true; selected instrumented mutation false');}
 const outcomes=await page.evaluate(async()=>{
  const {moduleContours}=await import('/module-contours.js'),svg=document.getElementById('outlines');
  const cases=[
   [{module:'a',x:0,y:0,w:200,h:100},{module:'a',x:600,y:0,w:200,h:100},{module:null,x:300,y:0,w:200,h:100}],
   [{module:'a',x:0,y:0,w:200,h:100},{module:'a',x:0,y:600,w:200,h:100},{module:'b',x:600,y:0,w:200,h:100},{module:'b',x:600,y:600,w:200,h:100}],
  ];
  const results=[];
  for(const input of cases)for(const [dx,dy,factor] of [[0,0,1],[-3500,2400,2],[3300,-2000,.5],[12000,-17000,20]]){
   const views=input.map(v=>({...v,x:v.x*factor+dx,y:v.y*factor+dy,w:v.w*factor,h:v.h*factor})),paths=moduleContours(views),checks=[];
   for(const result of paths){const p=document.createElementNS('http://www.w3.org/2000/svg','path');p.setAttribute('d',result.path);p.setAttribute('transform',result.transform);svg.replaceChildren(p);const inverse=p.getCTM().inverse();for(const v of views){const expected=v.module===result.id,points=expected?[[.5,.5],[.1,.1],[.9,.1],[.1,.9],[.9,.9],[0,.5],[1,.5],[.5,0],[.5,1]]:[[.5,.5]];checks.push(...points.map(([x,y])=>({expected,inside:p.isPointInFill(new DOMPoint(v.x+v.w*x,v.y+v.h*y).matrixTransform(inverse))})));}}
   results.push({ids:paths.map(p=>p.id).sort(),expected:[...new Set(views.filter(v=>v.module).map(v=>v.module))].sort(),checks});
  }
  return results;
 });
 for(const result of outcomes){assert.deepEqual(result.ids,result.expected);for(const check of result.checks)assert.equal(check.inside,check.expected);}
 console.log('module contours: actual SVG fill includes members and excludes nonmembers after offset, scaling and orientation changes');
}finally{await chrome.close();}
