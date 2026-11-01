# Run native performance experiments

Use these experiments to establish native input-processing costs and check renderer
availability before selecting production implementations. Use a current stable Rust
toolchain; the recorded runs used Rust 1.99.1. The project's supported minimum compiler
version remains undecided. Run commands from the repository root.

Recorded observations are in [the initial report](RESULTS.md) and
[the repository parsing and event report](SECOND_RESULTS.md).

## Build the executables

```sh
cargo build --release --workspace --features gpu --locked
```

The `gpu` feature adds the native `vizzi-gpu` executable. Leave it disabled when
working only on file inventory measurements. Use `--offline` if dependencies are
already cached and network access is unavailable.

Add `syntax,watch` to enable the parsing and native notification experiments, or build
with `--all-features` to include every current experiment.

## Measure a stable source snapshot

Choose an extracted repository snapshot or an otherwise stable source directory:

```sh
target/release/vizzi-perf inventory /path/to/source-snapshot 7
```

The JSON output includes file discovery, metadata reads, a complete content rescan
and comparison, JSON manifest decoding, and updates from a supplied source buffer.
The command stops if repeated scans disagree. Keep the source revision and capture
method with saved results.

All regular files are included, including documentation and files without a supported
language analyser. Symlinks are skipped. Only root `.git`, `target`, and `node_modules`
directories are excluded; a nested source directory such as `drivers/target` is included.

To collect process memory as well as timings on Linux:

```sh
mkdir -p experiments/performance/results/local
/usr/bin/time -v -o experiments/performance/results/local/memory.txt \
  target/release/vizzi-perf inventory /path/to/source-snapshot 7 \
  > experiments/performance/results/local/inventory.json
```

For a local Git checkout, capture the committed revision in a new directory before
measuring it. Record the revision returned by `git rev-parse HEAD`, then use
`git archive --format=tar REVISION` and extract into the new directory. This excludes
uncommitted changes and does not expand submodules. A shallow checkout is sufficient
when it contains the selected commit.

## Measure syntax parsing and incremental edits

```sh
cargo build --release --workspace --features syntax --locked
target/release/vizzi-syntax rust /path/to/lpc-rs-snapshot 3
target/release/vizzi-syntax c /path/to/linux-snapshot 3
target/release/vizzi-syntax python /path/to/pytorch-snapshot 3
target/release/vizzi-syntax cpp /path/to/pytorch-snapshot 3
```

The language argument selects a grammar and its file extensions. C and C++ experiments
include headers in their respective scopes. C++ excludes CUDA. Rust experiments exclude
LPC fixtures. An optional final `MAX_FILES` argument limits the run to a sorted path
prefix; the output records matching and measured file counts.

The report separates source reads, full parsing, and syntax-node inspection. Raw source
is parsed without preprocessing, compilation, or name resolution. Syntax recovery is
counted and reported; a returned tree is not evidence of complete language support.

Incremental probes select files near the median, 95th percentile, and maximum byte
size. They append a comment, prepend a comment, and replace an ASCII letter near the
middle. The replacement can produce invalid syntax; recovery is reported. Every edited
tree is compared with a full parse of the same buffer, including node kinds, fields,
flags, depths, and byte ranges, while the original tree must remain unchanged.

Each edit case requests 20 repetitions per corpus pass. Large-file repetitions are
capped by a 32 MiB source-byte budget, with at least three repetitions; requested and
actual sample counts are recorded. Treat these as exploratory observations, especially
for large files with few repetitions.

Edit preparation, incremental parsing, and full parsing of the edited buffer are timed
separately. Preparation includes copying the buffer and calculating byte-oriented
positions. Validation is outside the timed phases. These are independent edits against
the original tree, with full/incremental execution order alternated; they are not an
editor session, a semantic reindex, or a measurement of persistent index publication.

## Measure native filesystem notifications

```sh
cargo build --release --workspace --features watch --locked
target/release/vizzi-watch 1000 50
target/release/vizzi-watch 100000 50
```

Arguments are fixture file count and samples per operation. The file count must be at
least twice the sample count plus one. The command creates and removes an owned
temporary fixture, leaving reference repositories untouched. Files are grouped in
directories of 100; each operation uses a distinct path.

