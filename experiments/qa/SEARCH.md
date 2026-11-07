# Run question-driven source ranking

For contributors comparing optional model ranking with deterministic search. The
[contract](../../docs/qa/SEARCH.md) fixes the three known questions, candidate selection,
model settings and stopping rule. This experiment returns source cards, with no generated
answer. Read [the explanation](../../docs/explanation/question-driven-search.md) for its role.
The [completed results](SEARCH_RESULTS.md) and
[source-card trial](results/search/trial.md) retain the first comparison and its limitations.

Build the experimental native host and observer with one compiler task:

```sh
CARGO_BUILD_JOBS=1 nice -n 10 cargo build --release --offline \
  -p vizzi-qa -p vizzi-perf --bin vizzi-qa --bin vizzi-envelope
```

Run formatting, workspace tests/doctests, all-target Clippy and serial QA Node tests.
Check mutations before inference:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo mutants \
  -p vizzi-qa -f crates/vizzi-qa/src/ranking.rs -j 1 --jobserver-tasks 1 \
  --cargo-arg=--offline --output /tmp/vizzi-search-mutants \
  -- --test ranking --test ranking_protocol
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  node_modules/node/bin/node node_modules/@stryker-mutator/core/bin/stryker.js \
  run experiments/qa/search-stryker.config.json
```

The mutation scopes are the complete portable ranking module and Node search helper.
Native JSON-line transport has focused integration tests but is outside mutation scope.
Keep one worker and finish all builds, tests and mutation jobs before measurement.

Capture candidates from the same pinned source used by the maintainer-question studies:

```sh
nice -n 10 node experiments/qa/search.mjs prepare --output /tmp/vizzi-search-prepared
```

Use `--source-archive /path/to/lpc-rs.tar` if the original cached archive is elsewhere.
Its full-file checksum must match the pinned record. Preparation extracts a fresh source
tree using Python's data filter and does not execute inspected project code. The index
snapshot includes implementation identity, so preparation freezes its new value while
recording the older study's index ID separately. Runtime must reproduce the new value.

Inspect every literal candidate in `inputs.json`. Before inference, create
`/tmp/vizzi-search-prepared/relevance.json` with `cases` keyed by each captured case ID.
Each entry has `grades` and `reasons` keyed by every native label, plus a
`missing_expected_sources` note. Grade 2 means a direct starting point, 1 useful supporting
context, and 0 an unhelpful match. Judge the visible excerpt, not an unseen file body.
Record source-aware authorship and known-question status. Do not place grades in prompts.

Run after the audit, using the cached unedited Q4 model from the weight-edit study:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-search.memory.json -- \
  nice -n 10 node experiments/qa/search.mjs run \
  --prepared /tmp/vizzi-search-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-weight-edit/models \
  --output /tmp/vizzi-search-run
```

The runner freezes grades/code/inputs before loading a model. A fresh native index must
reproduce the captured source snapshot and all candidate pools. The model receives only
question text and bounded source cards; full identities remain in the native-held context.
It ranks each pool in forward and reversed presentation order with original labels.
No-match pools skip inference. Raw responses are preserved before applying the proposal.

Native `reorder` takes a held context `id` and a complete integer `order`, and returns
original source evidence in that order with `relevance: not_checked`. It rejects missing,
duplicate, unavailable, stale or expired references. It does not modify the original pool.
A rejected model proposal falls back to a freshly validated baseline. If that baseline
cannot be validated, the runner fails rather than publishing it as current.

`rankings.jsonl` retains exact requests/responses, proposed or fallback source cards,
failure reasons and timing. `assessment.json` compares first direct-result rank and
nDCG@3 within each fixed pool; a pool with no relevant source has undefined nDCG. Keep
candidate recall, validity, position sensitivity, relevance and resource costs separate.
Both owned native/model processes close on success or failure. No viewer or editor opens.
