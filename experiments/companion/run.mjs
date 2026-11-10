import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdir, readFile, writeFile, chmod} from 'node:fs/promises';
import {join, resolve} from 'node:path';
import {browser, startHost, paint} from '../browser/support.mjs';

const args = process.argv.slice(2);
const option = name => args[args.indexOf(name) + 1];
assert.ok(args.includes('--source') && args.includes('--output'), '--source and --output are required');
const output = resolve(option('--output'));
await mkdir(output);
const editor = join(output, 'record-editor');
await writeFile(editor, '#!/bin/sh\nprintf \'%s\\n\' "$@" >> "$0.args"\n');
await chmod(editor, 0o700);
const input = {
  root: resolve(option('--source')), cache: join(output, 'analysis'),
  sourceArguments: ['--investigation-file', join(output, 'investigation.json'), '--editor-code', editor],
};
const report = {
  schema: 1, source: input.root,
  fixtureRevision: '1a4f5b65dd2a269298748df2dfd68092a169a8d9',
  operator: 'Reference-aware scripted replay; no human editor-only comparison or comprehension measurement.',
  binarySha256: createHash('sha256').update(await readFile('target/release/vizzi')).digest('hex'),
  editor: 'Recording executable only. No VS Code window is launched or inspected.',
  errors: [], anchors: [],
};
const expectedFiles = {
  'src/compiler/mod.rs': '25012f2d87e4268a217fe59f35bca6a82223e60f1b2a978cdc38c27d5731ff48',
  'src/compiler/source_reader.rs': 'c36da30317c1ec9679a770b55a8bf6d813cac8767143534b144d48836c927382',
  'src/compiler/source.rs': '7193804ee144a3167a62f1515c2cda74f43243a8dee4f4d31d1f587ac4799c85',
};
const checkFiles = async () => {
  for (const [path, hash] of Object.entries(expectedFiles)) {
    assert.equal(createHash('sha256').update(await readFile(join(input.root, path))).digest('hex'), hash, `Pinned evidence file ${path}`);
  }
};
await checkFiles();
report.evidenceFileSha256 = expectedFiles;
let host, chromium;
try {
  host = await startHost(input, 1);
  chromium = await browser({desktopBus: false});
  report.browser = chromium.version();
  const page = await chromium.newPage({viewport: {width: 1600, height: 1000}});
  page.on('pageerror', e => report.errors.push(e.message));
  await page.goto(host.url);
  await page.waitForFunction(() => document.querySelector('#save-mode').textContent);
  await page.waitForFunction(() => document.querySelectorAll('.discovery-entry').length > 0);
  assert.equal(await page.locator('input:visible,textarea:visible').count(), 1);
  await page.screenshot({path: join(output, 'overview.png'), fullPage: true});
  const locations = [
    ['compile_file', 'src/compiler/mod.rs', 140, 'Obtains identity and source through read_file before compiling.'],
    ['read_file', 'src/compiler/mod.rs', 151, 'Uses the injected reader or DiskSourceReader; retries only NotFound on a path without the c extension.'],
    ['SourceReader', 'src/compiler/source_reader.rs', 17, 'Supplies raw bytes separately from source authorization and decoding.'],
    ['SourceReader for DiskSourceReader', 'src/compiler/source_reader.rs', 30, 'The default reader delegates the read to tokio::fs::read.'],
    ['CompilerSource', 'src/compiler/source.rs', 12, 'Keeps the input, resolved path and diagnostic name together.'],
    ['file_reads_stop_after_extension_fallback_and_preserve_other_io_errors', 'src/compiler/mod.rs', 2012, 'Specifies the bounded fallback and preservation of other I/O errors.'],
  ];
  for (const [title, path] of locations) {
    const response = page.waitForResponse(r => {
      const url = new URL(r.url());return url.pathname === '/api/discovery' && url.searchParams.get('search') === title && r.status() === 200;
    });
    await page.getByLabel('Find a name or file',{exact:true}).fill(title);
    await response;
    const match = page.locator('.discovery-entry').filter({has: page.getByRole('heading',{name:title,exact:true})}).filter({hasText:path}).first();
    await match.click();
    await page.getByRole('button',{name:'Keep this location',exact:true}).click();
    await page.waitForFunction(() => document.querySelector('#keep-discovered').textContent === 'Kept');
    await page.getByRole('button',{name:'Close source preview',exact:true}).click();
  }
  report.interaction = 'Declaration search and one-click keep; no typed paths, lines or titles.';
  await page.locator('#show-saved').click();
  report.task = 'How does the compiler obtain source, retry the c extension, and report read failure?';
  for (const [title, path, line, note] of locations) {
    const card = page.locator('.evidence-card').filter({has: page.getByRole('heading', {name: title, exact: true})});
    await card.getByRole('button', {name: 'Inspect evidence', exact: true}).click();
    await page.getByLabel('Your interpretation', {exact: true}).fill(note);
    await page.getByRole('button', {name: 'Save note', exact: true}).click();
    await page.waitForFunction(note => [...document.querySelectorAll('.evidence-summary')].some(n => n.textContent === note), note);
    await page.getByRole('button', {name: 'Close evidence', exact: true}).click();
  }
  await page.getByText('Explain a connection',{exact:true}).click();
  for (const [from, to, explanation] of [
    ['1', '2', 'The compile entry delegates source acquisition to this read/retry loop.'],
    ['2', '3', 'The loop chooses an injected implementation of this reader contract.'],
    ['3', '4', 'The default implementation obtains bytes from disk.'],
    ['2', '5', 'Path identity is replaced when retrying with the c extension.'],
    ['2', '6', 'The regression specifies when retry stops and which error is reported.'],
  ]) {
    await page.getByLabel('From', {exact: true}).selectOption(from);
    await page.getByLabel('To', {exact: true}).selectOption(to);
    await page.getByLabel('What connects them?', {exact: true}).fill(explanation);
    await page.getByRole('button', {name: 'Connect evidence', exact: true}).click();
    await page.waitForFunction(() => document.querySelector('#connection-text').value === '');
  }
  await page.locator('.evidence-card').first().getByRole('button', {name: 'Open in VS Code', exact: true}).click();
  await page.waitForFunction(() => !document.querySelector('#investigation-notice').hidden);
  report.recordedEditorArguments = (await readFile(editor + '.args', 'utf8')).trimEnd().split('\n');
  assert.deepEqual(report.recordedEditorArguments, ['--reuse-window', '--goto', join(input.root, 'src/compiler/mod.rs') + ':140:1']);
  const board = JSON.parse(await readFile(join(output, 'investigation.json'), 'utf8'));
  report.anchors = board.evidence.map(e => ({id: e.id, title: e.title, ...e.source}));
  for (let i=0;i<locations.length;i++) {
    assert.equal(report.anchors[i].path,locations[i][1]);
    assert.equal(report.anchors[i].line,locations[i][2]);
  }
  assert.equal(board.evidence.length, 6);
  assert.equal(board.connections.length, 5);
  await paint(page);
  await page.screenshot({path: join(output, 'investigation.png'), fullPage: true});
  await page.reload();
  await page.waitForFunction(() => document.querySelectorAll('.evidence-card').length === 6);
  assert.equal(await page.locator('#evidence-detail').isVisible(), false);
  assert.deepEqual(report.errors, []);
  await checkFiles();
  await writeFile(join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log('Companion replay: six source anchors, five reader assertions, saved notes and recorded editor handoff passed.');
} finally {
  await chromium?.close();
  await host?.close();
}
