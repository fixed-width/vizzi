# Measure reclamation across source history

Use the pinned Linux/PyTorch qualification archives. Run from the workspace root without
competing builds, tests, mutation jobs or benchmarks.

```sh
cargo build --release --offline -p vizzi --features qualification \
  --example source_storage --bin vizzi
python3 experiments/reclamation/measure.py \
  --output /tmp/vizzi-reclamation/observations.json
```

Choose a new output path. The driver creates marked owned copies, records target hashes,
and runs no analysed project tooling. It preserves configurations, progress, stderr and
completed reports beside the final JSON. Input archives remain read-only; target contents
are verified after restoration before owned copies are removed.

The source-only history workflow captures a baseline, verifies unchanged sharing, creates
four whole-scope changed generations while holding an expired baseline query pin, releases
that pin, restores source and releases publications. It verifies actual old bytes, restored
identity, source manifests and weak-handle expiry. Facts, parsing and UI work are omitted.

Backing allowances are 8 GiB Linux and 1 GiB PyTorch, with separate equal staging allowances.
They are below the previous workflow's cumulative committed bytes. Each phase records
allocation/span/gap/file lengths after explicit maintenance; maintenance duration is separate
from the preceding capture/validation duration. The driver checks every recorded span/file
against capacity and final allocation/span/file/staging/queue lengths against zero while
the spool is still open. See the [storage procedure](../source-storage/README.md) for phase
definitions and [source reference](../../docs/prototype/SOURCE_READS.md) for capacity semantics.

The child virtual address-space limit is 12 GiB; it does not constrain RSS/physical RAM
or qualify baseline-laptop hardware. File lengths differ from filesystem block allocation.
Memory peaks include source rewriting, inventories and verification. Single sequential
conditions are observations, not randomised speedups or latency tails. Historical timings
use the previous implementation and are not repeated controls here.

See [results](RESULTS.md) for observations and remaining costs.

`--binary PATH` selects a preserved probe build. `--maintenance-reads` enables a retained
64 KiB prefix and concurrent bounded reads during maintenance, adding a final prefix-release
phase. See the [maintenance-read procedure](../readable-maintenance/README.md) for the
before/after comparison and sampler limits. Defaults preserve the original workload.
