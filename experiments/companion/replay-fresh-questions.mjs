// Source-informed replay after first paired outputs; this does not measure independent comprehension.
import assert from 'node:assert/strict';
import {readFile, writeFile, mkdir, chmod} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {join, resolve} from 'node:path';
import {browser, startHost, paint} from '../browser/support.mjs';
import {groupResults} from '../../crates/vizzi/web/question-state.js';

const [preparedPath, firstPath, outputPath] = process.argv.slice(2);
assert.ok(preparedPath && firstPath && outputPath, 'Use replay-fresh-questions.mjs PREPARED_JSON FIRST_RESULTS_JSON NEW_OUTPUT_DIRECTORY');
const prepared = JSON.parse(await readFile(preparedPath, 'utf8'));
const cohortBytes = await readFile(prepared.cohort);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
assert.equal(hash(cohortBytes), prepared.cohortSha256);
const cohort = JSON.parse(cohortBytes), first = JSON.parse(await readFile(firstPath, 'utf8'));
assert.equal(first.cohortSha256, prepared.cohortSha256);
assert.equal(hash(await readFile('target/release/vizzi')), cohort.implementation.binarySha256);
const output = resolve(outputPath);await mkdir(output);
let chromium, host, page;
const observations = [], saved = [];
try {
  chromium = await browser({desktopBus: false});
  for (const [position, question] of cohort.cases.entries()) {
    const directory = join(output, question.id);await mkdir(directory);
    const editor = join(directory, 'record-editor');
    await writeFile(editor, '#!/bin/sh\nprintf \'%s\\n\' "$@" >> "$0.args"\n');await chmod(editor, 0o700);
    host = await startHost({root: prepared.sourceRoot, cache: join(directory, 'analysis'), sourceArguments: ['--editor-code', editor, '--investigation-file', join(directory, 'investigation.json')]}, 1);
    page = await chromium.newPage({viewport: {width: 1440, height: 1000}, colorScheme: 'light'});
    const errors = [];page.on('pageerror', e => errors.push(e.message));
    await page.goto(host.url + '/investigate?trial=search');
    await page.waitForFunction(() => document.querySelector('#source-state').textContent.includes('ready'));
    for (const selector of question.selectorOrder) {
      if (selector === 'original' || await page.locator('#search-comparison').getAttribute('open') !== null) {
        if (await page.locator('#search-comparison').getAttribute('open') === null) await page.locator('#search-comparison > summary').click();
        await page.locator('#search-original').setChecked(selector === 'original');
      }
      await page.getByLabel('Ask about this codebase', {exact: true}).fill(question.question);
      const responsePromise = page.waitForResponse(r => new URL(r.url()).pathname === '/api/search' && r.request().method() === 'POST');
      await page.getByRole('button', {name: 'Search sources', exact: true}).click();
      const response = await responsePromise;assert.equal(response.status(), 200);
      const data = await response.json();
      const expected = first.observations.find(o => o.case === question.id && o.selector === selector).data;
      assert.deepEqual(data, expected, 'Browser replay must preserve the untouched first results');
      await page.waitForFunction(() => document.querySelector('#search-results').getAttribute('aria-busy') === 'false');
      const renderedIds = await page.locator('[data-search-result]').evaluateAll(nodes => nodes.map(n => n.dataset.searchResult));
      assert.deepEqual(renderedIds, groupResults(data.results).flatMap(group => group.items.map(hit => hit.id)));
      await paint(page);await page.screenshot({path: join(directory, `${selector}.png`)});
      const primary = data.results[0], card = hit => page.locator(`[data-search-result="${hit.id}"]`);
      await card(primary).getByRole('button', {name: 'Inspect source', exact: true}).click();
      await page.locator('#preview-source').waitFor({state: 'visible'});
      assert.ok((await page.locator('#preview-path').textContent()).includes(primary.path));
      await page.screenshot({path: join(directory, `${selector}-inspect.png`)});
      const observation = {case: question.id, selector, renderedIds, inspectedFirst: {id: primary.id, path: primary.path, line: primary.line, title: primary.title}, kept: null};
      // These source-informed choices are orientation/usage clues, not completed answers.
      const chosen = position === 0 && selector === 'focused' ? primary : position === 1 && selector === 'original' ? data.results.find(h => h.path === 'minitosis/src/lib.rs') : null;
      if (chosen) {
        await page.locator('#close-preview').click();
        await card(chosen).getByRole('button', {name: 'Inspect source', exact: true}).click();
        await page.locator('#preview-source').waitFor({state: 'visible'});
        await card(chosen).getByRole('button', {name: 'Keep this location', exact: true}).click();
        await page.waitForFunction(() => document.querySelector('#show-saved').textContent.includes('1'));
        await page.getByRole('button', {name: 'Keep question', exact: true}).click();
        await page.waitForFunction(() => document.querySelector('#search-keep-question').disabled);
        await page.locator('#preview-editor').click();
        await page.locator('#investigation-notice').waitFor({state: 'visible'});
        observation.kept = {id: chosen.id, path: chosen.path, line: chosen.line, title: chosen.title};
      }
      await page.locator('#close-preview').click();observations.push(observation);
    }
    const board = (await (await fetch(host.url + '/api/investigation')).json()).board;
    assert.equal(board.question, question.question);assert.equal(board.evidence.length, 1);
    const handoff = await readFile(editor + '.args', 'utf8');assert.ok(handoff.includes('--goto\n'));
    await page.locator('#show-saved').click();await paint(page);await page.screenshot({path: join(directory, 'saved.png')});
    assert.deepEqual(errors, []);saved.push({case: question.id, board, recordingOnlyHandoff: handoff, pageErrors: errors});
    await page.close();page = null;await host.close();host = null;
  }
  assert.equal(hash(await readFile(prepared.cohort)), prepared.cohortSha256);
  await writeFile(join(output, 'replay.json'), JSON.stringify({schema: 1, sourceRevision: prepared.sourceRevision, cohortSha256: prepared.cohortSha256, authorship: 'Source-informed automated replay after first results. Selected usage/orientation clues only; no independent navigation timing, complete answer, or real editor launch.', observations, saved}, null, 2) + '\n');
  console.log('Four unchanged query/selector views, exact grouped results, source inspection, two separate saved investigations and recording-only editor handoffs passed.');
} finally {
  await page?.close();await chromium?.close();await host?.close();
}
