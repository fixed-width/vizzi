import assert from 'node:assert/strict';
import {writeFile, readFile, mkdir, chmod} from 'node:fs/promises';
import {join} from 'node:path';
import {buildHost, browser, startHost, workspace, paint} from './support.mjs';
if (!process.argv.includes('--no-build')) buildHost();
const input = await workspace({files: {
  'engine/input.rs': 'pub trait SourceReader {\n    fn read_file(&self);\n}\nmacro_rules! source_readers { ($name:ident) => { fn hidden_reader() {} }; }\n',
  'engine/compile.rs': 'pub fn compile() {}\n',
  'tools/run.py': 'class Runner: pass\n',
}});
await mkdir(input.cache);
const editor = join(input.cache, 'record-editor');
await writeFile(editor, '#!/bin/sh\nprintf \'%s\\n\' "$@" >> "$0.args"\n');
await chmod(editor, 0o700);
input.sourceArguments = ['--editor-code', editor, '--investigation-file', join(input.cache, '..', 'investigation.json')];
let host, chromium;
try {
  host = await startHost(input, 1);
  chromium = await browser({desktopBus: false});
  const page = await chromium.newPage({viewport: {width: 1440, height: 1000}}), errors = [];
  page.on('pageerror', e => errors.push(e.message));
  page.setDefaultTimeout(10000);
  await page.goto(host.url);
  await page.waitForFunction(() => document.querySelectorAll('.discovery-entry').length > 0);
  assert.equal(await page.locator('input:visible,textarea:visible').count(), 1);
  assert.equal(await page.locator('#question').count(), 0);
  assert.equal(await page.locator('#connection-form').isVisible(), false);
  assert.equal(await page.locator('#source-form').isVisible(), false);
  assert.equal(await page.locator('#evidence-detail').isVisible(), false);
  assert.match(await page.locator('#discovery-summary').textContent(), /3.*files/);
  await page.locator('.discovery-entry').first().click();
  await page.waitForFunction(() => document.querySelector('#discovery-breadcrumbs').textContent.includes('engine'));
  await page.locator('.discovery-entry').filter({hasText:'input.rs'}).click();
  await page.locator('#source-preview').waitFor({state:'visible'});
  await page.waitForFunction(() => document.querySelector('#preview-source').textContent.includes('SourceReader'));
  assert.match(await page.locator('#preview-source').textContent(), /SourceReader/);
  assert.equal(await page.locator('#preview-names').getByRole('button', {name:'macro source_readers',exact:true}).count(),1);
  assert.equal(await page.locator('#preview-names').getByRole('button', {name:'function hidden_reader',exact:true}).count(),0);
  await page.locator('#preview-names').getByRole('button', {name: 'trait SourceReader', exact: true}).click();
  await page.getByRole('button', {name:'Keep this location', exact:true}).click();
  await page.waitForFunction(() => document.querySelector('#show-saved').textContent.includes('1'));
  assert.equal((await (await fetch(host.url+'/api/investigation')).json()).board.evidence[0].title, 'SourceReader');
  await page.getByRole('button', {name:'Open in VS Code', exact:true}).filter({visible:true}).click();
  await page.waitForFunction(() => !document.querySelector('#investigation-notice').hidden);
  assert.match(await readFile(editor+'.args','utf8'), /--goto\n.*input\.rs:1:1\n/);
  await page.getByRole('button', {name:'Close source preview', exact:true}).click();
  await page.getByRole('button', {name:'Repository', exact:true}).click();
  await page.getByLabel('Find a name or file', {exact:true}).fill('read_file');
  await page.waitForFunction(() => document.querySelectorAll('.discovery-entry').length === 1 && document.querySelector('.discovery-entry').textContent.includes('read_file'));
  await page.locator('.discovery-entry').first().click();
  await page.getByRole('button', {name:'Keep this location', exact:true}).click();
  await page.waitForFunction(() => document.querySelector('#show-saved').textContent.includes('2'));
  await page.locator('#show-saved').click();
  assert.equal(await page.locator('.evidence-card').count(), 2);
  assert.equal(await page.locator('#connection-form').isVisible(), false);
  await page.locator('#connection-tools > summary').click();
  assert.equal(await page.locator('#connection-form').isVisible(), true);
  await page.reload();
  await page.waitForFunction(() => document.querySelectorAll('.evidence-card').length === 2);
  await page.locator('#show-discovery').click();
  await page.setViewportSize({width: 600, height:900});await paint(page);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  assert.deepEqual(errors, []);
  if (process.argv.includes('--screenshots')) {
    const dir = process.argv[process.argv.indexOf('--screenshots')+1];await mkdir(dir, {recursive:true});
    await page.getByRole('button', {name:'Repository', exact:true}).click();
    await page.waitForFunction(() => document.querySelectorAll('.discovery-entry').length === 2);
    await page.setViewportSize({width:1440,height:1000});await page.screenshot({path:join(dir,'discovery.png'),fullPage:true});
  }
  console.log('discovery browser: automatic orientation, one input, clickable areas/names, one-click keep, recording-only editor handoff and progressive controls passed');
} finally {await chromium?.close();await host?.close();await input.close();}
