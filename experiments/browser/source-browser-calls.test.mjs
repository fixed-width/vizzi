import assert from 'node:assert/strict';
import {browser,buildHost,paint,startHost,workspace} from './support.mjs';

if(!process.argv.includes('--no-build'))buildHost();
const wrappedLine='    combine("🙂 one, two, three", first_argument, second_argument, third_argument, finish_wrap());';
const input=await workspace({files:{
 'lib.rs':`fn retained() {}
fn start() {
    let _marker = "🙂 finish()"; finish(); // finish()
    step();
}
fn finish() {}
mod first { fn step() {} }
mod second { fn step() {} }
fn failed_handoff() {}
`,
 'long.rs':`fn long_line() { /*${'x'.repeat(33000)}🙂*/ tail(); }\nfn tail() {}\n`,
 'wrap.rs':['fn wrap_all() {',...Array.from({length:240},(_,i)=>i%4===0?wrappedLine:'    // context'),'}','fn finish_wrap() {}',''].join('\r\n'),
}});
let host,chrome;
try{
 host=await startHost(input,1);
 chrome=await browser({desktopBus:false});
 const page=await chrome.newPage({viewport:{width:1440,height:900}}),errors=[];
 page.setDefaultTimeout(10000);
 page.on('pageerror',error=>errors.push(error.message));
 const bubble=name=>page.locator(`[data-view][data-name="${name}"]`);
 const header=page.getByRole('navigation',{name:'View modes'});
 await page.goto(host.url+'/workspace');
 await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
 await page.getByRole('button',{name:'Add bubble',exact:true}).click();
 await page.getByLabel('Find a function',{exact:true}).fill('retained');
 await page.getByRole('button',{name:'Find functions',exact:true}).click();
 await page.locator('[data-add-symbol][data-name="retained"]').click();
 await bubble('retained').locator('.code-line').waitFor();
 const retained=await bubble('retained').getAttribute('data-view');

 async function browseCode(file='lib.rs'){
  await header.getByRole('link',{name:'Source browser',exact:true}).click();
  await page.getByText('Accessible file list',{exact:true}).click();
  await page.getByRole('navigation',{name:'Files',exact:true}).getByRole('button',{name:file,exact:true}).click();
  await page.waitForFunction(file=>document.querySelector('#navigation').getAttribute('aria-busy')==='false'&&document.querySelector('#file-title').textContent===file,file);
  await page.locator('#source .source-text').first().waitFor();
 }
 const source=page.locator('#source');
 const action=(kind,name)=>source.locator(`.code-${kind}`).and(source.getByRole('button',{name:`Actions for ${name}`,exact:true}));
 const links=page.locator('#workspace-links .link-stroke');
 const highlights=Promise.withResolvers();
 const delayHighlights=async route=>{
  if(route.request().postDataJSON()?.query?.kind==='source_actions')await highlights.promise;
  await route.continue().catch(()=>{});
 };
 await page.route('**/api/structure',delayHighlights);
 try{
  await browseCode();
  assert.match(await source.textContent(),/fn start\(\)/);
  assert.equal(await action('function','start').count(),0);
 }finally{highlights.resolve();}
 await page.unroute('**/api/structure',delayHighlights);
 await action('function','start').press('Enter');
 const openFunction=page.getByRole('menuitem',{name:'Open in Call workspace'});
 await openFunction.waitFor();
 await openFunction.press('Escape');
 assert.equal(await action('function','start').evaluate(node=>node===document.activeElement),true);
 await action('function','start').click();
 await openFunction.click();
 await bubble('start').locator('.code-line').first().waitFor();
 assert.equal(await bubble('retained').getAttribute('data-view'),retained);
 assert.equal(await page.locator('[data-view]').count(),2);
 assert.equal(await links.count(),0);
 const start=await bubble('start').getAttribute('data-view');
 await bubble('start').getByRole('button',{name:'Collapse bubble'}).click();

 await browseCode();
 await action('function','start').press('Control+Enter');
 await bubble('start').locator('.code-line').first().waitFor();
 assert.equal(await bubble('start').getAttribute('data-view'),start);
 assert.equal(await page.locator('[data-view]').count(),2);

 await browseCode();
 const finish=action('call','finish');
 await finish.waitFor();
 assert.equal(await finish.count(),1);
 assert.match(await finish.evaluate(node=>node.closest('.source-text').textContent),/"🙂 finish\(\)"; finish\(\); \/\/ finish\(\)/);
 await finish.click({modifiers:['Control']});
 await bubble('finish').locator('.code-line').waitFor();
 assert.equal(await bubble('start').getAttribute('data-view'),start);
 assert.equal(await bubble('retained').getAttribute('data-view'),retained);
 assert.equal(await links.count(),1);

 await browseCode();
 await action('call','step').click();
 await page.getByRole('menuitem',{name:'Open definition in Call workspace'}).click();
 const chooser=page.locator('#destination-stack');
 await chooser.waitFor();
 assert.match(await chooser.textContent(),/Choose among 2 terminal-name matches/);
 assert.equal(await bubble('step').count(),0);
 await chooser.getByRole('button',{name:/second::step/}).click();
 await bubble('step').locator('.code-line').waitFor();
 assert.equal(await links.count(),2);

 await browseCode('long.rs');
 await source.getByRole('button',{name:'Last text on line 1'}).click();
 const tail=action('call','tail');
 await tail.waitFor({state:'attached'});
 assert.equal(await tail.textContent(),'tail');
 assert.match(await tail.evaluate(node=>node.closest('.source-text').textContent),/^….*🙂\*\/ tail\(\); }$/s);

 const inspection=page.waitForResponse(r=>new URL(r.url()).pathname==='/api/inspection');
 await page.getByRole('navigation',{name:'Files',exact:true}).getByRole('button',{name:'wrap.rs',exact:true}).click();
 assert.ok((await (await inspection).json()).window.lines[1].reflow.points.length>0);
 await source.evaluate(node=>node.style.width='350px');
 const wrapRow=source.locator('[data-line="2"]');
 await wrapRow.locator('.reflow-segment').nth(1).waitFor();
 assert.equal(await wrapRow.evaluate(row=>row.offsetTop+row.offsetHeight===row.nextElementSibling.offsetTop),true);
 const laterCall=wrapRow.getByRole('button',{name:'Actions for finish_wrap',exact:true});
 assert.equal(await laterCall.evaluate(node=>Number(node.closest('.reflow-segment').dataset.start)>0),true);
 const copied=await source.evaluate(node=>{
  const range=document.createRange(),selection=getSelection();
  range.setStart(node.querySelector('[data-line="2"] .source-text'),0);
  range.setEnd(node.querySelector('[data-line="4"] .source-text'),0);
  selection.removeAllRanges();
  selection.addRange(range);
  const clipboardData=new DataTransfer();
  const event=new ClipboardEvent('copy',{bubbles:true,cancelable:true,clipboardData});
  document.dispatchEvent(event);
  return {text:clipboardData.getData('text/plain'),handled:event.defaultPrevented};
 });
 assert.deepEqual(copied,{text:wrappedLine+'\r\n    // context\r\n',handled:true});
 await source.evaluate(node=>node.style.width='850px');
 await paint(page);
 assert.ok(await wrapRow.locator('.reflow-segment').count()>1,'resize preserves the selection');
 await page.evaluate(()=>getSelection().removeAllRanges());
 await page.waitForFunction(()=>document.querySelectorAll('#source [data-line="2"] .reflow-segment').length===1);
 await source.evaluate(node=>{
  node.style.width='350px';
  node.scrollTop=(194-1)*22;
 });
 const distant=source.locator('[data-line="194"]');
 await distant.locator('.reflow-segment').nth(1).waitFor();
 const continuation=await distant.evaluate(row=>row.offsetTop+row.querySelectorAll('.reflow-segment')[1].offsetTop);
 await source.evaluate((node,top)=>{node.scrollTop=top;},continuation);
 await paint(page);
 const topLine=()=>source.evaluate(node=>[...node.querySelectorAll('.code-line')].find(row=>row.offsetTop+row.offsetHeight>node.scrollTop)?.dataset.line);
 assert.equal(await topLine(),'194');
 assert.ok(await source.locator('.code-line').count()<=96);
 await page.getByRole('navigation',{name:'Files',exact:true}).getByRole('button',{name:'lib.rs',exact:true}).click();
 await source.getByText('retained',{exact:true}).waitFor();
 await page.getByRole('button',{name:'Back',exact:true}).click();
 await distant.locator('.reflow-segment').nth(1).waitFor();
 assert.equal(await source.evaluate(node=>node.scrollTop-node.querySelector('[data-line="194"]').offsetTop),22);
 const settings=page.locator('#workspace-settings > summary');
 await settings.click();
 await page.getByLabel('Code layout',{exact:true}).selectOption('original');
 await page.waitForFunction(()=>document.querySelectorAll('#source [data-line="194"] .reflow-segment').length===1);
 assert.equal(await topLine(),'194');
 assert.ok(await source.evaluate(node=>node.scrollWidth>node.clientWidth));
 await settings.click();
 await header.getByRole('link',{name:'Call workspace',exact:true}).click();
 assert.equal(await page.getByLabel('Code layout',{exact:true}).inputValue(),'original');

 await browseCode();
 const failWindow=route=>route.fulfill({status:410,body:'Requested function source expired'});
 await page.route(/\/api\/window\?.*line_start=9(?:&|$)/,failWindow);
 await action('function','failed_handoff').press('Control+Enter');
 await page.waitForURL(host.url+'/workspace');
 await page.getByRole('alert').filter({hasText:'Requested function source expired'}).waitFor();
 assert.equal(await bubble('failed_handoff').count(),0);
 assert.equal(await bubble('retained').getAttribute('data-view'),retained);
 await page.reload();
 await bubble('retained').locator('.code-line').waitFor();
 assert.equal(await bubble('failed_handoff').count(),0);
 assert.deepEqual(errors,[]);
 console.log('Source browser smart wrapping, copying and paging, plus handoffs, highlighting, long-line actions, bubble reuse, and ambiguous destinations passed.');
}finally{
 await chrome?.close();
 await host?.close();
 await input.close();
}
