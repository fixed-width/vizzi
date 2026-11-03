# Measure engine and browser memory together

Use this procedure to compare resident source with opt-in spooling during real browser
interaction. Run from the workspace root with the pinned
[qualification archives](../qualification/README.md), installed
[browser dependencies](../browser/README.md) and compatible prepared facts.
No analysed project tooling runs.

Build the host and separate safe Rust observer before measuring:

```sh
cargo build --release --offline -p vizzi -p vizzi-perf \
  --bin vizzi --bin vizzi-envelope
python3 experiments/foreground-maintenance/measure.py \
  --cache /tmp/vizzi-foreground-facts --prepare-only \
  --output /tmp/vizzi-envelope-preparation.json
```

Reuse an existing compatible cache instead of preparing it again. Finish all builds,
tests, mutation jobs and preparation before running the matrix.

## Run with a verified memory limit

Use an execution context with access to its user systemd manager and delegated memory
controller. The recorded run uses the approved npm benchmark entry point. Choose a new
output directory:

```sh
npm test -- --envelope-matrix \
  --cache /tmp/vizzi-foreground-facts \
  --output /tmp/vizzi-memory-envelope-capped
```

The driver creates a fresh scope for each condition using MemoryMax=12G,
MemorySwapMax=0 and memory accounting. The observer requires actual memory.max to be
12884901888 and memory.swap.max to be zero before launching the workload. It rechecks
controls while sampling, verifies descendant membership and compares OOM counters.
There is no fallback to uncapped success. Read the recorded group path, before/after
memory/peak/events and cap_verified flag with the process data.

The 12 GiB group budget leaves a provisional 4 GiB outside the group in a 16 GiB model;
it does not simulate an editor, OS load, integrated GPU or physical laptop. Group charges
include controller-accounted memory beyond mapped process pages, and existing shared
page-cache ownership can affect attribution. A fresh scope avoids inheriting an old peak.

The observer reads controls only. Scope creation belongs to the external systemd launcher.
A normal terminal can also invoke measure.py directly with the same cache/output options.
Direct systemctl probes in a managed shell can fail while the approved benchmark entry
point succeeds; test the actual runner before concluding that enforcement is unavailable.
See [recorded capability evidence](results/capability.json).

## Collect observations without enforcement

When cgroup access is unavailable, explicitly select observation:

```sh
npm test -- --envelope-matrix --observe-only \
  --cache /tmp/vizzi-foreground-facts \
  --output /tmp/vizzi-memory-envelope-observed
```

This uses the same workload but always records cap_verified=false. Low sampled usage
does not qualify constrained RAM. For one prepared profile, use:

```sh
npm test -- --envelope --observe-only --reference-only --during-recovery \
  --profile /tmp/profile.json --output /tmp/browser.json
```

The envelope route skips compilation and writes a browser report plus a .memory.json
sidecar. It also isolates the browser's desktop session bus, preventing Chromium portal
registration from moving its main process to another scope. Ordinary browser tests keep
their default desktop integration. Output files and matrix directories must be new. Inspect each condition's .log
if a command fails; an incomplete matrix does not become a successful report.

## Inspect the workload and evidence

The matrix runs Linux resident, Linux spooled, PyTorch spooled and PyTorch resident.
Each uses four workers, native observation, two-second recovery, no syntax retention/input
quota, and the existing real browser workload: three steady/refresh motion pairs, picking,
ten ordinary edits after distinct observed recovery passes, and one large-header opening
and edit. It checks bounded delivery, visible source markers and final independent full
capture agreement. Disposable copies protect original archives.

Spooling uses shared 8 GiB Linux / 1 GiB PyTorch allowances per host, with separate equal
staging bounds. These are disk/source allowances. The clean-verification host uses the
same selected source policy and runs alongside the live host. Its memory enters peaks;
those peaks cannot be attributed solely to ordinary editing. No generated cleanup tails
or instrumented truncate overlap are added in this experiment.

The Rust parent samples itself and its descendants at a nominal 250 ms interval, even
while the Node driver synchronously waits for verification. Samples contain process
names, PIDs/start times, RSS/PSS, aggregate sums, missing/error diagnostics and sampling
duration. Use peak aggregate samples rather than summing separate process high-water
marks. RSS repeats shared mappings; PSS apportions mapped shared pages. Ancestry can miss
orphans and short-lived children, sampling is non-atomic, and process memory excludes
unmapped page cache/kernel charges. The outer npm/Python launchers are outside the sampled
ancestry; a capped scope includes its own launcher membership in controller charges.

See the primary [procfs reference](https://docs.kernel.org/filesystems/proc.html),
[cgroup v2 reference](https://docs.kernel.org/admin-guide/cgroup-v2.html) and
[systemd resource controls](https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html)
for accounting semantics. The [results](RESULTS.md) interpret these finite warm-cache
observations; the [plan](../../docs/plans/memory-envelope.md) records scope and validation.

Archive a completed matrix into a new directory:

```sh
python3 -B experiments/memory-envelope/archive.py \
  --input /tmp/vizzi-memory-envelope-observed \
  --output /tmp/vizzi-memory-envelope-archive
```

The archive retains compressed raw browser/memory reports with uncompressed SHA-256
digests, screenshots, simultaneous per-sample CSV rows and a summary. Original matrix
references name the input files; compressed reports use condition.browser.json.gz and
condition.memory.json.gz. Read one with Python's gzip module, for example:

```sh
python3 -c 'import gzip,json; print(json.load(gzip.open("experiments/memory-envelope/results/capped/linux-spooled.memory.json.gz"))["peak_pss_kib"])'
```
