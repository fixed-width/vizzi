# Measure pinned reads during maintenance

Use the pinned Linux/PyTorch qualification archives and run from the workspace root.
Keep builds, correctness tests, mutation jobs and other benchmarks separate from timing.

The comparison needs the preceding spool implementation with the **same current sampling
harness**. Prepare a separate checkout/archive of `0d0e20e`, copy the current
`crates/vizzi/src/source_storage_probe.rs` and its `source_storage_probe/maintenance.rs`
into that source tree, then build its qualified `source_storage` example. Use a separate
Cargo target directory for that build so executable paths cannot collide with the current
workspace. Verify the harness files match byte-for-byte and binary hashes differ before
timing. The recorded source hashes identify both implementations.

Build the current probe, then run the comparison with a new output path:

```sh
cargo build --release --offline -p vizzi --features qualification \
  --example source_storage --bin vizzi
python3 experiments/readable-maintenance/measure.py \
  --before /tmp/vizzi-readable-before-target/release/examples/source_storage \
  --output /tmp/vizzi-readable-maintenance/observations.json
```

The driver runs the preceding implementation first and the current implementation second,
with one Linux and PyTorch child each. It reuses the
[reclamation driver](../reclamation/README.md) with `--binary PATH --maintenance-reads`.
Owned marked copies are restored; analysed project tooling is never run. Source manifests,
byte checks, retained pins, allocation/span limits and final release are verified.

The sampler retains a separate 64 KiB prefix pin while corpus snapshots retire. Each sample
uses a fresh verified backing read and checks every byte; it sleeps about 2 ms between
samples and records at most 4,096 per phase. A ready handshake ensures an initial successful
read before maintenance starts. Read/maintenance intervals share a monotonic clock.
The `released` phase frees the large suffix while that prefix remains readable. The extra
`reader_released` phase drops the prefix and truncates the final backing to zero.

Raw observations include read start/end/duration, completed/overlapping reads and completions
before the maintenance midpoint. Report those counts together with latency; a sample near
the interval boundary alone does not establish progress throughout cleanup. Anchor writes
and actual read counters are checked independently against the preceding source workflow.

The 8 GiB Linux / 1 GiB PyTorch backing allowances retain separate equal staging allowances.
The child address-space limit is 12 GiB, not an RSS/physical-RAM limit. Source-only histories
omit parsers, facts, watchers, HTTP and UI. Sequential samples are not randomised trials,
timing tails, hard IO deadlines or baseline-laptop qualification. See [results](RESULTS.md)
and the [design](../../docs/plans/readable-maintenance.md) for interpretation.
