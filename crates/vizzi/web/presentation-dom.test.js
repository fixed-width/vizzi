import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
import {createRequire} from 'node:module';
const require=createRequire(resolve('package.json'));
const {INSTRUMENTER_CONSTANTS:stryker}=await import(pathToFileURL(require.resolve('@stryker-mutator/api/core')));
const {chromium}=await import(pathToFileURL(resolve('node_modules/playwright/index.mjs')));
test('styled fragments preserve exact Unicode and literal markup through DOM replacement and plain fallback',async()=>{
 const browser=await chromium.launch({headless:true,env:{...process.env,DBUS_SESSION_BUS_ADDRESS:'unix:path=/dev/null'}});
 try {
  const page=await browser.newPage();
  await page.addInitScript(({namespace,field,active,hitCount,hitLimit})=>{globalThis[namespace]={[field]:active,[hitCount]:0,[hitLimit]:1000000};},{namespace:stryker.NAMESPACE,field:stryker.ACTIVE_MUTANT,active:process.env[stryker.ACTIVE_MUTANT_ENV_VARIABLE],hitCount:stryker.HIT_COUNT,hitLimit:stryker.HIT_LIMIT});
  await page.route('http://fixture/**',async route=>route.fulfill({contentType:'text/javascript',body:await readFile(new URL('./presentation.js',import.meta.url),'utf8')}));
  await page.goto('http://fixture/');
  const result=await page.evaluate(async()=>{
   const {renderStyledText}=await import('/presentation.js');const node=document.createElement('span');document.body.append(node);
   renderStyledText(node,{text:'a🙂b<img>42',left:true,right:true,styles:[{start:1,end:5,kind:'string'},{start:11,end:13,kind:'number'}]});
   const styled={text:node.textContent,spans:[...node.querySelectorAll('span')].map(n=>[n.className,n.textContent]),images:node.querySelectorAll('img').length};
   renderStyledText(node,{text:'plain🙂',left:false,right:false});const plain={text:node.textContent,spans:node.querySelectorAll('span').length};
   renderStyledText(node,{text:'',left:false,right:true,styles:[]});return {styled,plain,empty:node.textContent};
  });
  assert.deepEqual(result,{styled:{text:'…a🙂b<img>42…',spans:[['syntax-string','🙂'],['syntax-number','42']],images:0},plain:{text:'plain🙂',spans:0},empty:'…'});
 }finally{await browser.close();}
});