The report separates watcher registration, same-length writes, and replacement through
a temporary file and rename. A callback records notification arrival; the main thread
checks the resulting content fingerprint and updates the manifest. Timings begin before
the write, so a callback can arrive before the write call completes. A five-second
deadline applies to each operation. Failures abort the run instead of disappearing
from successful latency summaries.

A final edit is deliberately ignored by event processing and recovered through a full
scan. This validates the reconciliation route; it does not induce a real operating-system
queue overflow. Fixture creation, initial capture, parsing, semantic invalidation,
immutable publication, and rendering are outside these latency measurements. Results
apply to the observed filesystem and notification backend.

## Generate a controlled file-count workload

Choose a destination that does not already exist. The generator creates 100 files per
directory, with distinct contents and the requested number of bytes per file:

```sh
target/release/vizzi-perf generate /tmp/vizzi-synthetic-100k 100000 1024
target/release/vizzi-perf inventory /tmp/vizzi-synthetic-100k 7
```

Identify this dataset as synthetic in any report. It tests file-count scaling; it does
not stand in for the kernel's source contents, build graph, or dependency patterns.

## Check rendering availability and calibrate geometry

```sh
target/release/vizzi-gpu 10000 4 30
```

Arguments are node count, edges per node, and measured frames. The executable renders
to a 1920 by 1080 texture after five warm-up frames. It reports the adapter and refuses
to time a CPU/software adapter without an explicit opt-in:

```sh
target/release/vizzi-gpu 10000 4 30 --allow-software
```

Use software runs to exercise the rendering path. Preserve the adapter identity and
`hardware_baseline` field when sharing results.

## Verify the experiment code

```sh
cargo fmt --all --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo mutants --workspace --features syntax,watch \
  --file 'crates/vizzi-perf/src/*.rs' --exclude '**/main.rs' --exclude '**/bin/**' \
  --config experiments/performance/mutants.toml --jobs 2 --timeout 60
```

The full library mutation scope covers manifests, filesystem capture, geometry,
timing, edit preparation, syntax inspection, and event classification. For a focused
run on the parser and event-classification modules, use:

```sh
cargo mutants --workspace --features syntax,watch \
  --file 'crates/vizzi-perf/src/syntax.rs' --file 'crates/vizzi-perf/src/watch.rs' \
  --config experiments/performance/mutants.toml --jobs 2 --timeout 60
```

CLI orchestration, third-party parser/watcher implementations, and GPU/shader code are
outside these mutation scopes. Tests and mutation runs should finish before collecting
timing results.

The experiment-specific mutation configuration excludes one allocation-capacity-only
mutation in edit preparation. It changes the initial allocation hint, while preserving
the edited bytes and tree-edit coordinates; buffer capacity is not an interface promise.
Performance measurements address allocation costs separately.

## Interpret the measurements

The first observed scan has uncontrolled OS cache state. Repeated scans are warm
observations, rather than controlled cold-cache measurements. Metadata timing measures
the cost of reading metadata; it does not prove that contents are unchanged. Inventory
processing is sequential; the experiment does not compare parallel scanning strategies.

Buffer-update timing includes hashing one supplied in-memory buffer and applying its
manifest change. It excludes filesystem event delivery, file reads, parsing, semantic
analysis, dependency invalidation, and visual updates. The working manifest is cloned
once before those samples; immutable snapshot publication is not measured. JSON decoding
uses an in-memory buffer and does not measure a persistent cache being opened from disk.

Rendering is native, offscreen, synchronously submitted and waited for, with unlabelled
rectangles and straight edges. Its wall-clock frame timing includes command encoding
and completion waiting; it is not an isolated GPU timestamp. It excludes browser
presentation, text, picking, multiple views, and background analysis. These calibration
results cannot establish application frame-rate or renderer superiority.

Report build mode, compiler, CPU, RAM, adapter, resolution, sample counts, and concurrent
load with results. The application target remains an integrated-GPU laptop with 16 GB
RAM; a run on another environment does not qualify that target.
