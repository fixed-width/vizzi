import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {writeFile,mkdir} from 'node:fs/promises';
import {join} from 'node:path';
import {buildHost,browser,startHost,workspace,paint} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const code=name=>`// readable comment\n#[derive(Debug)]\npub struct ${name};\npub fn demo() { let n: usize = 42; let s = "literal🙂"; }\n`;
const input=await workspace({files:{'src/compiler/a.rs':code('First'),'src/compiler/b.rs':'fn helper() {}\n','src/runtime/c.rs':'fn runtime() {}\n','tests/check.rs':'fn test_case() {}\n'}});
const git=(...args)=>{const r=spawnSync('git',['-C',input.root,'-c','core.hooksPath=/dev/null','-c','user.name=Fixture','-c','user.email=fixture@example.invalid',...args],{encoding:'utf8',env:{...process.env,GIT_CONFIG_GLOBAL:'/dev/null',GIT_CONFIG_NOSYSTEM:'1'}});assert.equal(r.status,0,r.stderr);return r.stdout.trim();};
let host,chromium;
try{
 git('init','--quiet','--initial-branch=main','--template=');git('add','.');git('commit','--quiet','-m','first');const first=git('rev-parse','HEAD');
 await writeFile(join(input.root,'src/compiler/a.rs'),code('Second'));git('add','.');git('commit','--quiet','-m','second');
 await writeFile(join(input.root,'src/compiler/a.rs'),code('Third'));git('add','.');git('commit','--quiet','-m','third');
 host=await startHost(input,1);chromium=await browser({desktopBus:false});const context=await chromium.newContext({viewport:{width:1600,height:1000},colorScheme:'dark'});const page=await context.newPage(),errors=[];page.setDefaultTimeout(8000);page.on('pageerror',e=>errors.push(e.message));
 await page.goto(host.url);await page.waitForFunction(()=>document.querySelector('#snapshot').textContent.length===8);await paint(page);
 assert.equal(await page.getByLabel('Theme',{exact:true}).inputValue(),'system');
 const background=()=>page.locator('body').evaluate(n=>getComputedStyle(n).backgroundColor==='rgba(0, 0, 0, 0)'?getComputedStyle(document.documentElement).backgroundColor:getComputedStyle(n).backgroundColor);
 assert.equal(await background(),'rgb(12, 19, 30)');
 await page.getByLabel('Theme',{exact:true}).selectOption('light');assert.equal(await background(),'rgb(243, 246, 249)');await page.reload();await page.waitForFunction(()=>document.querySelector('#snapshot').textContent.length===8);assert.equal(await page.getByLabel('Theme',{exact:true}).inputValue(),'light');
 for(const choice of ['dark','light']){
  await page.getByLabel('Theme',{exact:true}).selectOption(choice);await paint(page);
  const colours=await page.evaluate(()=>['keyword','type','function','string','number','comment'].map(kind=>{const node=document.querySelector(`.syntax-${kind}`);if(!node)throw new Error('missing '+kind);return {kind,color:getComputedStyle(node).color,weight:getComputedStyle(node).fontWeight};}));
  assert.equal(new Set(colours.map(c=>c.color)).size,6);assert.ok(Number(colours.find(c=>c.kind==='type').weight)>=650);
 }
 await page.getByLabel('Theme',{exact:true}).selectOption('system');assert.equal(await background(),'rgb(12, 19, 30)');await page.emulateMedia({colorScheme:'light'});assert.equal(await background(),'rgb(243, 246, 249)');
 const snapshot=await page.locator('#snapshot').textContent();await page.getByRole('button',{name:'Dismiss workspace status',exact:true}).click();assert.equal(await page.locator('#status').isVisible(),false);assert.equal(await page.locator('#snapshot').textContent(),snapshot);
 const initial=await page.evaluate(()=>window.vizziMetrics.scene);
 assert.ok(initial.groups.some(g=>g.path==='src' && g.parent==='.'),'directory containment is visible');
 await page.getByRole('button',{name:'Collapse directory src/compiler',exact:true}).click();await paint(page);assert.equal(await page.evaluate(()=>window.vizziMetrics.scene.files.some(f=>f.path.startsWith('src/compiler/'))),false);assert.equal(await page.locator('#file-title').textContent(),'src/compiler/a.rs');
 await page.getByRole('button',{name:'Expand directory src/compiler',exact:true}).click();await paint(page);assert.equal(await page.evaluate(()=>window.vizziMetrics.scene.files.filter(f=>f.path.startsWith('src/compiler/')).length),2);
 await page.getByRole('button',{name:'Move src/runtime earlier',exact:true}).click();assert.ok(await page.evaluate(()=>{const groups=window.vizziMetrics.scene.groups;return groups.findIndex(g=>g.path==='src/runtime')<groups.findIndex(g=>g.path==='src/compiler');}));
 await page.locator('[data-directory="src/compiler"]').dragTo(page.locator('[data-directory="src/runtime"]'));assert.ok(await page.evaluate(()=>{const g=window.vizziMetrics.scene.groups;return g.findIndex(d=>d.path==='src/compiler')<g.findIndex(d=>d.path==='src/runtime');}));
 await page.getByRole('button',{name:'Focus directory src/runtime',exact:true}).focus();await page.keyboard.press('Alt+ArrowUp');assert.ok(await page.evaluate(()=>{const g=window.vizziMetrics.scene.groups;return g.findIndex(d=>d.path==='src/runtime')<g.findIndex(d=>d.path==='src/compiler');}));
 await page.reload();await page.waitForFunction(()=>window.vizziMetrics?.scene);assert.ok(await page.evaluate(()=>{const groups=window.vizziMetrics.scene.groups;return groups.findIndex(g=>g.path==='src/runtime')<groups.findIndex(g=>g.path==='src/compiler');}));
 await page.getByRole('button',{name:'Focus directory src/compiler',exact:true}).click();await paint(page);assert.ok(await page.evaluate(()=>window.vizziMetrics.scene.files.every(f=>f.path.startsWith('src/compiler/'))));await page.getByRole('button',{name:'Repository root',exact:true}).click();
 await page.goto(host.url+'/review');await page.getByLabel('Comparison mode',{exact:true}).selectOption('commits');await page.getByLabel('Base revision',{exact:true}).fill('HEAD^^');await page.getByLabel('Target revision',{exact:true}).fill('HEAD~1');await page.getByRole('button',{name:'Compare commits',exact:true}).click();await page.waitForFunction(()=>document.querySelector('#review-workspace').dataset.token);
 assert.match(await page.locator('#base-source').textContent(),/First/);assert.match(await page.locator('#working-source').textContent(),/Second/);
 const old=await page.locator('#review-workspace').getAttribute('data-token');await page.getByLabel('Base revision',{exact:true}).fill(first.slice(0,8));await page.getByLabel('Target revision',{exact:true}).fill('missing-target');await page.getByRole('button',{name:'Compare commits',exact:true}).click();await page.waitForFunction(()=>!document.querySelector('#comparison-error').hidden);await page.getByRole('button',{name:'Dismiss comparison error',exact:true}).click();assert.equal(await page.locator('#comparison-error').isVisible(),false);assert.equal(await page.locator('#review-workspace').getAttribute('data-token'),old);
 await page.getByLabel('Target revision',{exact:true}).fill('HEAD~1');await page.getByRole('button',{name:'Compare commits',exact:true}).click();await page.waitForFunction(old=>document.querySelector('#review-workspace').dataset.token!==old,old);assert.match(await page.locator('#base-source').textContent(),/First/);
 await page.getByRole('button',{name:'Dismiss comparison details',exact:true}).click();assert.equal(await page.locator('#comparison-labels').isVisible(),false);
 await page.setViewportSize({width:600,height:900});await paint(page);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
 assert.deepEqual(errors,[]);
 if(process.argv.includes('--screenshots')){const dir=process.argv[process.argv.indexOf('--screenshots')+1];await mkdir(dir,{recursive:true});for(const theme of ['dark','light']){await page.setViewportSize({width:1600,height:1000});await page.getByLabel('Theme',{exact:true}).selectOption(theme);await page.screenshot({path:join(dir,theme+'-review.png'),fullPage:true});await page.goto(host.url);await page.waitForFunction(()=>document.querySelector('#snapshot').textContent.length===8);await page.screenshot({path:join(dir,theme+'-map.png'),fullPage:true});await page.goto(host.url+'/review');await page.waitForFunction(()=>document.querySelector('#review-workspace').dataset.token);}}
 console.log('orientation browser: revision syntax, themes, message dismissal, hierarchy/collapse/reordering, persisted layout, focus, source identity and narrow layout passed');
}finally{await chromium?.close();await host?.close();await input.close();}
