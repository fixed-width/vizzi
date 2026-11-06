# Inspect conditional next-token preferences

For contributors investigating the pin-release misread before attempting a weight edit.
The [contract](../../docs/qa/TOKEN_PROBE.md) fixes four known source contexts, two systems,
two answer prefixes and two passes: 32 one-token observations. These are conditional
readouts, not complete QA responses or calibrated confidence.
The [completed probe](TOKEN_PROBE_RESULTS.md) records weak source shifts, larger layout
effects and persistent No preference without identifying a weight edit.
The [matched precision procedure](BF16_PROBE.md) reuses this runner with explicit cohorts
and an archived preflight reference. Its mutation configuration now covers the
precision-pairing helper as well as the original token-probe helper.

Run the serial QA tests and the focused helper mutation configuration before inference:

```sh
node --test --test-concurrency=1 experiments/qa/*.test.mjs
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  node_modules/node/bin/node node_modules/@stryker-mutator/core/bin/stryker.js \
  run experiments/qa/token-probe-stryker.config.json
```

Keep one mutation worker. Production Rust is unchanged; retain the verified native test
baseline when source/build inputs are identical, and report that reuse explicitly. Fresh
formatting/Clippy and affected Node tests remain separate checks. Runner transport is
integration-checked but outside the helper mutation scope.

Prepare from the checksummed historical captured-input archive, without rebuilding or
executing inspected repositories:

```sh
nice -n 10 node experiments/qa/token-probe.mjs prepare \
  --output /tmp/vizzi-qa-token-probe-prepared
```

After all correctness and mutation work finishes:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-token-probe.memory.json -- \
  nice -n 10 node experiments/qa/token-probe.mjs run \
  --prepared /tmp/vizzi-qa-token-probe-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-weight-edit/models \
  --output /tmp/vizzi-qa-token-probe-run
```

Preflight tokenizes/detokenizes all six choice spellings and requires distinct single
tokens before any probabilities are requested. It renders each instruction/source
condition, appends the fixed prefix, tokenizes chat markers and records explicit input
token arrays. The production user/source message is preserved under the diagnostic system;
only its system instruction changes. Prefix spellings and token IDs differ by design.

Requests go to native `/completion` with one generated token and 512 pre-sampling top
log probabilities, caching disabled. No grammar, response_format or bias is supplied.
`responses.jsonl` preserves each raw response before parsing; failures are recorded rather
than repaired. The parser verifies one position, evaluated-token counts, echoed probability
settings, top-list order/uniqueness and finite values. The server closes on success/error.

`probes.jsonl` retains raw data, exact requests, selected token probabilities, censoring,
observed-choice mass and Yes-minus-No margins. `effects.json` compares unconditional and
guarded source within each readout and records permutation effects. Missing candidates
receive an upper bound at the top-list cutoff, never an invented zero or three-way
renormalisation. A candidate winner is emitted only when all three are observed and untied;
the global top token remains separate.

Interpret only within-prefix source shifts and their bounds. A larger Yes margin in the
unconditional case may coexist with an overall No preference. Low probability for the
specific Unknown token in the production system is not a calibrated abstention measure.
Different source states include their captured metadata and snapshot IDs; this does not
isolate a single semantic token or neural circuit. These observations do not qualify a
weight edit, a product answer path or the reserved real-code suite.
