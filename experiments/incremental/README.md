# Measure verified incremental capture

Use this procedure to compare dirty-path verification with complete capture. The
[capture contract](../../docs/prototype/INCREMENTAL_CAPTURE.md) defines retention,
fallbacks, and acknowledgement; [results](RESULTS.md) explain the recorded evidence.

Build the optimized application and run the native driver on a pinned source archive:

```sh
cargo build --release -p vizzi
python3 experiments/prototype/measure.py \
  --source /tmp/vizzi-perf-corpus/lpc-rs \
  --revision 8d3b1f54b36eae33f689bae3e42c716647000e4d \
  --output /tmp/vizzi-incremental-native.json
```

The driver edits a disposable copy and checks every observed edit against another
process's fresh complete capture. It also records startup, cache reuse, a reconciled
no-op, captured bytes/files, and unread retained documents. It requires Linux, Python,
`/usr/bin/time`, and the optimized host. Original source is never modified or built.

Repeat the [browser procedure](../browser/README.md#collect-interaction-measurements)
with a new output path. Single-edit samples now include the first changed snapshot's
native capture measurements. Each final browser index is also checked against a fresh
full capture. A headed run that fails its frame-count check does not qualify hardware
performance; keep that result separate from successful software and native runs.

Run correctness checks with formatting, workspace tests, Clippy, and `npm test`.
Native tests include same-size/same-mtime edits, atomic replacement, rename/deletion,
unreadable and symlink paths, new-directory watch registration, cancellation, failed
and superseded jobs, and reconciliation while edits continue every 75 ms.

For complete-module mutation runs:

```sh
cargo mutants -p vizzi --config experiments/incremental/mutants.toml \
  --file 'crates/vizzi/src/{source,engine,invalidation}.rs' \
  --jobs 4 --timeout 15 --build-timeout 180 -- --lib
cargo mutants -p vizzi --config experiments/incremental/mutants.toml \
  --file crates/vizzi/src/host.rs \
  --jobs 4 --timeout 20 --build-timeout 180
```

The host run includes the live CLI integration tests. One narrow exclusion covers
equivalent strict/non-strict comparisons at a directory prefix ending in `/`; validated
document paths cannot equal that prefix. Keep unviable replacements and timeouts
separate from assertion failures. No actual OS event-queue overflow is induced.
