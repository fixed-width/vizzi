import assert from 'node:assert/strict';
import {writeFile, readFile, mkdir, chmod} from 'node:fs/promises';
import {join} from 'node:path';
import {buildHost, browser, startHost, workspace, paint} from './support.mjs';

if (!process.argv.includes('--no-build')) buildHost();
const input = await workspace({files: {
  'engine/item.rs': 'struct Parcel; impl Parcel { fn new() -> Self { Self } }\nfn open_attempt() { let parcel = Parcel::new(); }\n',
  'tools/run.py': 'class Runner: pass\n',
  'unsupported.rs': Buffer.from([255]),
}});
await mkdir(input.cache);
const editor = join(input.cache, 'record-editor');
await writeFile(editor, '#!/bin/sh\nprintf \'%s\\n\' "$@" >> "$0.args"\n');
await chmod(editor, 0o700);
input.sourceArguments = ['--editor-code', editor];
let host, chromium, releaseOld, releaseBoard;
try {
  host = await startHost(input, 1);
  chromium = await browser({desktopBus: false});
  const page = await chromium.newPage({viewport: {width: 1440, height: 1000}}), errors = [];
  page.on('pageerror', e => errors.push(e.message));
  const boardHeld = new Promise(resolve => { releaseBoard = resolve; });
  await page.route('**/api/investigation', async route => {
    if (route.request().method() !== 'GET') return route.continue();
    const response = await route.fetch();
    await boardHeld;
    try { await route.fulfill({response}); } catch { /* Teardown can abort the held initial view. */ }
  });
  await page.goto(host.url + '/investigate?trial=search');
  await page.waitForFunction(() => document.querySelector('#source-state').textContent.includes('ready'));
  const questionInput = page.getByLabel('Ask about this codebase', {exact: true});
  const searchButton = page.getByRole('button', {name: 'Search sources', exact: true});
  const results = page.locator('[data-search-result]');
  async function search(question) {
    await questionInput.fill(question);
    await searchButton.click();
    await page.waitForFunction(() => document.querySelector('#search-results').getAttribute('aria-busy') === 'false');
  }
  assert.equal(await questionInput.count(), 1);
  assert.equal(await page.locator('input:visible,textarea:visible').count(), 1);
  const question = 'where are new parcels created?';
  await search(question);
  await results.first().waitFor();
  assert.equal(await results.first().getByRole('button', {name: 'Keep this location', exact: true}).isDisabled(), true);
  assert.equal(await page.getByRole('button', {name: 'Keep question', exact: true}).isDisabled(), true);
  await results.first().getByRole('button', {name: 'Inspect source', exact: true}).click();
  await page.locator('#preview-source').waitFor({state: 'visible'});
  assert.equal(await page.locator('#keep-discovered').isDisabled(), true);
  await page.locator('#close-preview').click();
  releaseBoard();
  await page.unroute('**/api/investigation');
  await page.waitForFunction(() => !document.querySelector('#search-keep-question').disabled);
  assert.ok(await results.count() <= 8);
  assert.match(await page.locator('#search-scope').textContent(), /2 of 3 indexed files/);
  await page.screenshot({path: '/tmp/vizzi-question-search-light.png'});
  await results.first().getByRole('button', {name: 'Inspect source', exact: true}).click();
  await page.locator('#preview-source').waitFor({state: 'visible'});
  await results.first().getByRole('button', {name: 'Keep this location', exact: true}).click();
  await page.waitForFunction(() => document.querySelector('#show-saved').textContent.includes('1'));
  await page.getByRole('button', {name: 'Keep question', exact: true}).click();
  await page.waitForFunction(() => document.querySelector('#search-keep-question').disabled);
  const board = (await (await fetch(host.url + '/api/investigation')).json()).board;
  assert.equal(board.question, question);
  assert.equal(board.evidence.length, 1);
  await page.locator('#preview-editor').click();
  await page.waitForFunction(() => !document.querySelector('#investigation-notice').hidden);
  assert.match(await readFile(editor + '.args', 'utf8'), /--goto\n.*item\.rs:/);
  await page.locator('#close-preview').click();
  await page.emulateMedia({colorScheme: 'dark'});
  await paint(page);
  await page.screenshot({path: '/tmp/vizzi-question-search-dark.png'});
  await page.locator('#search-comparison > summary').click();
  await page.getByLabel('Original matching', {exact: true}).check();
  await search(question);
  assert.equal(await page.locator('#search-results').getAttribute('data-selector'), 'original');
  await page.getByLabel('Original matching', {exact: true}).uncheck();

  // A superseded response cannot replace the newer visible query.
  let held;
  const heldReady = new Promise(resolve => { held = resolve; });
  const oldResponse = new Promise(resolve => { releaseOld = resolve; });
  await page.route('**/api/search', async route => {
    if (route.request().method() !== 'POST' || route.request().postDataJSON().question !== question) return route.continue();
    const response = await route.fetch();
    held();
    await oldResponse;
    try { await route.fulfill({response}); } catch { /* The superseded fetch may already be aborted. */ }
  });
  await questionInput.fill(question);
  await searchButton.click();
  await heldReady;
  await search('Runner');
  assert.match(await page.locator('#search-results').textContent(), /Runner/);
  releaseOld();
  await paint(page);
  assert.match(await page.locator('#search-results').textContent(), /Runner/);
  await page.unroute('**/api/search');

  // Source publication makes old actions unavailable, while preserving saved evidence.
  await writeFile(join(input.root, 'tools/run.py'), 'class Runner: pass\n# changed capture\n');
  await page.locator('#search-stale').waitFor({state: 'visible'});
  assert.equal(await results.first().getByRole('button', {name: 'Keep this location', exact: true}).isDisabled(), true);
  await search('Runner');
  assert.equal(await page.locator('#search-stale').isVisible(), false);
  assert.equal(await results.first().getByRole('button', {name: 'Keep this location', exact: true}).isDisabled(), false);
  assert.equal((await (await fetch(host.url + '/api/investigation')).json()).board.question, question);

  await page.route('**/api/search', route => route.fulfill({status: 503, contentType: 'application/json', body: JSON.stringify({message: 'Search temporarily unavailable'})}));
  await search('another topic');
  assert.equal(await results.first().getByRole('button', {name: 'Keep this location', exact: true}).isDisabled(), true);
  assert.match(await page.locator('#investigation-error').textContent(), /temporarily unavailable/);
  await page.getByRole('button', {name: 'Dismiss investigation error'}).click();
  await page.unroute('**/api/search');
  await search('where are new missingwidgets created?');
  await page.locator('#search-empty').waitFor({state: 'visible'});
  await page.setViewportSize({width: 600, height: 900});
  await paint(page);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  assert.deepEqual(errors, []);
  console.log('question search browser: one input, groups, bounded/partial source, inspect, explicit saves, recording editor, comparison, supersession, stale capture, error dismissal, empty results and narrow/dark layout passed');
} finally {
  releaseBoard?.();
  releaseOld?.();
  await chromium?.close();
  await host?.close();
  await input.close();
}
