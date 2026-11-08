import assert from 'node:assert/strict';
import {buildHost,browser,startHost,workspace,paint} from './support.mjs';
if(!process.argv.includes('--no-build'))buildHost();
const source='pub fn long() {\n'+Array.from({length:220},(_,i)=>`    // context ${i}\n`).join('')+'    tail();\n}\npub fn tail() {}\n';
const input=await workspace({files:{'long.rs':source}});let host,chromium,release;
try{
 host=await startHost(input,1);chromium=await browser({desktopBus:false});const page=await chromium.newPage();let captured=false;
 await page.route('**/api/window?**',async route=>{const url=new URL(route.request().url());if(url.searchParams.get('line_start')==='193'){const response=await route.fetch();captured=true;await new Promise(resolve=>{release=resolve;});await route.fulfill({response});}else await route.continue();});
 await page.goto(host.url+'/workspace');await page.waitForFunction(()=>document.querySelector('#visual-source-status').textContent.includes('ready'));await page.getByRole('button',{name:'Add fragment',exact:true}).click();await page.getByLabel('Find a function',{exact:true}).fill('long');await page.getByRole('button',{name:'Find fragments',exact:true}).click();await page.locator('[data-add-symbol][data-name="long"]').click();const fragment=page.locator('[data-view]');await fragment.locator('.code-line').first().waitFor();assert.equal(await fragment.locator('.code-line').count(),96);assert.match(await fragment.locator('.source-section').innerText(),/Lines 1–96/);
 await fragment.getByRole('button',{name:'Next section',exact:true}).click();await page.waitForFunction(()=>document.querySelector('.source-section').textContent.includes('Lines 97–192'));assert.equal(await fragment.locator('.code-line').count(),96);
 await fragment.getByRole('button',{name:'Next section',exact:true}).click();for(let i=0;!captured&&i<500;i++)await page.waitForTimeout(10);assert.equal(captured,true,'later section request was captured');await fragment.getByRole('button',{name:'Previous section',exact:true}).click();await page.waitForFunction(()=>document.querySelector('.source-section').textContent.includes('Lines 1–96'));release();await page.waitForTimeout(100);await paint(page);assert.match(await fragment.locator('.source-section').innerText(),/Lines 1–96/);assert.equal(await fragment.locator('.code-line').first().getAttribute('data-line'),'1');
 console.log('visual sections browser: bounded whole-declaration paging and newest section wins delayed replies');
}finally{release?.();await chromium?.close();await host?.close();await input.close();}
