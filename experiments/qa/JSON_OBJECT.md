# Measure generic JSON-object enforcement

For contributors comparing the full response schema with JSON syntax enforcement alone.
The [contract](../../docs/qa/JSON_OBJECT.md) and [cohort](library/json-object/cohort.json)
fix the unchanged model, twelve development cases and two-pass order. Eight real-source
reservations remain unrun. The existing runner and scoring module are reused.
The [completed results](JSON_OBJECT_RESULTS.md) show valid objects with worse content
and no host-conforming responses; full-schema enforcement remains the default.

Run workspace formatting/tests/Clippy and the serial QA tests before inference:

```sh
node --test --test-concurrency=1 experiments/qa/*.test.mjs
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  node_modules/node/bin/node node_modules/@stryker-mutator/core/bin/stryker.js \
  run experiments/qa/decoding-stryker.config.json
```

The mutation configuration covers the entire request builder and paired-study module
with one worker. Review survivors and record equivalents separately from caught mutations.
HTTP tests verify the real generic request body, not just a constructed plan.

Prepare a fresh freeze from local pinned source:

```sh
nice -n 10 node experiments/qa/evidence.mjs prepare \
  --pytorch /path/to/pytorch --decoding experiments/qa/library/json-object/cohort.json \
  --output /tmp/vizzi-qa-json-object-prepared
```

After all correctness/mutation/build jobs finish:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-json-object.memory.json -- \
  nice -n 10 node experiments/qa/evidence.mjs run \
  --decoding experiments/qa/library/json-object/cohort.json \
  --prepared /tmp/vizzi-qa-json-object-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-weight-edit/models \
  --output /tmp/vizzi-qa-json-object-run
```

Preflight requires byte-identical rendered prompts and request equality except omission
of `response_format.schema`. It stops before generation on a mismatch and preserves the
24 request/rendered-prompt records. Generation then makes 48 serial requests, with the
first arm alternating by case and reversing in pass two. Default schema behaviour stays
unchanged for older callers.

Generic mode sends only `response_format: {type: 'json_object'}`. Retain every returned
object without renaming keys, inserting statuses/citations, converting booleans to text,
or stripping extra fields. Report parsed objects separately from conformance to the
existing host schema and native reference binding. Review raw code facts even when the
object shape is unusable. Missing-definition cases also need `rawAbstention` judgments.

Use the existing `compareDecoding` interface with this cohort. Candidate pair fields are
`json_objectTask` and `json_objectContent`; the control fields remain `schemaTask` and
`schemaContent`. Reuse grades only after raw/context/message/native identity checks and
review every changed output. Compare live schema controls with the previous run.

Archive the raw outputs, request/rendered pairs, preparation, grades, metrics and validation
evidence. Keep one whole-cohort memory peak, without inventing per-arm peaks. Close the
server/indexes and retain zero reserved generations. No custom weight edit is included.
