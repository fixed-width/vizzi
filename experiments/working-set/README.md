# Measure syntax retention across a mixed working set

This how-to measures whether a retained tree survives edits to other files and concurrent
batches. The [results](RESULTS.md) distinguish cache admission weight, availability,
actual engine timing, and process memory. See the [retention reference](../../docs/prototype/SYNTAX_RETENTION.md).

Prepare the pinned Linux/PyTorch archives and compatible analysis caches using the
[qualification procedure](../qualification/README.md). The following command uses the
same prepared cache directory as the recovery follow-up; initial parsing must be zero.

```sh
cargo build --release --offline -p vizzi --features qualification --example working_set
python3 experiments/working-set/measure.py \
  --cache /tmp/vizzi-window-cache \
  --output results/local/working-set.json
```

Use `--profile linux` or `--profile pytorch` for one repository. The driver creates a
marked disposable copy per repository and runs disabled, 512 MiB and 1 GiB conditions
in separate sequential processes. Existing output/configuration/log files are not
overwritten. No analysed project is executed or built. Original targets are hashed;
restoration and a stable full baseline are checked across conditions.

Each condition keeps one cache for all four targets. Targets are the preceding
[recovery run's](../recovery/RESULTS.md) three large files and picked ordinary file,
including Linux's Python configuration and PyTorch's JNI header. The 20 steps cover:

1. Prime and repeat the largest header; edit the ordinary file; return to the header.
2. Edit the other two large files serially, then return to the largest header.
3. Edit all three large files together using the four-worker engine, then reconcile.
4. Edit the ordinary file ten times, then return to the largest header.

Unique per-condition UUID comments require fresh parsing. Every edit checks parsed/read
file counts, absent persisted hits, independent fresh analysis for each changed file,
and full-snapshot agreement. Reconciliation checks reuse without edits. Per-target
inspection observes availability, retained weight, source bytes and node count before
and after each step. It does not refresh recency or lease state. Aggregate counters
record eviction/bypass activity; batch snapshots cannot identify each worker interleaving.

Run without competing builds, tests, mutation jobs or benchmarks. Engine timings exclude
writing the probe input and verification. Point RSS precedes each verification; sampled
process peaks and high water include independent fresh parses and full captures. The
shared sampler's phase labels do not separate verification, so do not interpret its
per-phase peaks as pure indexing peaks. Quota weights and RSS are distinct measures.

The probe runs the production engine directly. It omits filesystem watchers, periodic
host scheduling, HTTP, rendering, and browser-visible latency. This separates admission
behaviour from the preceding browser observation; it cannot reconstruct that run's exact
cache history or establish laptop memory and latency guarantees.
