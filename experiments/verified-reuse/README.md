# Measure verified unchanged-source reuse

Use the existing pinned Linux/PyTorch archives and the
[streaming capture driver](../stream-capture/README.md). Run from the workspace root
without competing builds, tests, mutation jobs or other benchmarks.

```sh
cargo build --release --offline -p vizzi --features qualification \
  --example stream_capture --example source_storage --bin vizzi
python3 experiments/stream-capture/measure.py \
  --fixture-mib 512 --output /tmp/vizzi-reuse-current/observations.json
```

Choose new output paths. The driver uses owned marked copies, verifies restoration,
and records raw phases, memory and binary hashes. It does not execute analysed project
tooling. The generated fixture captures 512 MiB twice with bounded independent byte/hash
checks. Corpus histories retain old pins, create four whole-scope changed generations,
verify unchanged sharing, restore source and release all tracked handles.

For a before/after comparison, preserve the preceding stage's `stream_capture` and
`source_storage` release executables before rebuilding. Verify their SHA-256 hashes
against the [preceding raw observations](../stream-capture/results/observations.json).
Run the same driver with those executables in a separate directory:

```sh
python3 experiments/stream-capture/measure.py \
  --binary-directory /tmp/vizzi-reuse-baseline-bin \
  --fixture-mib 512 --output /tmp/vizzi-reuse-before/observations.json
```

The recorded comparison runs the preserved old implementation first and the current
implementation second. Each driver also runs whole-vector capture as a fixture reference.
Sequential conditions with different UUID prefixes are not randomised speedups or timing
tails. Source-only histories omit parsers, facts, watcher scheduling and UI work. The
2 GiB fixture / 12 GiB corpus address-space limits constrain virtual memory, not RSS or
physical RAM. Metadata and verification contribute to process peaks. Staging and committed
spool capacity retain their separate existing disk bounds.

To check implementation order on one owned 512 MiB fixture, run the additional two
opposite-order pairs after the corpus histories finish:

```sh
python3 experiments/verified-reuse/fixture_pairs.py \
  --before /tmp/vizzi-reuse-baseline-bin/stream_capture \
  --after target/release/examples/stream_capture \
  --output /tmp/vizzi-reuse-pairs/observations.json
```

Every child verifies bytes, identity, metadata and allocation sharing with bounded reads.
The additional samples check direction in both orders; they do not establish timing tails.

See [results](RESULTS.md) for measurements and limitations, the
[source reference](../../docs/prototype/SOURCE_READS.md) for the interface, and the
[design](../../docs/plans/verified-source-reuse.md) for why EOF and exact length are required.
