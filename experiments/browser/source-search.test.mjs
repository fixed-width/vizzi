import assert from 'node:assert/strict';
import {test} from 'node:test';
import {buildHost, browser, paint, startHost, workspace} from './support.mjs';

if (!process.argv.includes('--no-build')) buildHost();
const files = {
  'engine/input.rs': 'pub mod reader {\n    pub fn read_file() {}\n    pub fn read_file_extra() {}\n}\n' +
    Array.from({length:27}, (_, i) => `fn paged_name_${String(i).padStart(2,'0')}() {}\n`).join(''),
  'tools/run.py': 'def read_file():\n    pass\n',
};
let deepDirectory = 'z-deep';
for (let depth = 0; depth < 12; depth++) {
  files[`${deepDirectory}/side.rs`] = `fn deep_candidate_${depth}() {}\n`;
  deepDirectory += '/nested';
}
const deepFile = `${deepDirectory}/leaf.rs`;
files[deepFile] = 'fn deep_candidate() {}\n';
const input = await workspace({files});
let host, chromium;
async function withSearch(run) {
  const page = await chromium.newPage({viewport:{width:1280,height:960}});
  try {
    await page.goto(host.url);
    await page.locator('#file-title').getByText('engine/input.rs', {exact:true}).waitFor();
    await run(page, page.getByRole('searchbox', {name:'Find source'}),
      page.getByRole('list', {name:'Source matches'}));
  } finally {
    await page.close();
  }
}
try {
  host = await startHost(input, 1);
  chromium = await browser({desktopBus:false});
  const page = await chromium.newPage({viewport:{width:1280,height:960}});
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.setDefaultTimeout(10000);
  await page.goto(host.url);
  await page.locator('#file-title').getByText('engine/input.rs', {exact:true}).waitFor();
  await page.getByRole('button', {name:'Focus directory engine',exact:true}).click();
  const search = page.getByRole('searchbox', {name:'Find source'});
  const matches = page.getByRole('list', {name:'Source matches'});
  await search.fill('read_file');
  await matches.getByText('reader::read_file', {exact:true}).waitFor();
  assert.equal(await matches.getByRole('button').count(), 3);
  assert.deepEqual(await page.evaluate(() => window.vizziMetrics.scene.files.map(file => file.path).sort()),
    ['engine/input.rs', 'tools/run.py']);

  const scope = page.getByRole('checkbox', {name:/In this directory/});
  await scope.check();
  await matches.getByText('tools/run.py:1').waitFor({state:'detached'});
  assert.equal(await matches.getByRole('button').count(), 2);
  await scope.uncheck();
  const python = matches.getByRole('button', {name:/tools\/run\.py:1/});
  await python.waitFor();
  const rustType = page.locator('#map-legend').getByRole('button', {name:'Rust',exact:true});
  await rustType.click();
  await python.waitFor({state:'detached'});
  assert.deepEqual(await page.evaluate(() => window.vizziMetrics.scene.files.map(file => file.path)), ['engine/input.rs']);
  await rustType.click();
  await python.click();
  await page.locator('#file-title').getByText('tools/run.py', {exact:true}).waitFor();
  assert.equal(await python.getAttribute('aria-current'), 'location');
  assert.match(await page.locator('.code-line.active').first().textContent(), /def read_file/);

  await page.getByRole('button', {name:'Back',exact:true}).click();
  await page.locator('#file-title').getByText('engine/input.rs', {exact:true}).waitFor();
  await search.fill('reader.read_file');
  await python.waitFor({state:'detached'});
  assert.equal(await matches.getByRole('button').count(), 2);
  const exact = matches.getByRole('button', {name:/reader::read_file engine\/input\.rs:2/});
  await exact.click();
  await page.locator('#outline [aria-current="true"] .declaration-name').getByText('read_file', {exact:true}).waitFor();
  await page.locator('#source-matches summary').click();
  assert.equal(await matches.isVisible(), false);
  assert.equal(await page.locator('#map').isVisible(), true);
  await page.locator('#source-matches summary').click();
  await search.fill('engine/input.rs');
  await matches.getByRole('button', {name:/file input\.rs engine\/input\.rs/}).waitFor();
  assert.equal(await matches.getByRole('button').count(), 1);
  await search.fill('paged_name');
  await page.getByRole('button', {name:'Next matches'}).click();
  await matches.getByText('paged_name_24', {exact:true}).waitFor();
  assert.equal(await matches.getByRole('button').count(), 3);
  assert.deepEqual(await page.evaluate(() => window.vizziMetrics.scene.files.map(file => file.path)), ['engine/input.rs']);
  await search.fill('');
  await page.locator('#source-matches').waitFor({state:'hidden'});
  assert.equal(await page.evaluate(() => window.vizziMetrics.scene.focus), 'engine');
  assert.match(await page.locator('.code-line.active').first().textContent(), /pub fn read_file/);
  if (process.argv.includes('--screenshots')) {
    await search.fill('read_file');
    await matches.getByText('reader::read_file', {exact:true}).waitFor();
    await paint(page);
    await page.screenshot({path:process.argv[process.argv.indexOf('--screenshots')+1]});
  }
  await page.setViewportSize({width:600,height:690});
  await search.fill('reader');
  await matches.getByRole('button', {name:/module reader/}).click();
  await page.locator('#outline [aria-current="true"] .declaration-name').getByText('reader', {exact:true}).waitFor();
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  await page.getByRole('button', {name:'Save location',exact:true}).click();
  await page.waitForURL('**/investigate#saved');
  await page.locator('.evidence-card').getByRole('heading', {name:'reader',exact:true}).waitFor();
  assert.deepEqual(errors, []);
  console.log('Source search coordinates scope, map membership, ranked rows, exact source and reading history.');

  await test('opening a match preserves search text typed while the response is in flight', () => withSearch(async (page, search, matches) => {
    const responseReady = Promise.withResolvers(), releaseMatch = Promise.withResolvers();
    await page.route('**/api/view?**', async route => {
      if (new URL(route.request().url()).searchParams.get('reveal_match') !== 'true') {
        await route.continue();
        return;
      }
      const response = await route.fetch();
      responseReady.resolve();
      await releaseMatch.promise;
      await route.fulfill({response});
    });
    try {
      await search.fill('read_file');
      await matches.getByRole('button', {name:/tools\/run\.py:1/}).click();
      await responseReady.promise;
      await search.evaluate(input => {
        const schedule = window.setTimeout;
        const pending = [];
        window.setTimeout = (callback, _delay, ...args) => {
          pending.push(() => callback(...args));
          return 0;
        };
        try {
          input.value = 'paged_name';
          input.dispatchEvent(new Event('input', {bubbles:true}));
        } finally {
          window.setTimeout = schedule;
        }
        window.submitSearch = () => pending.forEach(submit => submit());
      });
      releaseMatch.resolve();
      await page.locator('#file-title').getByText('tools/run.py', {exact:true}).waitFor();
      assert.equal(await search.inputValue(), 'paged_name');
      await page.evaluate(() => window.submitSearch());
      await matches.getByText('paged_name_00', {exact:true}).waitFor();
    } finally {
      releaseMatch.resolve();
    }
  }));

  await test('clearing a search revisited with Back restores the directory it started in', () => withSearch(async (page, search, matches) => {
    await page.getByRole('button', {name:'Focus directory engine',exact:true}).click();
    await search.fill('read_file');
    await matches.getByRole('button', {name:/tools\/run\.py:1/}).click();
    await page.locator('#file-title').getByText('tools/run.py', {exact:true}).waitFor();
    await search.fill('');
    await page.locator('#source-matches').waitFor({state:'hidden'});
    await page.getByRole('button', {name:'Focus directory tools',exact:true}).click();
    await page.getByRole('button', {name:'Back',exact:true}).click();
    await matches.getByText('reader::read_file', {exact:true}).waitFor();
    await search.fill('');
    await page.locator('#source-matches').waitFor({state:'hidden'});
    assert.equal(await page.evaluate(() => window.vizziMetrics.scene.focus), 'engine');
  }));

  await test('selecting a deeply nested match reveals its map tile', () => withSearch(async (page, search, matches) => {
    await search.fill('deep_candidate');
    await matches.getByText('deep_candidate', {exact:true}).click();
    await page.locator('#file-title').getByText(deepFile, {exact:true}).waitFor();
    assert.equal(await page.evaluate(path => window.vizziMetrics.scene.files.some(file => file.path === path), deepFile), true);
    assert.equal(await matches.getByRole('button').count(), 13);
  }));

  await test('reopening the selected match keeps one visit in the reading trail', () => withSearch(async (page, search, matches) => {
    await search.fill('read_file');
    const match = matches.getByRole('button', {name:/tools\/run\.py:1/});
    await match.click();
    await page.locator('#file-title').getByText('tools/run.py', {exact:true}).waitFor();
    const marker = await page.evaluate(() => history.state);
    const response = page.waitForResponse(res => new URL(res.url()).searchParams.get('reveal_match') === 'true');
    await match.click();
    await response;
    await page.waitForFunction(() => document.querySelector('#navigation').getAttribute('aria-busy') === 'false');
    assert.deepEqual(await page.evaluate(() => history.state), marker);
    await page.getByRole('button', {name:'Back',exact:true}).click();
    await page.locator('#file-title').getByText('engine/input.rs', {exact:true}).waitFor();
  }));
} finally {
  await chromium?.close();
  await host?.close();
  await input.close();
}
