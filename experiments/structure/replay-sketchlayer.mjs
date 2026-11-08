// Source-informed cohort replay after untouched first outputs; no independent task-time claim.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir,chmod} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {createHash} from 'node:crypto';
import {browser,startHost,paint} from '../browser/support.mjs';
const [preparedPath,outputPath]=process.argv.slice(2);assert.ok(preparedPath&&outputPath);
const prepared=JSON.parse(await readFile(preparedPath,'utf8')),cohort=JSON.parse(await readFile(prepared.cohort,'utf8'));
assert.equal(createHash('sha256').update(await readFile('target/release/vizzi')).digest('hex'),cohort.implementation.binarySha256);
const output=resolve(outputPath);await mkdir(output);let host,chromium,page;const cases=[];
try{
 chromium=await browser({desktopBus:false});
 for(const [position,c]of cohort.cases.entries()){
  const directory=join(output,c.id);await mkdir(directory);const editor=join(directory,'record-editor');await writeFile(editor,'#!/bin/sh\nprintf \'%s\\n\' "$@" >> "$0.args"\n');await chmod(editor,0o700);
  host=await startHost({root:prepared.sourceRoot,cache:join(directory,'analysis'),sourceArguments:['--editor-code',editor,'--investigation-file',join(directory,'investigation.json')]},1);
  page=await chromium.newPage({viewport:{width:1440,height:1000}});const errors=[],operations=[];page.on('pageerror',e=>errors.push(e.message));page.on('response',async response=>{if(new URL(response.url()).pathname==='/api/structure'&&response.request().method()==='POST')operations.push({request:response.request().postDataJSON(),status:response.status()});});
  await page.goto(host.url+'/investigate?trial=structure');await page.waitForFunction(()=>document.querySelector('#source-state').textContent.includes('ready'));
  const idle=()=>page.waitForFunction(()=>document.querySelector('#structure-results').getAttribute('aria-busy')==='false');
  await page.getByLabel('Find a source area or name',{exact:true}).fill(position===0?'tool':'window');await page.getByRole('button',{name:'Find areas',exact:true}).click();await idle();await page.locator(`button[data-area="${position===0?'src/drawing.rs':'src/canvas.rs'}"]`).click();await idle();
  await paint(page);await page.screenshot({path:join(directory,'inventory.png')});
  if(position===0){
   await page.locator('[data-structure-symbol][data-name="set_tool"]').getByRole('button',{name:'Show candidate uses',exact:true}).click();await idle();
  }else{
   await page.locator('#structure-inventory').click();await idle();await page.locator('[data-structure-symbol][data-name="new"]').getByRole('button',{name:'Follow written calls',exact:true}).click();await idle();
   for(let i=0;!await page.getByRole('button',{name:'Follow setup_drawing',exact:true}).count();i++){assert.ok(i<12,'setup_drawing missing from bounded call pages');await page.getByRole('button',{name:'Next results',exact:true}).click();await idle();}
   await page.getByRole('button',{name:'Follow setup_drawing',exact:true}).click();await idle();
   for(let i=0;!await page.getByRole('button',{name:'Follow draw',exact:true}).count();i++){assert.ok(i<12,'draw missing from bounded call pages');await page.getByRole('button',{name:'Next results',exact:true}).click();await idle();}
  }
  const row=position===0?page.locator('[data-call-site]').first():page.locator('[data-call-site]').filter({has:page.getByRole('button',{name:'Follow draw',exact:true})}).first();
  await paint(page);await page.screenshot({path:join(directory,'call-sites.png')});await row.getByRole('button',{name:'Inspect call',exact:true}).click();await page.locator('#preview-source').waitFor({state:'visible'});await page.locator('#keep-discovered').click();await page.waitForFunction(()=>document.querySelector('#show-saved').textContent.includes('1'));await page.locator('#preview-editor').click();await page.locator('#investigation-notice').waitFor({state:'visible'});await page.locator('#close-preview').click();
  if(position===1){await page.getByRole('button',{name:'Follow draw',exact:true}).click();await idle();await paint(page);await page.screenshot({path:join(directory,'selected-trail.png')});}
  await page.locator('#show-saved').click();await page.getByText('Add or edit a question',{exact:true}).click();await page.locator('#question').fill(c.question);await page.getByRole('button',{name:'Save question',exact:true}).click();await page.waitForFunction(()=>!document.querySelector('#question-form button').disabled);await paint(page);await page.screenshot({path:join(directory,'saved.png')});
  const board=(await(await fetch(host.url+'/api/investigation')).json()).board;assert.equal(board.question,c.question);assert.equal(board.evidence.length,1);assert.match(board.evidence[0].source.excerpt,position===0?/set_tool\(tool\)/:/drawing_state_for_draw\.borrow\(\)\.draw\(cr\)/);
  const handoff=await readFile(editor+'.args','utf8');assert.ok(handoff.includes('--goto\n'));assert.deepEqual(errors,[]);cases.push({case:c.id,operations,board,recordingOnlyHandoff:handoff,pageErrors:errors});await page.close();page=null;await host.close();host=null;
 }
 await writeFile(join(output,'replay.json'),JSON.stringify({schema:1,authorship:'Source-informed automated replay after first observations and source audit. Demonstrates attributed call navigation and retained spelling; no independent effort/comprehension or semantic callback/dispatch proof.',cases},null,2)+'\n');console.log('Sketchlayer: inventory/use lookup, selected constructor/callback/draw trail, retained invocation text, separate saved questions and recording-only handoffs passed.');
}finally{await page?.close();await chromium?.close();await host?.close();}
