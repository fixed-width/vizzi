# Measure replacement allowances based on observed weight

This how-to repeats the mixed working-set matrix after replacing unconditional maximum
reservations for known state. See the [results](RESULTS.md) and
[retention reference](../../docs/prototype/SYNTAX_RETENTION.md).

Use the same pinned archives, compatible prepared analysis caches, and procedure as
[the original working-set investigation](../working-set/README.md). The driver is unchanged.

```sh
cargo build --release --offline -p vizzi --features qualification --example working_set
python3 experiments/working-set/measure.py \
  --cache /tmp/vizzi-window-cache \
  --output results/local/replacement-allowances.json
```

The 20-step sequence keeps one cache across four targets: isolated header edits, a small
edit, serial edits to other large files, a three-file batch, reconciliation, ten ordinary
edits, and a final primary-header edit. Disabled, 512 MiB and 1 GiB conditions run in
separate sequential processes. Every step checks a complete capture; each changed file
also matches fresh analysis. Unique UUID comments force actual parsing. Only marked owned
copies are edited, with target restoration/hash checks and a stable baseline.

Keep timing separate from builds, tests, mutation jobs and competing benchmarks. Record
per-key availability and aggregate charges, evictions, bypasses, and reuse. Point RSS
precedes verification; process peaks include fresh analysis and complete-capture checks.
No watcher, periodic host scheduling, transport or browser timing is included.

Compare with the committed [fixed-reservation observations](../working-set/results/observations.json).
Those are earlier processes with different UUID comments. The source scope, workload,
worker count, entry/count maxima and core analysis contexts are unchanged. Differences
are historical observations rather than randomized paired speedups or tail estimates.
Do not infer worker interleavings from batch endpoint inspection or interpret retained
weights as parser scratch/input or whole-process limits.
