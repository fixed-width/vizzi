const fs=require('node:fs/promises');
const assert=require('node:assert/strict');
const {chromium}=require('playwright');
const {writeSignal}=require('./test-signals.cjs');

exports.verifyComparisonWebview=async({port,report})=>{
  let browser,page;
  const pause=()=>new Promise(r=>setTimeout(r,100));
  try {
    const deadline=Date.now()+30000;let workspace;
    while(!workspace) {
      workspace=await fs.readFile(report+'.workspace','utf8').then(JSON.parse,error=>{if(error.code!=='ENOENT')throw error;});
      assert.ok(Date.now()<deadline,'Comparison host did not become ready');
      if(!workspace)await pause();
    }
    browser=await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
    page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url().startsWith('vscode-file:'));
    assert.ok(page);await page.bringToFront();
    async function review() {
      const deadline=Date.now()+15000;
      for(;;) {
        const frame=browser.contexts().flatMap(c=>c.pages()).flatMap(p=>p.frames()).find(f=>f.url()===workspace.url+'review');
        if(frame){await frame.locator('#review-workspace[data-token]:not([data-token=""])').waitFor();return frame;}
        assert.ok(Date.now()<deadline,'Embedded review did not load');await pause();
      }
    }
    let frame=await review();
    const paths=await frame.locator('#review-file-list button').allTextContents();
    for(const file of ['a.py','staged.py','untracked.py'])assert.ok(paths.some(p=>p.startsWith(file)),JSON.stringify(paths));
    await frame.locator('#review-file-list button').filter({hasText:'a.py'}).click();
    await frame.locator('#working-source').getByText('def working(): return 3',{exact:false}).waitFor();
    async function menu(command) {
      await page.locator('.tabs-container .tab').filter({hasText:'a.py'}).first().click();
      const header=page.locator('.part.sidebar .pane-header').first();
      await header.hover();
      await header.getByRole('button',{name:'More Actions...',exact:true}).click();
      const item=page.getByRole('menuitem',{name:command,exact:false});
      await item.waitFor();await page.screenshot({path:report+'.menu.png'});await item.locator('.action-label').click();
    }
    await menu('Vizzi: Compare Working Changes with HEAD');
    frame=await review();
    await frame.waitForFunction(token=>document.querySelector('#review-workspace').dataset.token!==token,workspace.token);
    const workingToken=await frame.locator('#review-workspace').getAttribute('data-token');
    await menu('Vizzi: Compare Revisions');
    const input=page.locator('.quick-input-widget input');
    await input.waitFor();await input.fill('First version');
    await page.locator('.quick-input-list .monaco-list-row').filter({hasText:'First version'}).click();
    await page.locator('.quick-input-title').filter({hasText:'(2/2)'}).waitFor();
    await input.fill('Second version');
    await page.locator('.quick-input-list .monaco-list-row').filter({hasText:'Second version'}).click();
    frame=await review();
    await frame.waitForFunction(token=>document.querySelector('#review-workspace').dataset.token!==token,workingToken);
    await frame.locator('#comparison-labels').getByText(`Target ${workspace.target}`,{exact:false}).waitFor();
    const status=await frame.evaluate(()=>fetch('/api/comparisons/status').then(r=>r.json()));
    assert.equal(status.active.resolved,workspace.base);assert.equal(status.active.target.resolved,workspace.target);
    assert.equal(await frame.locator('#comparison-mode').inputValue(),'commits');
    const files=await frame.locator('#review-file-list button').allTextContents();
    assert.deepEqual(files,['a.pymodified']);
    await frame.locator('#working-source').getByText('def added(): return 2',{exact:false}).waitFor();
    assert.equal(await frame.locator('#refresh-working').isVisible(),false);
    await page.screenshot({path:report+'.comparison.png'});
    await writeSignal(report+'.webview',{passed:true});
  } catch(error) {
    if(page){await fs.writeFile(report+'.dom.txt',await page.locator('body').innerText());await fs.writeFile(report+'.actions.json',JSON.stringify(await page.locator('.action-label.codicon-more').evaluateAll(nodes=>nodes.map(n=>({label:n.getAttribute('aria-label'),html:n.outerHTML,parent:n.parentElement.parentElement.parentElement.outerHTML.slice(0,1800)}))),null,2));await page.screenshot({path:report+'.failure.png'});}
    throw error;
  } finally {await browser?.close();}
};
