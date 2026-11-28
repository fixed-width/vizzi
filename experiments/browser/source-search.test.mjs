import assert from 'node:assert/strict';
import {test} from 'node:test';
import {buildHost, browser, paint, startHost, workspace} from './support.mjs';

if (!process.argv.includes('--no-build')) buildHost();
const files = {
  'engine/input.rs': 'pub mod reader {\n    pub fn read_file() {}\n    pub fn read_file_extra() {}\n}\n' +
    Array.from({length:53}, (_, i) => `fn paged_name_${String(i).padStart(2,'0')}() {}\n`).join(''),
  'tools/ReadFile.rs': '// CamelHumps file match\n',
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
async function withSearch(run, options = {}) {
  const page = await chromium.newPage({viewport:{width:1280,height:960}, ...options});
  try {
    page.setDefaultTimeout(10000);
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
  await matches.getByText('paged_name_23', {exact:true}).waitFor();
  assert.equal(await matches.getByRole('button').count(), 24);
  const searchVisit = await page.evaluate(() => history.state);
  await matches.hover();
  await page.mouse.wheel(0, 2000);
  await matches.getByText('paged_name_24', {exact:true}).waitFor();
  assert.equal(await matches.getByRole('button').count(), 48);
  assert.deepEqual(await page.evaluate(() => history.state), searchVisit);
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

  await test('CamelHumps finds declarations and filenames and follows Match case', () => withSearch(async (page, search, matches) => {
    const file = matches.getByRole('button', {name:/file ReadFile\.rs tools\/ReadFile\.rs/});
    const declaration = matches.getByText('reader::read_file', {exact:true});
    await search.fill('rf');
    await file.waitFor();
    await declaration.waitFor();

    const matchCase = page.getByRole('checkbox', {name:'Match case',exact:true});
    await matchCase.check();
    await file.waitFor({state:'detached'});
    assert.equal(await declaration.isVisible(), true);

    await search.fill('RF');
    await file.waitFor();
    assert.equal(await declaration.count(), 0);
    await file.click();
    await page.locator('#file-title').getByText('tools/ReadFile.rs', {exact:true}).waitFor();
  }));

  await test('search modes update matches, survive Back and report invalid patterns inline', () => withSearch(async (page, search, matches) => {
    const regex = page.getByRole('checkbox', {name:'Regex',exact:true});
    const matchCase = page.getByRole('checkbox', {name:'Match case',exact:true});
    const pattern = String.raw`^READER::READ_FILE$|^tools/run\.py$`;
    const declaration = matches.getByText('reader::read_file', {exact:true});
    const python = matches.getByRole('button', {name:/file run\.py tools\/run\.py/});
    await regex.check();
    await search.fill(pattern);
    await declaration.waitFor();
    await python.waitFor();

    await matchCase.check();
    await declaration.waitFor({state:'detached'});
    assert.equal(await matches.getByRole('button').count(), 1);
    assert.deepEqual(await page.evaluate(() => window.vizziMetrics.scene.files.map(file => file.path)), ['tools/run.py']);
    await python.click();
    await page.locator('#file-title').getByText('tools/run.py', {exact:true}).waitFor();
    await page.getByRole('button', {name:'Clear filters'}).click();
    await matches.waitFor({state:'hidden'});
    assert.equal(await search.inputValue(), '');
    assert.equal(await regex.isChecked(), true);
    assert.equal(await matchCase.isChecked(), true);

    await regex.uncheck();
    await matchCase.uncheck();
    await page.getByRole('button', {name:'Back',exact:true}).click();
    await python.waitFor();
    assert.equal(await search.inputValue(), pattern);
    assert.equal(await regex.isChecked(), true);
    assert.equal(await matchCase.isChecked(), true);

    await search.fill('[');
    await page.getByText(/Invalid regex.*unclosed character class/).waitFor();
    assert.equal(await search.getAttribute('aria-invalid'), 'true');
    assert.equal(await matches.isVisible(), false);
    assert.equal(await page.locator('#error').isVisible(), false);
    assert.equal(await page.locator('#file-title').textContent(), 'engine/input.rs');
    await search.fill('^read_file$');
    await declaration.waitFor();
    assert.equal(await search.getAttribute('aria-invalid'), null);
  }));

  await test('search modes alone leave Clear filters hidden and survive clearing file types', () => withSearch(async page => {
    const regex = page.getByRole('checkbox', {name:'Regex',exact:true});
    const matchCase = page.getByRole('checkbox', {name:'Match case',exact:true});
    const clear = page.getByRole('button', {name:'Clear filters'});
    await regex.check();
    assert.equal(await clear.isVisible(), false);
    await regex.uncheck();
    await matchCase.check();
    assert.equal(await clear.isVisible(), false);
    await regex.check();
    assert.equal(await clear.isVisible(), false);

    const rust = page.locator('#map-legend').getByRole('button', {name:'Rust',exact:true});
    await rust.click();
    await clear.click();
    assert.equal(await rust.getAttribute('aria-pressed'), 'false');
    assert.equal(await clear.isVisible(), false);
    assert.equal(await regex.isChecked(), true);
    assert.equal(await matchCase.isChecked(), true);
  }));

  await test('keyboard scrolling appends ranked results and opening a match preserves the list', () => withSearch(async (page, search, matches) => {
    const offsets = [];
    page.on('request', request => {
      const url = new URL(request.url());
      const offset = Number(url.searchParams.get('results_offset'));
      if (offset) offsets.push({path:url.pathname,offset});
    });
    await search.fill('paged_name');
    await matches.getByText('paged_name_23', {exact:true}).waitFor();
    await matches.press('End');
    await matches.getByText('paged_name_24', {exact:true}).waitFor();
    await matches.press('End');
    const last = matches.getByRole('button', {name:/function paged_name_52 /});
    await last.waitFor();
    assert.deepEqual(await matches.locator('code').allTextContents(),
      Array.from({length:53}, (_, i) => `paged_name_${String(i).padStart(2,'0')}`));
    const expected = [24, 48].map(offset => ({path:'/api/matches',offset}));
    assert.deepEqual(offsets, expected);
    assert.equal(await page.getByRole('navigation', {name:'Source match pages'}).count(), 0);

    await matches.press('End');
    await last.focus();
    const scrollTop = await matches.evaluate(list => list.scrollTop);
    await last.press('Enter');
    await page.locator('#outline [aria-current="true"] .declaration-name').getByText('paged_name_52', {exact:true}).waitFor();
    assert.equal(await last.getAttribute('aria-current'), 'location');
    assert.equal(await last.evaluate(button => button === document.activeElement), true);
    assert.equal(await matches.evaluate(list => list.scrollTop), scrollTop);
    await page.getByRole('button', {name:'Back',exact:true}).click();
    await page.waitForFunction(() => document.querySelector('#navigation').getAttribute('aria-busy') === 'false' &&
      !document.querySelector('#match-list [aria-current="location"]'));
    assert.equal(await matches.getByRole('button').count(), 53);
    assert.deepEqual(offsets, expected);
  }));

  await test('changing a search mode resets scrolling and discards a pending page', {timeout:15000}, () => withSearch(async (page, search, matches) => {
    const pending = Promise.withResolvers(), release = Promise.withResolvers();
    const scrollToBottom = () => matches.evaluate(list => list.scrollTo({top:list.scrollHeight, behavior:'instant'}));
    let requests = 0;
    await page.route('**/api/matches?**', async route => {
      const query = new URL(route.request().url()).searchParams;
      if (query.get('results_offset') !== '48') return route.continue();
      requests++;
      const response = await route.fetch();
      pending.resolve();
      await release.promise;
      await route.fulfill({response});
    });
    try {
      await search.fill('paged_name');
      await matches.getByText('paged_name_23', {exact:true}).waitFor();
      await scrollToBottom();
      await matches.getByText('paged_name_47', {exact:true}).waitFor();
      await scrollToBottom();
      await pending.promise;
      await matches.evaluate(list => list.scrollTo({top:0, behavior:'instant'}));
      await paint(page);
      await scrollToBottom();
      await paint(page);
      assert.equal(requests, 1);
      assert.ok(await matches.evaluate(list => list.scrollTop) > 0);
      await page.getByRole('checkbox', {name:'Match case',exact:true}).check();
      await matches.getByText('paged_name_47', {exact:true}).waitFor({state:'detached'});
      release.resolve();
      await paint(page);
      assert.equal(await matches.getByRole('button').count(), 24);
      assert.equal(await matches.evaluate(list => list.scrollTop), 0);
    } finally {
      release.resolve();
    }
  }));

  await test('tapping Retry restores a failed page even after scrolling back to the top', {timeout:15000}, () => withSearch(async (page, search, matches) => {
    const pending = Promise.withResolvers(), release = Promise.withResolvers();
    let attempts = 0;
    await page.route('**/api/matches?**', async route => {
      if (new URL(route.request().url()).searchParams.get('results_offset') === '24' && ++attempts === 1) {
        pending.resolve();
        await release.promise;
        await route.fulfill({status:503,body:'Source temporarily unavailable'});
      } else {
        await route.continue();
      }
    });
    try {
      await search.fill('paged_name');
      await matches.getByText('paged_name_23', {exact:true}).waitFor();
      await matches.evaluate(list => list.scrollTop = list.scrollHeight);
      await pending.promise;
      await matches.evaluate(list => list.scrollTop = 0);
      release.resolve();
      const retry = page.getByRole('button', {name:'Retry loading matches',exact:true});
      await retry.waitFor();
      assert.equal(await matches.getByRole('button').count(), 24);
      await retry.tap();
      await matches.getByText('paged_name_24', {exact:true}).waitFor();
      await retry.waitFor({state:'hidden'});
      assert.equal(await matches.evaluate(list => list.scrollTop), 0);
      assert.equal(attempts, 2);
    } finally {
      release.resolve();
    }
  }, {hasTouch:true}));

  await test('restoring a draft search resumes pagination at the bottom', {timeout:15000}, () => withSearch(async (page, search, matches) => {
    const pageReady = Promise.withResolvers(), releasePage = Promise.withResolvers();
    const releaseOtherSearch = Promise.withResolvers();
    let attempts = 0;
    await page.route(/\/api\/(?:view|matches)\?/, async route => {
      const query = new URL(route.request().url()).searchParams;
      if (query.get('filter') === 'read_file') {
        await releaseOtherSearch.promise;
      } else if (query.get('results_offset') === '24' && ++attempts === 1) {
        const response = await route.fetch();
        pageReady.resolve();
        await releasePage.promise;
        return route.fulfill({response});
      }
      await route.continue();
    });
    try {
      await search.fill('paged_name');
      await matches.getByText('paged_name_23', {exact:true}).waitFor();
      await matches.evaluate(list => list.scrollTop = list.scrollHeight);
      await pageReady.promise;
      const otherSearch = page.waitForRequest(request => {
        const url = new URL(request.url());
        return url.pathname === '/api/view' && url.searchParams.get('filter') === 'read_file';
      });
      await search.fill('read_file');
      await otherSearch;
      releasePage.resolve();
      await page.waitForFunction(() => document.querySelector('#match-list').getAttribute('aria-busy') === 'false');
      assert.equal(await matches.getByRole('button').count(), 24);

      await search.fill('paged_name');
      await matches.getByText('paged_name_24', {exact:true}).waitFor();
      assert.equal(attempts, 2);
    } finally {
      releasePage.resolve();
      releaseOtherSearch.resolve();
    }
  }));

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
