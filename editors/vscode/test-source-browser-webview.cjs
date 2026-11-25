const fs=require('node:fs/promises');
const assert=require('node:assert/strict');
const {chromium}=require('playwright');
const {writeSignal}=require('./test-signals.cjs');

exports.verifySourceBrowser=async({port,report})=>{
  let browser,page;
  const pause=()=>new Promise(r=>setTimeout(r,100));
  try {
    const deadline=Date.now()+30000;let workspace;
    while(!workspace) {
      workspace=await fs.readFile(report+'.workspace','utf8').then(JSON.parse,error=>{if(error.code!=='ENOENT')throw error;});
      assert.ok(Date.now()<deadline,'Source-browser host did not become ready');if(!workspace)await pause();
    }
    browser=await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
    page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url().startsWith('vscode-file:'));
    assert.ok(page);await page.bringToFront();
    const allFrames=()=>browser.contexts().flatMap(c=>c.pages()).flatMap(p=>p.frames());
    let frame;
    while(!(frame=allFrames().find(f=>f.url()===workspace.url+'explore'))){assert.ok(Date.now()<deadline,'Source browser did not load');await pause();}
    await frame.waitForFunction(()=>document.querySelector('#file-title').textContent==='b.py');
    await frame.evaluate(()=>{window.vizziSourceProbe=true;});
    const tab=name=>page.locator('.tabs-container .tab').filter({hasText:name}).first();
    const command=()=>page.getByRole('menuitem',{name:'Vizzi: Open in Source Browser',exact:false});
    async function menu(open,name) {
      let last;
      for(let attempt=0;attempt<3;attempt++) {
        await open();
        try{await command().waitFor({timeout:2000});await page.screenshot({path:report+name});return;}
        catch(error){last=error;await page.keyboard.press('Escape');}
      }
      throw last;
    }
    await tab('a.py').click();
    const line=page.locator('.monaco-editor .view-line').filter({hasText:'# source marker 129'}).first();
    await menu(async()=>{
      const point=await line.evaluate(element=>{const range=document.createRange();range.selectNodeContents(element);const b=range.getBoundingClientRect();return {x:b.x+b.width/2,y:b.y+b.height/2};});
      await page.mouse.click(point.x,point.y);await page.screenshot({path:report+'.focused.png'});await page.mouse.click(point.x,point.y,{button:'right'});
    },'.editor-menu.png');await command().locator('.action-label').click();
    await frame.waitForFunction(()=>document.querySelector('#file-title').textContent==='a.py' && document.querySelector('#source').scrollTop>0);
    await frame.locator('#source .code-line').filter({hasText:'# source marker 129'}).waitFor();
    assert.equal(await frame.evaluate(()=>window.vizziSourceProbe),true,'Repeated handoff must retain the source-browser document');
    const sourceTop=await frame.locator('#source').evaluate(n=>n.scrollTop);
    assert.equal(await frame.locator('#error').isVisible(),false);
    await menu(async()=>{await tab('a.py').click();await page.screenshot({path:report+'.tab-focused.png'});await tab('b.py').click({button:'right'});},'.tab-menu.png');await command().locator('.action-label').click();
    await frame.waitForFunction(()=>document.querySelector('#file-title').textContent==='b.py');
    assert.equal(await frame.evaluate(()=>window.vizziSourceProbe),true);
    await frame.getByRole('button',{name:'Back',exact:true}).click();
    await frame.waitForFunction(top=>document.querySelector('#file-title').textContent==='a.py' && document.querySelector('#source').scrollTop===top,sourceTop);
    await frame.getByRole('button',{name:'Forward',exact:true}).click();
    await frame.waitForFunction(()=>document.querySelector('#file-title').textContent==='b.py');
    await frame.getByRole('link',{name:'Call workspace',exact:true}).click();
    const deadline2=Date.now()+10000;while(!allFrames().find(f=>f.url()===workspace.url+'workspace')){assert.ok(Date.now()<deadline2,'Call workspace did not load');await pause();}
    await menu(async()=>{await tab('a.py').click();await page.screenshot({path:report+'.call-focused.png'});await tab('b.py').click({button:'right'});},'.tab-from-call.png');await command().locator('.action-label').click();
    while(!(frame=allFrames().find(f=>f.url()===workspace.url+'explore'))){assert.ok(Date.now()<deadline2+10000,'Source browser did not reopen');await pause();}
    await frame.waitForFunction(()=>document.querySelector('#file-title').textContent==='b.py');
    assert.equal(await frame.getByRole('button',{name:'Back',exact:true}).isDisabled(),true,'Reopening from another mode starts with the requested file');
    await writeSignal(report+'.webview',{passed:true});
  } catch(error) {
    if(page){await fs.writeFile(report+'.dom.txt',await page.locator('body').innerText());await page.screenshot({path:report+'.failure.png'});}
    throw error;
  } finally{await browser?.close();}
};
