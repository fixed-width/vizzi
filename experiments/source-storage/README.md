# Measure captured-source buffers across version history

This how-to isolates source/index storage from parsers, facts, caches and UI work.
The [results](RESULTS.md) distinguish current data, publication history, external handles,
allocation capacities and process RSS. See the [source accounting reference](../../docs/prototype/CAPTURED_SOURCE.md).

```sh
cargo build --release --offline -p vizzi --all-features --example source_storage
python3 experiments/source-storage/measure.py \
  --output results/local/source-storage.json
```

The pinned [qualification profiles](../qualification/profiles.json) must exist locally.
Use `--profile linux` or `--profile pytorch` for one. No analysis cache preparation is
needed because these storage snapshots intentionally contain no analysis facts or
verified engine capture-plan marker. Assignment/classification metadata is preserved;
this is not a source-only language-policy change or an end-to-end indexing benchmark.

Each profile runs two separate processes over marked disposable copies:

- `headers`: edit the same three large files used in earlier reports; PyTorch includes
  two C++ source files as well as a header.
- `workspace`: edit every captured source file, stressing full-version retention.

The sequence captures a baseline, reopens all unchanged files, prepends four unique
version markers, drops an external baseline handle after its publication identity
expires, restores original bytes, then releases publication state. Prefix edits are
byte changes to unanalysed storage; they do not execute or build a project. Source-only
snapshots use the normal model/publication interfaces and retain three previous versions.
Original archives remain untouched. Restoration requires the full initial snapshot
identity/source manifest, with additional target SHA-256 checks.

Per-phase ownership inventories deduplicate `Arc<CapturedSource>` allocation identity.
Equal contents in separate allocations count separately. Current, publication-retained
and publication-plus-external sets are reported separately. No-op capture must reuse
all sources. Expired external handles are rehashed against original source; weak handles
verify that published snapshots and selected source allocations die after final release.

Capture timings exclude owned-copy preparation and file rewriting. They include verified
reads/hashing, line indexing, source binding and snapshot creation; the initial phase
also constructs its source manifest. Inventory collection and integrity/restoration work
contribute to process peaks. Capacities exclude struct/Arc/document/fact/allocator overhead
and need not equal resident pages. Released objects need not make RSS fall immediately.

Run without competing builds, tests, mutation jobs or benchmarks. The existing Linux
`RLIMIT_AS` wrapper applies a 12 GiB child address-space limit, adjustable with
`--address-space-mib`; this is not an RSS/physical-RAM limit or 16 GB laptop qualification.
The larger diagnostic limit accommodates the deliberately retained whole-source versions.
Existing output/configuration/logs are not overwritten. Completed conditions are saved
as checkpoints; a failed later run does not erase their evidence.

Recreate the standalone ownership plot with the existing Matplotlib tooling:

```sh
MPLCONFIGDIR=/tmp/vizzi-source-mpl python3 experiments/source-storage/plot.py \
  experiments/source-storage/results/observations.json \
  results/local/source-ownership.svg
```

The plot separates buffer capacities from point RSS and uses different panel scales.
