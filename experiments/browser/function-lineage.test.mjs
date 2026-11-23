import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {unlink,writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {browser,buildHost,paint,startHost,workspace} from './support.mjs';

if(!process.argv.includes('--no-build'))buildHost();
const input=await workspace({files:{'earlier.rs':'fn before() { helper(); }\nfn helper() { let older_helper=1; }\n'}});
function git(...args){
 const result=spawnSync('git',['-C',input.root,...args],{encoding:'utf8'});
 assert.equal(result.status,0,result.stderr);
 return result.stdout.trim();
}
function commit(){
 git('add','.');git('-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','fixture');
 return git('rev-parse','HEAD');
}

let host,chromium;
try{
 git('init','-q');const older=commit();
 await unlink(join(input.root,'earlier.rs'));
 await writeFile(join(input.root,'current.rs'),'fn after() { helper(); }\nfn helper() { let newer_helper=1; }\n');
 const newer=commit();
 await writeFile(join(input.root,'earlier.rs'),'fn before() { unrelated_live_source(); }\n');
 await writeFile(join(input.root,'current.rs'),'fn after() { helper(); }\nfn helper() { let live_helper=1; }\n');
 host=await startHost(input,1);
 chromium=await browser({desktopBus:false});
 const page=await chromium.newPage({viewport:{width:1580,height:1030}}),errors=[];
 page.on('pageerror',error=>errors.push(error.message));
 await page.goto(host.url+'/workspace');
 await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));
 await page.getByRole('button',{name:'Add bubble',exact:true}).click();
 await page.getByLabel('Find a function',{exact:true}).fill('after');
 await page.getByRole('button',{name:'Find functions',exact:true}).click();
 await page.locator('[data-add-symbol][data-name="after"]').click();
 const bubble=page.locator('[data-view][data-name="after"]');
 await bubble.locator('.code-line').first().waitFor();
 const id=await bubble.getAttribute('data-view'),view=page.locator(`[data-view="${id}"]`);
 const rect=await view.evaluate(node=>node.style.cssText);
 await view.getByRole('button',{name:'Choose revision…',exact:true}).click();
 const revision=page.getByLabel('Local commit ID, branch or tag');
 await revision.fill(newer);await revision.press('Enter');
 await page.getByRole('dialog',{name:'Choose revision'}).waitFor({state:'hidden'});
 await view.getByRole('button',{name:'Actions for helper',exact:true}).click({modifiers:['Control']});
 const helper=page.locator('[data-view][data-name="helper"]');
 await helper.locator('.code-line').first().waitFor();
 assert.match(await helper.locator('.code-pane').textContent(),/newer_helper/);

 const dialog=page.getByRole('dialog',{name:'Find earlier declaration'});
 const actions=view.getByRole('button',{name:'Actions for after bubble',exact:true});
 const open=async()=>{
  await actions.press('Enter');
  await page.getByRole('menuitem',{name:'Find earlier declaration…',exact:true}).press('Enter');
 };
 const waiting=Promise.withResolvers(),release=Promise.withResolvers(),delivered=Promise.withResolvers();
 const delay=async route=>{
  if(route.request().postDataJSON().kind!=='preview')return route.continue();
  const response=await route.fetch();waiting.resolve();await release.promise;
  await route.fulfill({response}).catch(()=>{});delivered.resolve();
 };
 await page.route('**/api/function-history/lineage',delay);
 try{
  await open();await waiting.promise;
  await page.keyboard.press('Escape');await dialog.waitFor({state:'hidden'});
  assert.equal(await actions.evaluate(button=>button===document.activeElement),true);
  release.resolve();await delivered.promise;await paint(page);
  assert.equal(await view.getAttribute('data-name'),'after');
 }finally{release.resolve();await page.unroute('**/api/function-history/lineage',delay);}

 await open();
 const accept=dialog.getByRole('button',{name:'Use earlier declaration',exact:true});
 await dialog.getByText(new RegExp(`before · earlier.rs:1 · ${older}`)).waitFor();
 assert.equal(await view.getAttribute('data-name'),'after');
 await accept.press('Enter');await dialog.waitFor({state:'hidden'});
 assert.equal(await view.getAttribute('data-name'),'before');
 assert.equal(await view.evaluate(node=>node.style.cssText),rect);
 assert.match(await view.locator('.fragment-heading small').textContent(),/earlier\.rs/);
 assert.match(await view.locator('.code-pane').textContent(),/fn before\(\) \{ helper/);
 assert.match(await page.locator('#workspace-links .link-stroke').getAttribute('aria-label'),/Earlier call/);
 await page.reload();await view.locator('.code-line').first().waitFor();
 assert.equal(await view.getAttribute('data-name'),'before');
 assert.match(await view.locator('.code-pane').textContent(),/fn before\(\) \{ helper/);

 await view.getByRole('button',{name:'Next function change',exact:true}).click();
 await page.waitForFunction(id=>document.querySelector(`[data-view="${id}"]`).dataset.name==='after',id);
 await view.getByRole('button',{name:'Compare with previous version',exact:true}).click();
 const before=page.locator('[data-comparison-side="older"]'),after=page.locator('[data-comparison-side="newer"]');
 await before.locator('.comparison-removed').first().waitFor();
 assert.match(await before.locator('.fragment-history span').getAttribute('title'),new RegExp(older));
 assert.match(await after.locator('.fragment-history span').getAttribute('title'),new RegExp(newer));
 assert.match(await before.locator('.fragment-heading small').textContent(),/earlier\.rs/);
 assert.match(await after.locator('.fragment-heading small').textContent(),/current\.rs/);
 await page.getByRole('button',{name:'Add bubble',exact:true}).click();
 await page.getByLabel('Find a function',{exact:true}).fill('helper');
 await page.getByRole('button',{name:'Find functions',exact:true}).click();
 await page.locator('[data-add-symbol][data-name="helper"]').click();
 const liveHelper=page.locator('[data-view][data-name="helper"]:not([data-captured-file])');
 await liveHelper.getByRole('button',{name:'Actions for helper bubble',exact:true}).click();
 await page.getByRole('menuitem',{name:'Find callers',exact:true}).click();
 const showPair=page.locator('[data-reference-stack]').getByRole('button',{name:'Show comparison',exact:true});
 await page.getByRole('button',{name:'Fit workspace',exact:true}).click();
 for(const side of [before,after])await side.getByRole('button',{name:'Collapse bubble',exact:true}).click();
 await showPair.press('Enter');
 assert.equal(await page.locator('[data-comparison-side] .code-pane:visible').count(),2);
 await after.getByRole('button',{name:'Close bubble',exact:true}).click();
 await before.getByRole('button',{name:'Collapse bubble',exact:true}).click();
 await showPair.press('Enter');
 assert.equal(await before.locator('.code-pane').isVisible(),true);
 const olderId=await before.getAttribute('data-view'),olderView=page.locator(`[data-view="${olderId}"]`);
 await before.getByRole('button',{name:'Return to live',exact:true}).click();
 await page.waitForFunction(id=>document.querySelector(`[data-view="${id}"]`).dataset.name==='after',olderId);
 assert.match(await olderView.locator('.fragment-heading small').textContent(),/current\.rs/);
 assert.match(await olderView.locator('.code-pane').textContent(),/fn after\(\) \{ helper/);
 assert.deepEqual(errors,[]);
 console.log('function lineage: explicit rename and move, canceled preview, saved call evidence, retained comparison sides and original live binding');
}finally{
 await chromium?.close();await host?.close();await input.close();
}
