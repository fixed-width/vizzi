# Run the compact-evidence comparison

This how-to is for contributors repeating the fixed-model iteration in
[COMPACT_PRODUCT](../../docs/qa/COMPACT_PRODUCT.md). Complete the build and correctness
checks in [the pilot procedure](README.md) first, plus
`node experiments/qa/compact.test.mjs` and
`node experiments/qa/compact-assess.test.mjs`. Finish compiler, correctness and serial mutation
jobs before collecting performance results. The experiment requires no Vizzi viewer,
browser or editor.

Use fresh output paths. Prepare the pinned repositories and audit every gold source
anchor, including the fresh formulations. Preparation omits fresh candidate retrieval:

```sh
node experiments/qa/compact.mjs prepare --lpc-root /path/to/lpc-rs \
  --output /tmp/vizzi-qa-v2-source
node experiments/qa/compact.mjs development \
  --prepared /tmp/vizzi-qa-v2-source/prepared.json \
  --output /tmp/vizzi-qa-v2-development.json
```

Inspect only the development report while tuning. It compares both selectors at both
8,000- and 2,500-byte allowances on 32 previously observed questions. Preferred literal
anchor coverage is a diagnostic; inspect whether excerpts preserve the behaviour and
source roles. Rebuild and repeat preparation if native code changes.

Freeze the question library, selection code, adapter, cohort configuration and binary
before inspecting fresh retrieval or model outputs:

```sh
node experiments/qa/compact.mjs freeze \
  --prepared /tmp/vizzi-qa-v2-source/prepared.json \
  --output /tmp/vizzi-qa-v2-freeze.json
node experiments/qa/compact.mjs held-out \
  --freeze /tmp/vizzi-qa-v2-freeze.json \
  --output /tmp/vizzi-qa-v2-held-out.json
```

The eight fresh questions are source-aware agent formulations. They test generalisation
to new wording, not independent developer acceptance. After seeing their results, retain
their failures and use new questions for subsequent tuning rounds.

Use the same pinned runtime and 2B weights as the pilot. The runner checks model bytes
and the runtime executable hash, verifies the freeze, and checks captured snapshots
against preparation. It starts and stops its own CPU server:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-v2.memory.json -- \
  nice -n 10 node experiments/qa/compact.mjs run \
  --freeze /tmp/vizzi-qa-v2-freeze.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-models \
  --output /tmp/vizzi-qa-v2-run
```

The [preregistered cohort](library/v2/cohort.json) selects 23 questions and three arms,
for 69 responses. It includes three known production/test-role regressions in addition
to revised, temporal and fresh cases. Arm order rotates by question. Quote and ID compact arms assert
identical captured contexts. Status/answer field order, decoding, CPU threads and output
limits stay fixed. Legacy versus compact also changes selection, excerpt length and
role/scope metadata; attribute that result to the combined condition.

Grade preserved raw responses against [the rubric](RUBRIC.md), recording facts,
unsupported claims, answerability and citation support separately. ID checks resolve
supplied labels to canonical snapshot/path/content/line references; they do not prove
that the source supports the answer. Keep semantic grading separate from source binding,
and report development and fresh denominators separately. Memory and first-output
timing have the same limitations as the pilot procedure.

After grading every response, generate the assessment with the recorded memory file:

```sh
node experiments/qa/compact-assess.mjs \
  --input /tmp/vizzi-qa-v2-run/answers.jsonl \
  --grades /path/to/grades.json --memory /tmp/vizzi-qa-v2.memory.json \
  --output /tmp/vizzi-qa-v2-assessment.json
```

The assessment reports the full cohort, development/fresh formulations, real repository
questions and temporal fixtures separately. Grades preserve per-fact decisions, unsupported
claims, citation support, source-role confusion and reasons. The checked-in results provide
an example grade file for this frozen cohort; use fresh grades for a new set of responses.

## Evaluate the separate maintainer questions

The three [literal maintainer questions](library/maintainer-2026-11-05/questions.json)
arrived after the v2 freeze. Their [source-bound rubrics](library/maintainer-2026-11-05/cases.json)
are kept separately. Run `node experiments/qa/maintainer.test.mjs`, then audit their
anchors and freeze this cohort against the unchanged v2 inputs:

```sh
node experiments/qa/maintainer.mjs prepare \
  --base-freeze /tmp/vizzi-qa-v2-freeze.json \
  --output /tmp/vizzi-qa-maintainer-freeze.json
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-maintainer.memory.json -- \
  nice -n 10 node experiments/qa/maintainer.mjs run \
  --freeze /tmp/vizzi-qa-maintainer-freeze.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-models \
  --output /tmp/vizzi-qa-maintainer-run
```

Finish the original model run and all checks before measuring this cohort. The runner
uses the same three conditions and asserts identical compact contexts. It supplies the
exact question, not the rubric's scope notes or facts. After grading, pass
`--library experiments/qa/library/maintainer-2026-11-05/cases.json` to
`compact-assess.mjs` with the new response, grade and memory paths. Keep these nine
responses separate from the original 69-response denominator.
