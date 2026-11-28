import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
import {createRequire} from 'node:module';
const require=createRequire(resolve('package.json'));
const {INSTRUMENTER_CONSTANTS:stryker}=await import(pathToFileURL(require.resolve('@stryker-mutator/api/core')));
const {chromium}=await import(pathToFileURL(resolve('node_modules/playwright/index.mjs')));
async function withSourcePage(run) {
 const browser=await chromium.launch({headless:true,env:{...process.env,DBUS_SESSION_BUS_ADDRESS:'unix:path=/dev/null'}});
 try {
  const page=await browser.newPage();
  await page.addInitScript(({namespace,field,active,hitCount,hitLimit})=>{globalThis[namespace]={[field]:active,[hitCount]:0,[hitLimit]:1000000};},{namespace:stryker.NAMESPACE,field:stryker.ACTIVE_MUTANT,active:process.env[stryker.ACTIVE_MUTANT_ENV_VARIABLE],hitCount:stryker.HIT_COUNT,hitLimit:stryker.HIT_LIMIT});
  await page.route('http://fixture/**',async route=>{
   const {pathname}=new URL(route.request().url());
   if(pathname==='/')return route.fulfill({contentType:'text/html',body:'<!doctype html><meta charset="utf-8">'});
   return route.fulfill({contentType:'text/javascript',body:await readFile(new URL('.'+pathname,import.meta.url),'utf8')});
  });
  await page.goto('http://fixture/');
  return await run(page);
 }finally{await browser.close();}
}

test('styled fragments preserve exact Unicode and literal markup through DOM replacement and plain fallback',()=>withSourcePage(async page=>{
  const result=await page.evaluate(async()=>{
   const {renderStyledText}=await import('/reflow-text.js');const node=document.createElement('span');document.body.append(node);
   renderStyledText(node,{text:'a🙂b<img>42',left:true,right:true,styles:[{start:1,end:5,kind:'string'},{start:11,end:13,kind:'number'}]});
   const styled={text:node.textContent,spans:[...node.querySelectorAll('span')].map(n=>[n.className,n.textContent]),images:node.querySelectorAll('img').length};
   renderStyledText(node,{text:'plain🙂',left:false,right:false});const plain={text:node.textContent,spans:node.querySelectorAll('span').length};
   renderStyledText(node,{text:'',left:false,right:true,styles:[]});return {styled,plain,empty:node.textContent};
  });
  assert.deepEqual(result,{styled:{text:'…a🙂b<img>42…',spans:[['syntax-string','🙂'],['syntax-number','42']],images:0},plain:{text:'plain🙂',spans:0},empty:'…'});
}));

test('wrapped syntax retains captured spans and encodes a line once',()=>withSourcePage(async page=>{
 const result=await page.evaluate(async()=>{
  const {copySourceSelection,renderReflowText}=await import('/reflow-text.js');
  const {renderCallText}=await import('/call-actions.js');
  const root=document.createElement('div'), first=document.createElement('div'), last=document.createElement('div');
  root.append(first,last);document.body.append(root);
  const prefix='\t  ';
  const parts=[prefix+'// 🙂 ','\uFEFF café ','終; ','42 <b>'];
  const encoder=new TextEncoder();
  const offsets=[0];
  for(const part of parts)offsets.push(offsets.at(-1)+encoder.encode(part).length);
  const text=parts.join('');
  const line={text,ending:'\r\n',styles:[
   {start:0,end:1,kind:'keyword'},
   {start:1,end:offsets[3]-1,kind:'comment'},
   {start:offsets[3],end:offsets[3]+2,kind:'number'},
  ]};
  const margin={bytes:prefix.length,columns:2};
  const plan=parts.map((_,index)=>({start:offsets[index],end:offsets[index+1],indent:index*8}));
  const encode=TextEncoder.prototype.encode;
  function countEncodes(render){
   let count=0;
   TextEncoder.prototype.encode=function(...args){count++;return encode.apply(this,args);};
   try{render();return count;}
   finally{TextEncoder.prototype.encode=encode;}
  }
  const encodes=countEncodes(()=>renderReflowText(first,line,plan,margin));
  const withoutActions=document.createElement('div');
  const encodesWithoutActions=countEncodes(()=>renderCallText(withoutActions,line,[],undefined,plan,margin));

  const tail={text:'tail',ending:''};
  renderReflowText(last,tail);
  const range=document.createRange();
  range.setStart(first.querySelector('.syntax-comment').firstChild,0);
  range.setEndAfter(last);
  const selection=getSelection();selection.removeAllRanges();selection.addRange(range);
  const clipboardData=new DataTransfer();
  const event=new ClipboardEvent('copy',{cancelable:true,clipboardData});
  copySourceSelection(root,[{line,text:first,margin},{line:tail,text:last}],event);
  const clipped=document.createElement('span');
  renderReflowText(clipped,{text:'🙂',left:true,right:true,styles:[{start:0,end:4,kind:'string'}]});
  const segments=[...first.querySelectorAll('.reflow-segment')];
  return {
   encodes,encodesWithoutActions,text:first.textContent,
   offsets:segments.map(segment=>Number(segment.dataset.start)),
   markers:segments.map(segment=>Array.from(segment.querySelectorAll(':scope > .reflow-marker'),
    marker=>marker.getAttribute('aria-hidden'))),
   comments:[...first.querySelectorAll('.syntax-comment')].map(node=>node.textContent),
   number:first.querySelector('.syntax-number')?.textContent, markup:first.querySelector('b')!==null,
   copied:clipboardData.getData('text/plain'),handled:event.defaultPrevented,
   clipped:clipped.textContent,clipMarks:clipped.querySelectorAll('.source-clip').length,
  };
 });
 assert.deepEqual(result.comments,['// 🙂 ','\uFEFF café ','終;']);
 assert.equal(result.number,'42');
 assert.deepEqual(result.offsets,[0,11,21,26]);
 assert.deepEqual(result.markers,[[],['true'],['true'],['true']],
  'only continuation segments have a decorative wrap marker');
 assert.equal(result.text,'\t  // 🙂 \uFEFF café 終; 42 <b>');
 assert.equal(result.markup,false);
 assert.equal(result.handled,true);
 assert.equal(result.copied,result.text+'\r\ntail');
 assert.equal(result.clipped,'…🙂…');
 assert.equal(result.clipMarks,2);
 assert.equal(result.encodes,1,'the rendered line is encoded once across all four segments');
 assert.equal(result.encodesWithoutActions,1,'the call adapter skips decoration when there are no actions');
}));
