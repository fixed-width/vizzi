import assert from 'node:assert/strict';
import {mkdir, writeFile, readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {join, resolve} from 'node:path';
import {browser, startHost, paint} from '../browser/support.mjs';

const args = process.argv.slice(2), option = name => args[args.indexOf(name) + 1];
assert.ok(args.includes('--source') && args.includes('--output'), '--source and --output are required');
const output = resolve(option('--output'));
await mkdir(output);
let host, chromium;
try {
  host = await startHost({root: resolve(option('--source')), cache: join(output, 'analysis')}, 1);
  const api = async (path, body) => {
    const response = await fetch(host.url + path, body === undefined ? {} : {method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(body)});
    assert.ok(response.ok, await response.clone().text());
    return response.json();
  };
  await api('/api/workspace/start', {});
  for (let i = 0; !(await api('/api/status')).snapshot; i++) {
    assert.ok(i < 600, 'Index activation timed out');
    await new Promise(done => setTimeout(done, 100));
  }
  const target = await api('/api/investigation/lookup?path=src/compiler/mod.rs');
  let revision = 0;
  for (let i = 1; i <= 32; i++) {
    const result = await api('/api/investigation/evidence', {revision, snapshot: target.snapshot, file: target.file, version: target.version, line: 140, title: `Source context ${i}`});
    revision = result.view.board.revision;
  }
  for (let from = 1; from <= 32; from++) {
    for (const distance of [1, 2]) {
      const result = await api('/api/investigation', {action: 'connect', revision, from, to: (from + distance - 1) % 32 + 1, explanation: 'Synthetic reader assertion for the finite interaction workload.'});
      revision = result.board.revision;
    }
  }
  const response = await fetch(host.url + '/api/investigation');
  const responseBytes = (await response.arrayBuffer()).byteLength;
  chromium = await browser({desktopBus: false});
  const page = await chromium.newPage({viewport: {width: 1600, height: 1000}}), errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.goto(host.url+'/investigate#saved');
  await page.waitForFunction(() => document.querySelectorAll('.evidence-card').length === 32 && document.querySelectorAll('#evidence-links path').length === 64);
  const samples = [];
  for (let i = 0; i < 35; i++) {
    const elapsed = await page.evaluate(async i => {
      const card = document.querySelectorAll('.evidence-card')[i * 7 % 32];
      card.scrollIntoView({block: 'center'});
      const before = performance.now();
      card.querySelector('button').click();
      await new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)));
      const elapsed = performance.now() - before;
      document.querySelector('#close-detail').click();
      return elapsed;
    }, i);
    if (i >= 5) samples.push(elapsed);
    await paint(page);
  }
  const sorted = [...samples].sort((a, b) => a - b);
  const report = {
    schema: 1, browser: chromium.version(), viewport: {width: 1600, height: 1000}, workers: 1,
    binarySha256: createHash('sha256').update(await readFile('target/release/vizzi')).digest('hex'),
    evidence: 32, connections: 64, responseBytes, samplesMs: samples,
    medianMs: sorted[Math.floor(sorted.length / 2)], p95Ms: sorted[Math.ceil(sorted.length * .95) - 1], maxMs: sorted.at(-1),
    errors, limitations: [
      'Headless local Chromium, optimized Rust host, no competing test/mutation cohort.',
      'Thirty warm detail-open to two-animation-frame observations after five warmups; includes frame scheduling, not end-to-end input latency.',
      'Repeated source anchor and synthetic links exercise the investigation cap, not semantic graph discovery or whole-repository rendering.',
      'No actual editor launch, GPU qualification, hardware frame-time capture or human task-time measurement.',
    ],
  };
  assert.deepEqual(errors, []);
  await writeFile(join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify({responseBytes, medianMs: report.medianMs, p95Ms: report.p95Ms, maxMs: report.maxMs}));
} finally {
  await chromium?.close();
  await host?.close();
}
