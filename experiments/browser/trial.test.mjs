// Maintainer feedback exercised through the native host and real browser.
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {writeFile,mkdir} from 'node:fs/promises';
import {join} from 'node:path';
import {buildHost,browser,startHost,workspace,paint} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const before='// note 🙂\npub struct Before;\npub fn demo() { let n = 42; let s = "<img src=x onerror=alert(1)>"; }\n';
const after=before.replace('Before','After');
const files={'src/a.rs':before};for(let i=0;i<40;i++)files[`part${i}/file.rs`]=`pub fn demo_${i}() {}\n`;
const input=await workspace({files});
const git=(...args)=>{const result=spawnSync('git',['-C',input.root,'-c','core.hooksPath=/dev/null','-c','user.name=Fixture','-c','user.email=fixture@example.invalid',...args],{encoding:'utf8',env:{...process.env,GIT_CONFIG_GLOBAL:'/dev/null',GIT_CONFIG_NOSYSTEM:'1'}});assert.equal(result.status,0,result.stderr);return result.stdout.trim();};
let host,chromium;
try{
 git('init','--quiet','--initial-branch=main','--template=');git('add','.');git('commit','--quiet','-m','base');const base=git('rev-parse','HEAD');
 await writeFile(join(input.root,'src/a.rs'),after);git('add','.');git('commit','--quiet','-m','target');const target=git('rev-parse','HEAD');
 await writeFile(join(input.root,'src/a.rs'),'fn dirty_working() {}\n');
 host=await startHost(input,1);chromium=await browser({desktopBus:false});const page=await chromium.newPage({viewport:{width:1600,height:1000}}),errors=[];page.setDefaultTimeout(10000);page.on('pageerror',e=>errors.push(e.message));
 await page.goto(host.url+'/review');
 const submit=async()=>{await page.locator('#compare').click();};
 await page.getByLabel('Base revision',{exact:true}).fill('missing-base');await submit();await page.waitForFunction(()=>!document.querySelector('#comparison-error').hidden);assert.match(await page.locator('#comparison-error').textContent(),/Base revision/);
 await page.getByLabel('Base revision',{exact:true}).fill(base);await page.getByLabel('Comparison mode',{exact:true}).selectOption('commits');await page.getByLabel('Target revision',{exact:true}).fill('missing-target');await submit();await page.waitForFunction(()=>document.querySelector('#comparison-error').textContent.includes('Target revision'));
 await page.getByLabel('Target revision',{exact:true}).fill(target);await page.getByLabel('Repository path',{exact:true}).fill('does-not-exist');await submit();await page.waitForFunction(()=>document.querySelector('#comparison-error').textContent.includes('Repository path'));
 await page.getByLabel('Repository path',{exact:true}).fill('.');await submit();await page.waitForFunction(()=>document.querySelector('#review-workspace').dataset.token);await paint(page);
 const pair=await page.locator('#review-workspace').getAttribute('data-token');
 assert.equal(await page.locator('#comparison-error').isVisible(),false);assert.match(await page.locator('#comparison-labels').textContent(),new RegExp(target));
 assert.match(await page.locator('#base-source').textContent(),/Before/);assert.match(await page.locator('#working-source').textContent(),/After/);assert.doesNotMatch(await page.locator('#working-source').textContent(),/dirty_working/);assert.equal(await page.locator('#select-working').textContent(),'Target');assert.equal(await page.locator('#refresh-working').isVisible(),false);
 assert.ok(await page.locator('#base-source .syntax-keyword').count());assert.ok(await page.locator('#working-source .syntax-string').count());assert.equal(await page.locator('#working-source img').count(),0);assert.ok(await page.locator('#working-outline .kind-type').count());assert.match(await page.locator('#working-outline').textContent(),/Line 2/);assert.match(await page.locator('#working-outline button').first().getAttribute('aria-label'),/^Reveal struct After starting at Line 2$/);assert.equal(await page.locator('footer').count(),0);assert.equal(await page.locator('#file-page-label').textContent(),'1 file');
 const splitters=page.getByRole('separator');assert.equal(await splitters.count(),4);
 for(let i=0;i<4;i++){const h=splitters.nth(i),old=Number(await h.getAttribute('aria-valuenow'));await h.focus();await page.keyboard.press('ArrowRight');await paint(page);assert.ok(Number(await h.getAttribute('aria-valuenow'))>old);}
 const handle=splitters.first(),box=await handle.boundingBox(),old=Number(await handle.getAttribute('aria-valuenow'));await page.mouse.move(box.x+4,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x+80,box.y+box.height/2,{steps:5});await page.mouse.up();assert.ok(Number(await handle.getAttribute('aria-valuenow'))>old);
 const saved=await handle.getAttribute('aria-valuenow');await page.reload();await page.waitForFunction(()=>document.querySelector('#review-workspace').dataset.token);await paint(page);assert.equal(await page.getByRole('separator').first().getAttribute('aria-valuenow'),saved);
 await page.getByLabel('Comparison mode',{exact:true}).selectOption('commits');await page.getByLabel('Target revision',{exact:true}).fill('another-missing');await submit();await page.waitForFunction(()=>document.querySelector('#comparison-error').textContent.includes('Target revision'));assert.equal(await page.locator('#review-workspace').getAttribute('data-token'),pair);
 await page.getByLabel('Comparison mode',{exact:true}).selectOption('working');await page.getByLabel('Base revision',{exact:true}).fill(target);await submit();await page.waitForFunction(pair=>document.querySelector('#review-workspace').dataset.token!==pair,pair);assert.match(await page.locator('#working-source').textContent(),/dirty_working/);assert.equal(await page.locator('#refresh-working').isVisible(),true);
 await page.goto(host.url);await page.waitForFunction(()=>document.querySelector('#snapshot').textContent.length===8);await paint(page);
 assert.doesNotMatch(await page.locator('#counts').textContent(),/shown|matching/);assert.equal(await page.locator('footer').count(),0);
 const camera=await page.evaluate(()=>window.vizziMetrics.lastFrame.camera);await page.locator('#map-overview').click({position:{x:140,y:90}});await paint(page);const jumped=await page.evaluate(()=>window.vizziMetrics.lastFrame.camera);assert.notDeepEqual(jumped,camera);assert.equal(jumped.scale,camera.scale);
 await page.getByRole('button',{name:'Zoom in',exact:true}).click();await paint(page);assert.ok(await page.evaluate(old=>window.vizziMetrics.lastFrame.camera.scale>old,camera.scale));
 await page.getByRole('button',{name:'Fit map',exact:true}).click();await paint(page);
 await page.locator('#map').evaluate(n=>{const r=n.getBoundingClientRect();window.mapRect={x:r.x,y:r.y};});
 const data=await (await fetch(host.url+'/api/view?projection=summary')).json();const c=await page.evaluate(()=>window.vizziMetrics.lastFrame.camera),tile=data.files[0].rect,rect=await page.locator('#map').boundingBox();await page.mouse.move(rect.x+c.x+(tile.x+tile.width/2)*c.scale,rect.y+c.y+(tile.y+tile.height/2)*c.scale);await paint(page);assert.doesNotMatch(await page.locator('#tooltip').textContent(),/syntax syntax/i);
 await page.setViewportSize({width:600,height:900});await paint(page);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
 assert.deepEqual(errors,[]);
 if(process.argv.includes('--screenshots')){const out=process.argv[process.argv.indexOf('--screenshots')+1];await mkdir(out,{recursive:true});await page.setViewportSize({width:1600,height:1000});await page.goto(host.url+'/review');await page.waitForFunction(()=>document.querySelector('#review-workspace').dataset.token);await page.screenshot({path:join(out,'review.png'),fullPage:true});await page.goto(host.url);await page.waitForFunction(()=>document.querySelector('#snapshot').textContent.length===8);await page.screenshot({path:join(out,'map.png'),fullPage:true});}
 console.log('trial feedback browser: revision errors and retention, commit pair, working mode, lexical colours, labelled types, resizing and persistence, overview navigation, concise counts and narrow layout passed');
}finally{await chromium?.close();await host?.close();await input.close();}
