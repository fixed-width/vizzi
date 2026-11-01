# Run the initial performance experiments

Use these experiments to establish native input-processing costs and check renderer
availability before selecting production implementations. Use a current stable Rust
toolchain; the recorded runs used Rust 1.99.1. The project's supported minimum compiler
version remains undecided. Run commands from the repository root.

## Build the executables

```sh
cargo build --release --workspace --features gpu --locked
```

The `gpu` feature adds the native `vizzi-gpu` executable. Leave it disabled when
working only on file inventory measurements. Use `--offline` if dependencies are
already cached and network access is unavailable.

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
cargo mutants --workspace --file 'crates/vizzi-perf/src/*.rs' \
  --exclude '**/main.rs' --exclude '**/bin/**' --jobs 2 --timeout 45
```

The mutation scope covers content manifests, filesystem capture, geometry generation,
and timing summaries. CLI orchestration and GPU/shader code are outside that mutation
scope. Tests and mutation runs should finish before collecting timing results.

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
