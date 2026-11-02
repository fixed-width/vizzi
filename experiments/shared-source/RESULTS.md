# Shared-source capture resource results

The later [windowed-delivery report](../windows/RESULTS.md) bounds browser source/list
transfer and records the remaining recovery/overview costs.

Sharing freshly verified source removes most source-storage overlap during full
verification and context reparsing. Linux peak RSS falls from **6.49 GiB to 2.53 GiB**
during retained reparse; PyTorch falls from **1.15 GiB to 0.76 GiB**. All configured
files are still reopened and hashed. Timings do not improve uniformly: the paired
PyTorch cached-load median increases from 459 ms to 547 ms.

## Implementation and method

An immutable [captured source](../../docs/prototype/CAPTURED_SOURCE.md) owns bytes,
encoding availability, content identity, and the line index. Native capture adopts a
fresh buffer or shares matching baseline source after hashing the newly read bytes.
Jobs and documents reference that same storage. Context changes can therefore produce
new facts without duplicating unchanged source and line indices alongside old pins.
Capture also supports cancellation between filesystem operations; partial work returns
no candidate.

The before implementation is commit `9cbf92e`. Both implementations use the same pinned
[profiles](../qualification/profiles.json), explicit raw-language assignments, four
workers, Rust 1.99.1, and warm OS caches on the i7-13700H / 20-logical-CPU / 62.5 GiB host.
Each condition has three samples. Current-context caches are prepared separately;
preparation is excluded. Source model/fingerprint changes invalidate old analyzer
contexts, so before/after digests differ without changing source bytes or identity
composition. Before timing precedes implementation; after timing follows correctness
and mutation checks. No concurrent builds, tests, mutation jobs, or other benchmarks
compete with measurements. System activity and power settings remain uncontrolled.

The controlled native probe loads a baseline, retains it, then performs full verification
or changes the analyzer context to force reanalysis into a new empty phase cache. It
serialises a bounded core view during work and checks the selected old source. No source
repository is built or edited, and no compiler-dependent tooling runs. Captured scope is
66,066 files / 1,495,484,792 bytes for Linux and 10,578 files / 186,092,681 bytes for
PyTorch, including 431 source-only CUDA files. Raw header assignments remain separate
from compiler/configuration coverage.

## Native memory and elapsed time

Memory below is maximum sampled process RSS across three runs, in GiB. Peaks include
baseline and controlled work, not a memory cap. Sampling about every 50 ms can miss
short peaks. The old snapshot stays alive during verification/reparse.

| Condition | Linux before | Linux shared source | PyTorch before | PyTorch shared source |
| --- | --- | --- | --- | --- |
| Fresh-process persisted-cache load | 3.493 | 2.098 | 0.504 | 0.330 |
| Full verification with old snapshot retained | 4.269 | 2.123 | 0.620 | 0.338 |
| Context reparse with old snapshot retained | 6.487 | 2.526 | 1.145 | 0.757 |

The largest total-peak reduction is about 61% for Linux retained reparse. Differences
between peaks from separate processes are not an exact allocation attribution.

Times below are median total engine seconds. Process startup/teardown and browser/HTTP
work are outside these times; captured-source preparation is inside them.

| Condition | Linux before | Linux shared source | PyTorch before | PyTorch shared source |
| --- | --- | --- | --- | --- |
| Fresh-process persisted-cache load | 2.965 | 2.750 | 0.459 | 0.547 |
| Full verification with old snapshot retained | 1.492 | 1.051 | 0.209 | 0.196 |
| Context reparse with old snapshot retained | 48.963 | 48.622 | 8.622 | 8.323 |

PyTorch cached loading is about 19% slower in this comparison. Newly owned source is
decoded/indexed in serial capture rather than worker document construction, a plausible
contributor from code inspection; these measurements do not isolate its cause. Linux
verification is faster, while full reparse times remain similar. Source storage sharing
does not eliminate necessary parsing, fact validation, or snapshot construction.

`capture_ms` is a changed phase boundary: it now includes hashing, UTF-8 validation,
and new source/index construction. Before, some of that work happened later in the
engine/document phases. Total engine time is the meaningful cross-implementation
endpoint; a capture-phase comparison alone cannot establish faster/slower filesystem I/O.

Every full condition still reads the exact file and byte counts above. After retained
verification/reparse, `shared_sources` equals 66,066 / 10,578. Fresh-process loads report
zero because no prior in-memory source exists, despite persisted analysis hits.
Unchanged verification parses nothing. Phase caches start empty for reparsing; duplicate
content/context inputs may hit records produced earlier in the same phase. Actual
parse counts therefore vary slightly by scheduling and are preserved in the reports.
They are not claims that every path needs an independent grammar parse.

