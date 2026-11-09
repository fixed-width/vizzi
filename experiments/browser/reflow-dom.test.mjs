import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {browser,paint} from './support.mjs';
const chromium=await browser({desktopBus:false});
try {
 const page=await chromium.newPage({viewport:{width:1200,height:900}});
 await page.route('http://fixture/**',async route=>{const path=new URL(route.request().url()).pathname;if(path==='/')return route.fulfill({contentType:'text/html',body:'<link rel="stylesheet" href="/visual.css"><div id="code" class="code-pane" style="flex:none;width:260px;height:220px"></div>'});return route.fulfill({contentType:path.endsWith('.css')?'text/css':'text/javascript',body:await readFile('crates/vizzi/web'+path,'utf8')});});
 await page.goto('http://fixture/');
 await page.evaluate(async()=>{
  const {SourcePane}=await import('/source-pane.js'),{renderCallText}=await import('/call-actions.js');
  const text='    let value = self.run(first_argument, second_argument, third_argument);';
  const at=s=>new TextEncoder().encode(text.slice(0,text.indexOf(s))).length;
  const line={number:1,text,ending:'\r\n',styles:[],left:false,right:false,reflow:{leading:4,points:[{offset:at('first_argument'),anchor:at('first_argument'),priority:1},{offset:at('second_argument'),anchor:at('first_argument'),priority:0},{offset:at('third_argument'),anchor:at('first_argument'),priority:0}]}};
  const actions=[{id:'a'.repeat(64),name:'run',position:{line:1,start:at('run('),end:at('run(')+3}}];
  window.events=[];window.lines=Array.from({length:60},(_,i)=>({...line,number:i+1}));window.pane=new SourcePane(document.querySelector('#code'),(node,line,plan)=>renderCallText(node,line,actions,(site,element,direct)=>window.events.push([site.id,direct]),plan));window.pane.set(window.lines,{top:0,left:0});
 });
 await page.waitForFunction(()=>document.querySelectorAll('.reflow-segment').length>60);
 assert.equal(await page.locator('.code-line').count(),60);
 assert.equal(await page.locator('.code-call').count(),1);
 await page.locator('.code-call').click({modifiers:['Control']});assert.deepEqual(await page.evaluate(()=>window.events),[['a'.repeat(64),true]]);
 const copied=await page.evaluate(()=>{const code=document.querySelector('#code'),range=document.createRange();range.selectNodeContents(code);getSelection().removeAllRanges();getSelection().addRange(range);const data=new DataTransfer(),event=new ClipboardEvent('copy',{bubbles:true,cancelable:true,clipboardData:data});document.dispatchEvent(event);return {text:data.getData('text/plain'),prevented:event.defaultPrevented,expected:window.lines.map(l=>l.text+l.ending).join('')};});assert.equal(copied.prevented,true);assert.equal(copied.text,copied.expected);
 const selected=await page.locator('.code-text').first().innerHTML();await page.locator('#code').evaluate(n=>n.style.width='900px');await paint(page);assert.equal(await page.locator('.code-text').first().innerHTML(),selected);await page.evaluate(()=>getSelection().removeAllRanges());await page.waitForFunction(()=>document.querySelectorAll('.reflow-segment').length===60);
 await page.locator('#code').evaluate(n=>{n.scrollTop=n.querySelector('[data-line="20"]').offsetTop;});await page.locator('#code').evaluate(n=>n.style.width='220px');await page.waitForFunction(()=>document.querySelectorAll('.reflow-segment').length>60);const anchor=await page.locator('#code').evaluate(n=>[...n.children].find(r=>r.offsetTop+r.offsetHeight>n.scrollTop)?.dataset.line);assert.equal(anchor,'20');
 await page.evaluate(()=>window.pane.setReflow(false));assert.equal(await page.locator('.reflow-segment').count(),60);assert.ok(await page.locator('#code').evaluate(n=>n.scrollWidth>n.clientWidth));
 await page.evaluate(()=>window.pane.setReflow(true));assert.ok(await page.locator('.reflow-segment').count()>60);assert.equal(await page.locator('.code-text').first().textContent(),await page.evaluate(()=>window.lines[0].text));await page.evaluate(()=>{window.savedReading=window.pane.reading();window.pane.dispose();});
 await page.evaluate(async()=>{const {SourcePane}=await import('/source-pane.js'),{renderCallText}=await import('/call-actions.js');const original=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=()=>null;window.pane=new SourcePane(document.querySelector('#code'),(node,line,plan)=>renderCallText(node,line,[],()=>{},plan));HTMLCanvasElement.prototype.getContext=original;window.pane.set(window.lines,window.savedReading);});assert.equal(await page.evaluate(()=>window.pane.reading().anchor.line),await page.evaluate(()=>window.savedReading.anchor.line));assert.equal(await page.locator('.reflow-segment').count(),60);await page.evaluate(()=>window.pane.dispose());
 console.log('source reflow DOM: readable narrow panes, exact CRLF copy, live calls, selection deferral, original mode and source reading anchor');
} finally {await chromium.close();}
