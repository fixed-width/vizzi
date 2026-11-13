const fs = require('node:fs/promises');
const assert = require('node:assert/strict');
const {chromium} = require('playwright');
const {writeSignal}=require('./test-signals.cjs');

async function verifyWebview({port,report,editorCall=false}) {
  const pause = ()=>new Promise(resolve=>setTimeout(resolve,100));
  async function read(file) {
    const deadline = Date.now()+30000;
    for (;;) {
      const value = await fs.readFile(file,'utf8').then(JSON.parse,error=>{
        if (error.code !== 'ENOENT') throw error;
      });
      if (value) return value;
      assert.ok(Date.now()<deadline,`Timed out waiting for ${file}`);
      await pause();
    }
  }
  let browser;
  if(editorCall) {
    const source=await read(report+'.editor-source');
    browser=await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
    const page=browser.contexts().flatMap(context=>context.pages()).find(page=>page.url().startsWith('vscode-file:'));
    assert.ok(page,'VS Code workbench is missing');
    await page.bringToFront();
    const line=page.locator('.monaco-editor .view-line').filter({hasText:source.callee}).first();
    await line.waitFor();
    const point=await line.evaluate((element,name)=>{
      const all=element.textContent,start=all.indexOf(name),end=start+name.length;
      const walker=document.createTreeWalker(element,NodeFilter.SHOW_TEXT),range=document.createRange();
      let node,offset=0,found=false;
      while((node=walker.nextNode())) {
        const next=offset+node.textContent.length;
        if(!found&&start>=offset&&start<next){range.setStart(node,start-offset);found=true;}
        if(found&&end<=next){range.setEnd(node,end-offset);break;}
        offset=next;
      }
      const rect=range.getBoundingClientRect();return {x:rect.x+rect.width/2,y:rect.y+rect.height/2};
    },source.callee);
    await page.mouse.click(point.x,point.y,{button:'left'});
    await page.mouse.click(point.x,point.y,{button:'right'});
    const action=page.getByRole('menuitem',{name:'Vizzi: Explore Call',exact:true});
    await page.screenshot({path:report+'.menu.png'});
    await action.locator('.action-label').click({button:'left'});
    const deadline=Date.now()+30000;
    let frame;
    while(!frame) {
      frame=browser.contexts().flatMap(context=>context.pages()).flatMap(page=>page.frames()).find(frame=>/^http:\/\/127\.0\.0\.1:\d+\//.test(frame.url()));
      assert.ok(Date.now()<deadline,'editor menu did not create a Vizzi frame');
      if(!frame)await pause();
    }
    await frame.locator(`[data-view][data-name="${source.callee}"] .code-line`).first().waitFor();
    assert.equal(await frame.locator('[data-view]').count(),2);
    assert.equal(await frame.locator('#workspace-links .link-stroke').count(),1);
    await writeSignal(report+'.editor-opened',{url:new URL(frame.url()).origin+'/'});
  }
  const workspace = await read(report+'.workspace');
  const callerName = workspace.caller || 'run', calleeName = workspace.callee || 'leaf';
  browser ||= await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  async function findFrame(previous) {
    const deadline = Date.now()+20000;
    for (;;) {
      const frames = browser.contexts().flatMap(context=>context.pages()).flatMap(page=>page.frames());
      const frame = frames.find(frame=>frame!==previous && frame.url().startsWith(workspace.url));
      if (frame) return frame;
      assert.ok(Date.now()<deadline,`Vizzi iframe did not load: ${frames.map(frame=>frame.url()).join(', ')}`);
      await pause();
    }
  }
  const frame = await findFrame();
  await frame.waitForFunction(()=>document.querySelector('#visual-source-status')?.textContent.includes('ready'));
  if(!workspace.editorCall) {
    await frame.getByRole('button',{name:'Add bubble',exact:true}).click();
    await frame.getByLabel('Find a function',{exact:true}).fill(callerName);
    await frame.getByRole('button',{name:'Find functions',exact:true}).click();
    await frame.locator(`[data-add-symbol][data-name="${callerName}"]`).click();
    const caller = frame.locator(`[data-view][data-name="${callerName}"]`);
    await caller.getByRole('button',{name:`Actions for ${calleeName}`,exact:true}).click();
    await frame.getByRole('menuitem',{name:'Open definition',exact:false}).click();
  }
  await frame.locator(`[data-view][data-name="${calleeName}"] .code-line`).first().waitFor();
  assert.equal(await frame.locator('[data-view]').count(),2);
  await frame.locator('#workspace-actions > summary').click();
  await frame.getByRole('button',{name:'Rename task',exact:true}).click();
  await frame.getByLabel('Task name',{exact:true}).fill('Embedded reading');
  await frame.getByRole('button',{name:'Save name',exact:true}).click();
  const previous=workspace.editorCall?await frame.locator('[data-view]').evaluateAll(elements=>elements.map(element=>({name:element.dataset.name,left:element.style.left,top:element.style.top}))):undefined;
  if(workspace.editorCall)await frame.evaluate(()=>{window.vizziDocumentProbe=true;});
  const repeat=workspace.editorCall?frame.page().waitForResponse(response=>response.url().endsWith('/api/call-definition')&&response.request().method()==='POST'):undefined;
  await writeSignal(report+'.webview',{passed:true});
  if(workspace.editorCall) {
    await read(report+'.editor-repeat');
    try {await (await repeat).finished();}
    catch(error){throw new Error(error.message+' '+JSON.stringify(await frame.evaluate(()=>({hash:location.hash,error:document.querySelector('#visual-error')?.textContent,status:document.querySelector('#visual-source-status')?.textContent,probe:window.vizziDocumentProbe}))));}
    assert.equal(await frame.evaluate(()=>window.vizziDocumentProbe),true,'repeated editor navigation reloaded the workspace');
    assert.equal(await frame.locator('[data-view]').count(),2);
    const evidence=await frame.locator('#workspace-links .link-stroke').evaluateAll(elements=>elements.map(element=>element.dataset.linkId));
    assert.ok(evidence.length>=1);
    assert.equal(new Set(evidence).size,evidence.length,'identical relationship evidence was duplicated');
    assert.deepEqual(await frame.locator('[data-view]').evaluateAll(elements=>elements.map(element=>({name:element.dataset.name,left:element.style.left,top:element.style.top}))),previous);
    await writeSignal(report+'.webview-repeat',{passed:true});
  }
  await read(report+'.reopened');
  const reopened = await findFrame(frame);
  await reopened.locator(`[data-view][data-name="${calleeName}"] .code-line`).first().waitFor();
  assert.equal(await reopened.locator('[data-view]').count(),2);
  assert.equal(await reopened.locator('#task-title').textContent(),'Embedded reading');
  await writeSignal(report+'.webview-reopened',{passed:true});
}

module.exports = {verifyWebview};