Old selected source stays consistent, successful reparses publish generation 2, and
identities agree within each implementation/condition. The largest after-change per-run
core-query p95 is 33.25 ms for Linux reparse and 8.13 ms for PyTorch verification.
These omit HTTP, browser decoding, painting, and presentation. They do not qualify
the 16 GB baseline laptop or establish a general cancellation deadline.

[Comparison JSON](results/comparison.json) and [CSV](results/comparison.csv),
[before summary](results/before-summary.json), [after summary](results/after-summary.json),
and [full observations](results/native-observations.json) retain source arguments,
revisions, samples, query observations, counters, binary hashes, and limits. Each
complete probe report is stored once; final stdout events refer to it.

## Browser recheck

The current PyTorch host runs with the existing 512 MiB syntax-retention option, a
prepared cache, software-rendered headless Chromium 156, and a 1440×900 viewport.
Three four-second steady and three refresh-motion runs have 16.8 ms rAF p99, no rAF
intervals above 50 ms, and draw CPU p99 at most 1.0 ms. This remains the bounded
2,000-tile Canvas2D reference workload, not a production GPU qualification.

Five ordinary edits take **219.9, 119.1, 105.3, 98.7, and 84.6 ms** to matching DOM
plus two rAF callbacks. The first reports one targeted parse without tree reuse;
the subsequent four reuse trees. No full reconciliation is reported in those five
update samples. This does not show the earlier recovery outlier has disappeared,
and the proposed all-five-below-200-ms gate still fails.

The Perfetto header opens in 184.0 ms and updates in 858.2 ms. Its edit reports one
read/parse/tree reuse and 658.5 ms engine time. The opening has no observed long frames;
editing has an 87.3 ms long frame (0.8 ms blocking) and a 66.7 ms maximum rAF interval.
Source stays at 56 rendered lines and the outline at 21 controls. Browser errors are
empty and final data matches a complete capture. Native lifetime high water is 1.210 GiB.
That browser workload has more background repetitions than the earlier retention run,
so its process peak is not a paired memory-saving comparison.

[Browser observations](results/browser-pytorch.json) and
[screenshot](results/browser-pytorch.png) preserve the measurements. Source bytes are
freshly probed in disposable copies. Five edits and one header operation do not establish
a latency tail, GPU completion, or physical presentation behaviour.

## Correctness and mutation evidence

New core/native tests first failed before the interfaces existed. They verify owned
buffer adoption, unchanged recapture sharing, same-size changed bytes, UTF-8/unsupported
input, line ranges, equivalent document versions, independent validation, sharing
across changed contexts, cancelled full/hinted capture, clean retry, old pins, and
clean-capture agreement. Existing source-policy, invalid-baseline, watcher, syntax
retention, and publication tests continue to pass.

Formatting, **169 Rust tests**, doctests, Clippy, and **42 Node/browser checks** pass.
The complete affected-module mutation run covers core captured source/model and native
source/engine: **201 tested, 139 caught, 56 unviable, six timeouts, zero survivors**.
No new exclusions were added; the existing equivalent directory-prefix comparison
exclusion remains. Timeout mutants prevent complete capture/refresh results and leave
startup fixtures waiting; those are separate from assertion catches.
The [mutation report](results/mutation.json) records scope, command, source hashes,
non-caught outcomes, and limits. Standard-library allocation internals, dependency
grammars, macro expansion, filesystem event delivery, and browser execution are outside
that Rust mutation scope. Coverage evidence is not proof of correctness.

The [before-source manifest](results/before-source-sha256.json) and
[after-source manifest](results/after-source-sha256.json) identify implementations and
binaries measured before this change's commit. No dependency or unsafe-code exception
was introduced.

## Remaining work

Temporary duplicate read overlap is one file at a time, but candidate source still grows
with the workspace and different old/new contents may coexist. Unsupported-encoding
documents now retain their captured raw bytes internally; textual responses remain
unavailable. Analysis facts, syntax-cache state, parser scratch space, and transport
copies have separate lifetimes and budgets. This is no whole-process memory cap.

Full verification still visits and reads every selected file. Recovery scheduling and
capture-phase cancellation latency are not newly qualified by these measurements.
Serial new-source preparation, bounded analysis/capture admission, and bounded selected
source/outline transport remain architectural work. Keep those costs explicit before
expanding configured compiler analysis.
