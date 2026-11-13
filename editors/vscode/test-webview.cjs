const fs = require('node:fs/promises');
const assert = require('node:assert/strict');
const {chromium} = require('playwright');

async function verifyWebview({port,report}) {
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
  const workspace = await read(report+'.workspace');
  const callerName = workspace.caller || 'run', calleeName = workspace.callee || 'leaf';
  const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
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
  await frame.getByRole('button',{name:'Add bubble',exact:true}).click();
  await frame.getByLabel('Find a function',{exact:true}).fill(callerName);
  await frame.getByRole('button',{name:'Find functions',exact:true}).click();
  await frame.locator(`[data-add-symbol][data-name="${callerName}"]`).click();
  const caller = frame.locator(`[data-view][data-name="${callerName}"]`);
  await caller.getByRole('button',{name:`Actions for ${calleeName}`,exact:true}).click();
  await frame.getByRole('menuitem',{name:'Open definition',exact:false}).click();
  await frame.locator(`[data-view][data-name="${calleeName}"] .code-line`).first().waitFor();
  assert.equal(await frame.locator('[data-view]').count(),2);
  await frame.locator('#workspace-actions > summary').click();
  await frame.getByRole('button',{name:'Rename task',exact:true}).click();
  await frame.getByLabel('Task name',{exact:true}).fill('Embedded reading');
  await frame.getByRole('button',{name:'Save name',exact:true}).click();
  await fs.writeFile(report+'.webview',JSON.stringify({passed:true}));
  await read(report+'.reopened');
  const reopened = await findFrame(frame);
  await reopened.locator(`[data-view][data-name="${calleeName}"] .code-line`).first().waitFor();
  assert.equal(await reopened.locator('[data-view]').count(),2);
  assert.equal(await reopened.locator('#task-title').textContent(),'Embedded reading');
  await fs.writeFile(report+'.webview-reopened',JSON.stringify({passed:true}));
}

module.exports = {verifyWebview};
