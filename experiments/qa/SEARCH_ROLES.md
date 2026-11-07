# Compare role-specific search evidence

For contributors reproducing the role distinction without changing older source-search
results. Read [the contract](../../docs/qa/SEARCH_ROLES.md) for requirements and limits.
The [cohort](library/search-roles/cohort.json) keeps the preceding fifteen questions and
all frozen literal/scope targets, with no new held-out claim or inference.
The [completed report](SEARCH_ROLES_RESULTS.md) and
[source cards](results/search-roles/trial.md) retain the improvement and remaining gap.

Run workspace tests/doctests, formatting, all-target Clippy and serial QA Node tests.
Use the role, ownership, discovery and native protocol fixtures for focused checks:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo test --offline -p vizzi-qa \
  --test search_roles --test ownership --test discovery --test protocol_options
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo mutants \
  -p vizzi-qa -f crates/vizzi-qa/src/discovery.rs -j 1 --jobserver-tasks 1 \
  --cargo-arg=--offline --output /tmp/vizzi-roles-mutants \
  -- --test discovery --test ownership --test search_roles --test protocol_options
```

Keep one low-priority mutant/compiler job. Retain initial whole-module outcomes and
baseline-checked survivor retests after stronger fixtures. Report equivalents, unviable
variants, timeouts and incomplete runs separately from caught mutations.

Build the native experimental host and observer with one compiler task:

```sh
CARGO_BUILD_JOBS=1 nice -n 10 cargo build --release --offline \
  -p vizzi-qa -p vizzi-perf --bin vizzi-qa --bin vizzi-envelope
```

Freeze a fresh capture of the pinned source, code and unchanged oracles:

```sh
nice -n 10 node experiments/qa/coverage.mjs prepare \
  --cohort experiments/qa/library/search-roles/cohort.json \
  --output /tmp/vizzi-search-roles-prepared
```

`--source-archive /path/to/lpc-rs.tar` can locate the original cached archive elsewhere.
Its checksum must match. Preparation verifies case/target equality to the preceding cohort,
audits markers and captures a new index identity without issuing queries or executing
inspected source. Core/browser analysis is unchanged in this revision.

After every correctness/mutation/build job finishes:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-search-roles.memory.json -- \
  nice -n 10 node experiments/qa/coverage.mjs run \
  --cohort experiments/qa/library/search-roles/cohort.json \
  --prepared /tmp/vizzi-search-roles-prepared/prepared.json \
  --output /tmp/vizzi-search-roles-run
```

Ninety serial observations compare compact and revised search with immediate native source
binding checks. Keep literal-marker and scope-locator ranks separate. Compare earlier archived
search gains explicitly; no targets may be changed to fit observed alternatives. Raw responses,
result pools, timings and assessment are retained. The owned native process closes even on
failure. No model server, viewer/editor or reserved-case query is required.
