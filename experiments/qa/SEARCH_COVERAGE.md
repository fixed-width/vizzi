# Compare source-search candidate coverage

For contributors reproducing the deterministic candidate-generation comparison. Read the
[contract](../../docs/qa/SEARCH_COVERAGE.md) for frozen targets, limits and interpretation.
This experiment runs no model and returns source locations rather than an answer.
The [completed results](SEARCH_COVERAGE_RESULTS.md) and
[source-card trial](results/search-coverage/trial.md) retain gains, misses and the creation regression.

Run the normal workspace tests/doctests, serial QA Node tests, formatting and all-target
Clippy. Raw macro declarations also require the browser discovery regression. Check the
portable implementation and raw adapter with one low-priority mutation worker:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo mutants \
  -p vizzi-qa -p vizzi-core -f crates/vizzi-qa/src/discovery.rs \
  -f crates/vizzi-core/src/rust.rs -j 1 --jobserver-tasks 1 \
  --cargo-arg=--offline --output /tmp/vizzi-coverage-rust-mutants
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  node_modules/node/bin/node node_modules/@stryker-mutator/core/bin/stryker.js \
  run experiments/qa/coverage-stryker.config.json
```

The new whole-module scopes are separate from supplemental diff checks of shared intent
extraction and native transport. Retain initial runs, strengthened tests and reviewed
exclusions. A timeout or unviable build is not a mutant caught by failing tests.

Build the native host/observer with one compiler task:

```sh
CARGO_BUILD_JOBS=1 nice -n 10 cargo build --release --offline \
  -p vizzi-qa -p vizzi-perf --bin vizzi-qa --bin vizzi-envelope
```

Prepare from the original checksummed lpc-rs archive into a fresh directory:

```sh
nice -n 10 node experiments/qa/coverage.mjs prepare \
  --output /tmp/vizzi-search-coverage-prepared
```

Use `--source-archive /path/to/lpc-rs.tar` if the cached archive is elsewhere. Its hash must
match the pinned source record. Preparation extracts with Python's data filter, checks
all frozen literal source markers and records the new index identity. It runs no search
queries. The [cohort](library/search-coverage/cohort.json) contains only known development
questions. Grades/oracles are separate from the question-only native requests.

After every correctness, mutation and build job finishes:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-search-coverage.memory.json -- \
  nice -n 10 node experiments/qa/coverage.mjs run \
  --prepared /tmp/vizzi-search-coverage-prepared/prepared.json \
  --output /tmp/vizzi-search-coverage-run
```

The run verifies frozen hashes and reproduces the prepared index snapshot. It alternates
compact/search order and question order across three repetitions: 90 observations for
15 questions. Each pool has at most eight results and 4,000 literal bytes. Every returned
pool is immediately checked through native reorder binding. The native host closes even
when a query fails; there is no inference server, inspected build or viewer/editor launch.

`retrieval.jsonl` retains source cards, selectors and native/round-trip timings.
`responses.jsonl` preserves native responses before assessment. `assessment.json` keeps
visible literal-marker rank separate from captured-scope locator rank. Source scopes do
not establish semantic call relationships, and locating a validator does not establish
that every requested check is in a 500-byte preview. Keep source gaps visible.

For direct use of the experimental JSON-line host, pass `selector: search` to `retrieve`.
The `legacy` default and `compact` selector remain available with their existing bounds.
Source-backed proposals still use the same native-held context IDs and reorder validation.
