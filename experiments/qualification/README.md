# Qualify full-corpus indexing and retained snapshots

Use these experiments for full Linux/PyTorch resource measurements. Raw language
assignment remains separate from compiler configuration. Run one benchmark at a time;
do not run builds, tests, or mutation jobs concurrently with measurements.
The [recorded results](RESULTS.md) distinguish successful collection from the performance
failures it exposed.

## Profiles

[profiles.json](profiles.json) records pinned archive paths/revisions and exact source
arguments. Linux uses an explicit raw C root assignment, with eight C++ source-file
overrides. Three known shell `.inc` fixtures are excluded. PyTorch uses an explicit raw
C++ root assignment, with 194 C source-file overrides. CUDA remains source-only under
the production policy. Rust/Python retain their default selection.

These header assignments are declared experimental inputs. They do not establish a
kernel architecture/configuration, PyTorch build, target ABI, include environment, or
which conditional declarations are active. Archives omit unexpanded submodules.

Update local archive paths when reproducing. The tools read originals for load and
retained-context probes. Live/browser edit probes make disposable copies. No source
repository is built, no imported compiler command is executed, and no original source
is edited.

## Native load and memory

Build both the normal host and optional controlled experiment:

```sh
cargo build --release -p vizzi --offline
cargo build --release -p vizzi --features qualification --example qualify --offline
```

The `qualification` feature exposes only experiment code; ordinary host builds do not
include it. The example adapts JSON configuration and JSON-lines results to that code.
It invokes the same public `Directory`, `Engine`, `Application`, snapshot, and view
interfaces as the host.

Run three cold loads into separate empty caches:

```sh
python3 experiments/qualification/native.py \
  --profiles experiments/qualification/profiles.json --profile linux \
  --action cold --workers 4 --samples 3 \
  --cache /tmp/vizzi-qual-linux --output /tmp/vizzi-qual-linux-cold.json
```

Repeat for `pytorch`. Cold means empty analysis cache, with warm OS caches. The tool
refuses existing output/cold-cache paths. Seeded cache paths can then be used for
`--action warm`, `reconcile`, `reparse`, and `cancel`:

```sh
python3 experiments/qualification/native.py \
  --profiles experiments/qualification/profiles.json --profile linux \
  --action reparse --workers 4 --samples 3 \
  --cache /tmp/vizzi-qual-linux/cold-4-0 \
  --output /tmp/vizzi-qual-linux-reparse4.json
```

Repeat reparse with eight workers and a new output path. The default query file is
`kernel/locking/mutex.c` or `c10/core/Device.cpp`; `--query-file` overrides it.

The controlled phases mean:

| Phase | Work after initial baseline publication |
| --- | --- |
| `reconcile` | Full rediscovery/read/hash verification, reusing unchanged documents |
| `reparse` | A new analyzer-context identity forces real parsing while the old snapshot remains retained |
| `cancel` | The same reparse, superseded after an observed parse start plus the configured delay |

The context revision changes analysis provenance, not source or grammar semantics.
Phase caches are separate and initially empty. The fixed revision permits snapshot
identity comparison across worker counts; repeated runs cannot accidentally use prior
phase-cache results. `--cancel-after-ms` defaults to 100; zero exercises immediate
cancellation at the first analyzer call. Nonzero timers are observed by the query
loop, so cancellation timing includes that loop's cadence.

During controlled work, the main thread queries and serializes a bounded view of a
selected captured file. `--query-interval-ms` defaults to 50. Results record each query's
duration/size/pending observation, completed refresh measurements, publication outcome,
and old-source pin consistency. These queries omit HTTP and browser work. The controlled
experiment omits watcher scheduling and the host's periodic reconciliation; production
host behavior is measured separately below.

The Python parent samples `/proc/PID/status` approximately every 50 ms. Per-phase RSS
maxima are sampled lower bounds. `VmHWM` is process-lifetime high water and cannot be
attributed exclusively to a phase. Reports also retain one-second samples, event
observations, process wall time, CPU affinity, binary hashes, and host memory metadata.
The sampling and query probes themselves consume resources.

## Live source and large-header edits

The prototype driver accepts `--seed-cache`, `--load-samples`, `--edit-samples`, and a
larger `--timeout`. Repeat profile source arguments from `profiles.json` exactly.
Use `--atomic-edits` and `--allow-reconcile` for full-repository runs.
Use `--file-only` for large headers whose first declaration may have an ambiguous
identity; this measures file/source behavior without inventing declaration continuity.

`--allow-reconcile` preserves observed measurements even when a periodic full pass
dominates a file hint or replaces an earlier update's metadata. It does not disable
clean-capture/source/selection checks. The first observed changed snapshot is recorded
even if another refresh is already pending. Ordinary prototype runs still require
one parsed/read file. JSON-line buffering handles large source responses without
repeatedly copying/scanning the accumulated response.

Test ordinary source and the largest selected headers, using owned copies. Every edit
must match a separate complete capture with the same arguments. Stdio response timing
includes the diagnostic envelope and Python decoding; it is not browser presentation.
The complete scope exceeds the 2,000-tile view bound, so view-derived inventory remains
explicitly incomplete and must not be treated as full file coverage.

## Browser interaction

Create a single-profile JSON object from `profiles.json` and add `cache` pointing to
its seeded cache. Then run:

```sh
npm test -- --measure --reference-only --profile /tmp/vizzi-browser-profile.json \
  --samples 3 --duration 4000 --output /tmp/vizzi-full-browser.json
```

`--reference-only` skips the synthetic workload. Reference files are copied before
editing. Full-profile inventory recognizes the declared raw candidate suffixes and
explicit exclusions; its count must agree with the native captured count. The largest
three assigned files drive background work; Python uses `#` probes, other supported
languages use `//`. CUDA source-only files are not used as parse workloads.

The browser collects steady/refresh pan/zoom frames, draw CPU times, picking, ordinary
edit visibility, large-file opening/editing, long frames, renderer heap, native RSS,
request counts, and clean-capture agreement. Header opening is timed after filter
results are ready. The large-source operation uses actual UI selection and records
rendered line/outline counts. The overview remains bounded to 2,000 tiles.

Hardware, OS cache state, scheduling probes, and renderer choice limit interpretation.
Headless software Chromium does not qualify GPU or physical presentation performance.
A 62.5 GiB host does not emulate the 16 GB baseline laptop, even when a measured
process peak fits a proposed memory budget.

## Export observations

After collecting all conditions, validate identities and export a compact JSON/CSV:

```sh
python3 experiments/qualification/summarize.py /tmp/vizzi-native-results /tmp/vizzi-native-summary.json
MPLCONFIGDIR=/tmp/vizzi-mpl python3 experiments/qualification/plot.py \
  /tmp/vizzi-native-summary.json /tmp/vizzi-native-resources.svg
```

Plot generation uses Matplotlib; it is not a product dependency. The checked-in raw
native observations store each probe report once, with query samples exported to CSV.
Source manifests and explicit source-only counts remain distinct from compiler coverage.

## Validate the measurement tool

```sh
cargo test --workspace --all-features --offline
cargo clippy --workspace --all-targets --all-features --offline -- -D warnings
cargo mutants -p vizzi --features qualification \
  --file crates/vizzi/src/qualification.rs --jobs 4 --timeout 15 \
  --build-timeout 180 -- --lib
npm test
```

Fixture tests verify retention/reparse/cancellation, units and timer boundaries,
explicit file selection, rejected inconsistent views, public JSON entry, and invalid
configuration. Report unviable/timeouts/exclusions separately from caught mutations.
