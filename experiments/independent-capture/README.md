# Compare capture during cleanup with independent backing files

Use the pinned archives and compatible cache from the
[foreground measurement procedure](../foreground-maintenance/README.md).
The comparison uses the preceding `93d5632` backend with the identical updated
`crates/vizzi/src/foreground_maintenance.rs` harness. Copy that harness into a separate
archive/checkout and build its qualified example using a separate Cargo target directory.
The updated harness permits readiness before cleanup ends; post-cleanup byte/fact/pin
verification remains mandatory. Verify harness hashes match and executable hashes differ.

Build current probes, then run without competing builds, tests, cache preparation,
mutation jobs or benchmarks, using new output paths:

```sh
cargo build --release --offline -p vizzi --features qualification \
  --example foreground_maintenance --example source_storage --bin vizzi
python3 experiments/independent-capture/measure.py \
  --before /tmp/vizzi-independent-before-target/release/examples/foreground_maintenance \
  --cache /tmp/vizzi-foreground-facts \
  --output /tmp/vizzi-independent-measurements/observations.json
python3 experiments/reclamation/measure.py \
  --output /tmp/vizzi-independent-history/observations.json
```

The first command repeats three alternating condition pairs for Linux and PyTorch on
each implementation. Generated tails are 4 GiB / 768 MiB; backing allowances are 8 / 1 GiB
and the child address-space limit is 12 GiB. The old backend runs first. Source copies
are marked/owned, original targets are restored, and analysed project tooling is never run.
Actual write/capture overlap is recorded rather than assumed. Readiness/publication may
precede cleanup completion; bytes, fresh facts and old pins are verified after it joins.

The second command repeats full source-history churn with old pins and final release,
checking cumulative writes exceed reusable capacity and both files return to zero length.
Backing capacity is aggregate across both files, including cleaning space until IO succeeds;
staging retains a separate equal allowance. With no headroom capture can refuse capacity.

Use raw intervals and per-sample gate/capture/analysis timings with the summary and figure.
These finite warm native observations do not establish browser tails, physical-RAM limits,
hardware defaults, cold-storage behaviour or a representative distribution of retired tails.
See [results](RESULTS.md), [the design](../../docs/plans/independent-capture.md) and
[the source reference](../../docs/prototype/SOURCE_READS.md) for semantics and limits.
