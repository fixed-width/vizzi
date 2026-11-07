// First paired outputs only. Keep relevance oracles and source-aware navigation out of acquisition.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile, writeFile, mkdir, chmod} from 'node:fs/promises';
import {join, resolve} from 'node:path';
import {startHost} from '../browser/support.mjs';

const [preparedPath, outputPath] = process.argv.slice(2);
assert.ok(preparedPath && outputPath, 'Use fresh-questions.mjs PREPARED_JSON NEW_OUTPUT_DIRECTORY');
const prepared = JSON.parse(await readFile(preparedPath, 'utf8'));
const cohortBytes = await readFile(prepared.cohort);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
assert.equal(hash(cohortBytes), prepared.cohortSha256, 'Questions changed after freezing');
const cohort = JSON.parse(cohortBytes);
assert.equal(hash(await readFile('target/release/vizzi')), cohort.implementation.binarySha256);
for (const [path, record] of Object.entries(prepared.files)) {
  assert.equal(hash(await readFile(join(prepared.sourceRoot, path))), record.sha256, `Pinned source changed: ${path}`);
}
const output = resolve(outputPath);
await mkdir(output);
const editor = join(output, 'record-editor');
await writeFile(editor, '#!/bin/sh\nprintf \'%s\\n\' "$@" >> "$0.args"\n');
await chmod(editor, 0o700);
let host;
const observations = [];
try {
  host = await startHost({root: prepared.sourceRoot, cache: join(output, 'analysis'), sourceArguments: ['--editor-code', editor]}, 1);
  const request = async (path, body) => {
    const started = performance.now();
    const response = await fetch(host.url + path, body === undefined ? {} : {method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(body)});
    const text = await response.text(), elapsedMs = performance.now() - started;
    assert.ok(response.ok, text);
    return {data: JSON.parse(text), elapsedMs, responseBytes: Buffer.byteLength(text)};
  };
  const started = performance.now();
  await request('/api/workspace/start', {});
  let status;
  for (let i = 0; ; i++) {
    status = (await request('/api/status')).data;
    if (status.snapshot) break;
    assert.ok(i < 600 && !status.error, 'Source activation failed');
    await new Promise(done => setTimeout(done, 100));
  }
  const activationMs = performance.now() - started;
  for (const question of cohort.cases) {
    for (const selector of question.selectorOrder) {
      const result = await request('/api/search', {snapshot: status.snapshot, question: question.question, selector});
      assert.equal(result.data.question, question.question);
      assert.equal(result.data.snapshot, status.snapshot);
      assert.equal(result.data.selector, selector);
      assert.ok(result.data.results.length <= 8);
      assert.ok(result.data.results.reduce((n, hit) => n + Buffer.byteLength(hit.excerpt), 0) <= 4000);
      const observation = {case: question.id, selector, recordedAt: new Date().toISOString(), ...result};
      observations.push(observation);
      await writeFile(join(output, `${question.id}.${selector}.json`), JSON.stringify(observation, null, 2) + '\n');
    }
  }
  assert.equal(hash(await readFile(prepared.cohort)), prepared.cohortSha256);
  const report = {schema: 1, recordedAt: new Date().toISOString(), cohort: prepared.cohort, cohortSha256: prepared.cohortSha256, binarySha256: cohort.implementation.binarySha256, sourceRevision: prepared.sourceRevision, sourceRoot: prepared.sourceRoot, status, activationMs, observations, model: null, workers: 1, protocol: 'Single untouched query per selector in the frozen counterbalanced order, before source/relevance audit. No inference, query rewriting, tuning or editor launch. Single observations are not latency distributions.'};
  await writeFile(join(output, 'first-results.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify({activationMs, snapshot: status.snapshot, observations: observations.map(o => ({case: o.case, selector: o.selector, results: o.data.results.length, searchedFiles: o.data.searched_files, omittedFiles: o.data.omitted_files, elapsedMs: o.elapsedMs}))}, null, 2));
} finally {
  await host?.close();
}
