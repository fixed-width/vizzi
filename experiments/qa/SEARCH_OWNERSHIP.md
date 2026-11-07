# Compare captured ownership and retained baseline anchors

For contributors checking the creation-search regression without changing the earlier
coverage study. Read [the contract](../../docs/qa/SEARCH_OWNERSHIP.md) for requirements,
limits and stopping rules. The [cohort](library/search-ownership/cohort.json) retains the
same fifteen questions and all original literal/scope targets.
The [completed report](SEARCH_OWNERSHIP_RESULTS.md) and
[source cards](results/search-ownership/trial.md) retain the unsuccessful qualification.

Run workspace tests/doctests, formatting, all-target Clippy and the serial QA Node tests.
The affected native implementation is experimental; core/browser analysis is unchanged.
Use the ownership fixtures plus existing discovery/protocol tests for focused work:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo test --offline -p vizzi-qa \
  --test ownership --test discovery --test protocol_options
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo mutants \
  -p vizzi-qa -f crates/vizzi-qa/src/discovery.rs -j 1 --jobserver-tasks 1 \
  --cargo-arg=--offline --output /tmp/vizzi-ownership-mutants \
  -- --test discovery --test ownership --test protocol_options
```

Keep one low-priority mutant/compiler job. Preserve full-scope results and any baseline-
checked retests after stronger tests. Equivalent, unviable and timeout outcomes remain
separate from mutations caught by failures. Source identity and hidden range serialization
are tested through public Corpus/native responses, not private field inspection.

Build the experimental native host and process-tree observer:

```sh
CARGO_BUILD_JOBS=1 nice -n 10 cargo build --release --offline \
  -p vizzi-qa -p vizzi-perf --bin vizzi-qa --bin vizzi-envelope
```

Prepare a fresh pinned archive capture and freeze the revision's code and oracles:

```sh
nice -n 10 node experiments/qa/coverage.mjs prepare \
  --cohort experiments/qa/library/search-ownership/cohort.json \
  --output /tmp/vizzi-search-ownership-prepared
```

`--source-archive /path/to/lpc-rs.tar` locates the cached archive elsewhere; its full-file
checksum must match the original source record. The runner checks that questions/targets
are identical to the previous cohort, verifies their source markers and captures the new
index identity without issuing search queries. Inspected repository code is not executed.

After every correctness/mutation/build job finishes:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-search-ownership.memory.json -- \
  nice -n 10 node experiments/qa/coverage.mjs run \
  --cohort experiments/qa/library/search-ownership/cohort.json \
  --prepared /tmp/vizzi-search-ownership-prepared/prepared.json \
  --output /tmp/vizzi-search-ownership-run
```

This makes ninety serial paired observations, with exact source-bound result checks and
three alternating-order repetitions. Both revised selectors share one current index.
Use previous archived search pools as historical controls; do not silently overwrite them.
No model loads, reserved cases, editor or viewer is involved. The owned native process closes.

Inspect literal marker and scope-locator ranks separately. Also inspect false positives,
alternative creation sites and whether useful baseline source identities survived.
Reported query times include retained-baseline retrieval cost. A found scope or identifier
does not establish a resolved call graph or a complete answer to the original question.
