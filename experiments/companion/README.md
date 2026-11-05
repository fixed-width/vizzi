# Validate the investigation workflow

This procedure is for contributors checking the bounded companion slice. Use the
[human comparison protocol](../../docs/companion/TRIAL.md) to assess usefulness.
The scripted replay knows the answer and does not measure comprehension.

Build an optimized host with one compiler job:

```sh
CARGO_BUILD_JOBS=1 nice -n 10 cargo build --release --offline -p vizzi
```

Run the focused browser correctness flow:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 npm test -- --companion --no-build
```

It uses a recording executable for editor handoff. It checks the primary question,
cards/notes/connections, source chooser, persistence, changed-source refusal, errors
inside the source dialog and narrow layout. No real editor is launched.

Prepare the pinned lpc-rs learning source as described in the
[trial kit](../usability/README.md), then run the reference-aware replay with a new
output directory:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 npm test -- \
  --companion-replay --source /path/to/trial/learning/workspace \
  --output /tmp/vizzi-companion-replay
```

The replay verifies three evidence-file SHA-256 values against the frozen
`1a4f5b65dd2a269298748df2dfd68092a169a8d9` fixture, captures six locations, saves
interpretation and five reader connections, and records one editor argument vector.
The output contains an investigation export, screenshot and report. This is
facilitator material: it contains the task's expected locations and interpretation.
It does not prefill the participant's investigation, modify source, or run lpc-rs.

Run the interaction observation separately, after tests and mutation work stop:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 npm test -- \
  --companion-measure --source /path/to/trial/learning/workspace \
  --output /tmp/vizzi-companion-measure
```

It exercises the 32-card/64-link cap with repeated source anchors and synthetic
assertions. Thirty warm detail-open observations include two animation frames after
each action. They are local headless browser observations, not input-latency or GPU
frame-time qualification. No editor is launched. See [results](RESULTS.md).
