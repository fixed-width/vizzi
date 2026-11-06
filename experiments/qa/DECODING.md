# Compare schema-constrained and unconstrained decoding

For contributors testing the output constraint while preserving the evidence-use inputs.
The [contract](../../docs/qa/DECODING.md) fixes the comparison and interpretation. The
[cohort](library/decoding/cohort.json) selects twelve development cases; reserved cases
are never admitted. The existing evidence runner is reused with an explicit study file.
The [completed results](DECODING_RESULTS.md) show mixed content changes with no aggregate
gain and no valid unconstrained JSON responses.

Run workspace formatting/tests/Clippy, serial QA tests and the focused mutation check
before inference. No Rust implementation or dependency changes are needed.

```sh
node --test --test-concurrency=1 experiments/qa/*.test.mjs
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  node_modules/node/bin/node node_modules/@stryker-mutator/core/bin/stryker.js \
  run experiments/qa/decoding-stryker.config.json
```

The mutation configuration uses one worker and covers the complete new request builder
and decoding-study module. Server startup, streaming and runner orchestration are outside
that mutation scope; the HTTP-stub and real runtime checks cover their affected interfaces.
Investigate survivors individually rather than interpreting the threshold alone.

Prepare a fresh freeze. This still audits all sixteen development and eight reserved
source contexts, while the decoding plan admits only the selected twelve development cases:

```sh
nice -n 10 node experiments/qa/evidence.mjs prepare \
  --pytorch /path/to/pytorch --decoding experiments/qa/library/decoding/cohort.json \
  --output /tmp/vizzi-qa-decoding-prepared
```

After all correctness/mutation/build jobs finish, measure one serial server:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-decoding.memory.json -- \
  nice -n 10 node experiments/qa/evidence.mjs run \
  --decoding experiments/qa/library/decoding/cohort.json \
  --prepared /tmp/vizzi-qa-decoding-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-weight-edit/models \
  --output /tmp/vizzi-qa-decoding-run
```

Preparation/run study hashes must match. Before generating any answer, the runner posts
each exact request to `/apply-template`, checks equal rendered prompts, and writes
`rendered.json`. Any mismatch stops the run. It then makes 48 requests, alternating the
first arm per case and reversing that order on pass two. The request construction and
HTTP-stub tests verify that only `response_format` is omitted in the unconstrained arm.

The ordinary prompt still asks for JSON without gaining new schema examples or key-order
instructions. Keep prose, fences and invalid JSON verbatim. Record parsing, schema shape,
observed key order and native reference binding independently. Do not strip a fence or
repair keys to obtain a usable answer.

Grade all material facts/unsupported code claims using the existing oracles. For a missing
definition, also record `rawAbstention` as a boolean: the raw answer must explain that the
evidence is unavailable, not merely emit No or a status token. Raw content can succeed
while strict format/native-bound task success fails. No generated text is not a successful
abstention. `compareDecoding` validates the complete paired ledger and equal inputs.

Compare the live schema rows with the earlier evidence-use run before reusing any grades.
Require raw/context/message/native-reference identity and retain all changed judgments.
Archive full request/rendered-prompt pairs, raw outputs, grades, format/content/task
metrics, one overall memory observation and verification evidence. The single process-tree
peak cannot be assigned separately to the two request arms. Close the server and indexes;
keep reserved generations at zero. No model weight edit is part of this comparison.
