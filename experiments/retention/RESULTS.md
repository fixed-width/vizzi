# Bounded syntax retention results

Opt-in retained trees reduce repeated large-header analysis time on both full reference
repositories. Linux's 24 MB AMD header improves from **703 ms to 163 ms** median engine
time; PyTorch's 8.4 MB Perfetto header improves from **1,532 ms to 645 ms**. A paired
browser observation improves the Perfetto edit from **1,645 ms to 794 ms**. Ordinary
edits still encounter periodic full reconciliation, including one 299 ms browser update.

The implementation keeps parsers worker-local, transfers only owned tree/source state,
and admits it through shared weight/count-limited leases. Published documents remain
immutable. It is disabled by default; `--syntax-cache-mib 512` enables the measured
policy. The [contract](../../docs/prototype/SYNTAX_RETENTION.md) defines ownership,
source/context checks, cancellation, fallback, and accounting.

## Native comparison

The [driver](README.md) uses the existing full Linux/PyTorch profiles and revisions.
Captured counts remain 66,066 and 10,578 files; PyTorch has 10,147 analyzed files and
431 source-only CUDA inputs. Header language assignments are explicit raw inputs, not
compiler configurations. Original archives are untouched and no analyzed project runs.

Each profile has separate disabled/enabled processes on disposable source copies.
Every file/position starts with a new private cache, one recorded priming edit, and
three subsequent edits. Start, middle-line-boundary, and end positions give nine
measured updates per file/condition. Probe IDs differ by condition, preventing earlier
persisted results from satisfying an edit. All 96 priming/measured cases read and parse
one file with zero persisted hits and no full reconciliation. Each matches an
independent fresh parse bypassing the analysis cache and a complete snapshot capture.

Times include source verification, diff, parse/extraction, document validation, cache
writes, and snapshot construction. Disk writes to the edited file and correctness
verification are outside engine timing. The host is the same i7-13700H with 20 logical
CPUs, about 62.5 GiB RAM, Rust 1.99.1, four shared workers, and warm OS caches.
Current-context analysis-cache preparation is excluded. No builds, tests, mutations,
or other benchmark jobs compete with timing runs.

| File | Disabled median | Retained median | Retained range |
| --- | --- | --- | --- |
| Linux `kernel/locking/mutex.c` | 33.6 ms | 32.3 ms | 31.2–50.6 ms |
| Linux 24 MB AMD generated header | 702.7 ms | 163.2 ms | 135.8–219.8 ms |
| PyTorch `c10/core/Device.cpp` | 5.13 ms | 3.93 ms | 3.68–6.61 ms |
| PyTorch 8.4 MB Perfetto header | 1,531.6 ms | 645.3 ms | 522.7–705.7 ms |

All 36 enabled measured updates report one tree reuse; disabled and priming cases
report zero. Linux small-file ranges overlap, so the data does not establish a useful
improvement for every ordinary edit. Large-header median improvements are about 4.3×
and 2.4×. These are distinct comment edits with a small varying marker, a favorable
incremental workload. Correctness fixtures also cover structural/Unicode changes;
their tests do not establish structural-edit performance or latency tails.

[Native observations](results/native-observations.json),
[summary](results/native-summary.json), and [CSV](results/native-summary.csv) retain
inputs, priming, samples, counters, source checks, process observations, and limits.
Disabled then enabled is a fixed order, not a randomized experiment; system activity
and power settings remain uncontrolled. One targeted file changes at a time despite
the four-worker configuration. Concurrent quota behavior has separate fixture tests.

## Admission weight and memory

The measured policy allows 512 MiB total weight, 256 MiB per new state, and eight
entries. Weight is source bytes plus 128 per descendant node plus 4,096. Checkout
charges old state and the new allowance together. Busy keys or insufficient quota
fall back to ordinary analysis without waiting for another lease. Full scans do not
fill the cache with cold-indexed files.

| Retained header | Idle source + tree weight | Peak reservation charge |
| --- | --- | --- |
| Linux AMD | 121.5 MiB | 377.5 MiB |
| PyTorch Perfetto | 244.3 MiB | 500.3 MiB |

All measured cache charges fit the configured quota and finish with zero active
leases. Each file/position has an isolated private working set, so these runs do not
qualify switching among many large files. A Perfetto update uses nearly the whole
512 MiB reservation budget and may evict other idle entries in a shared working set.

An [exploratory calibration](results/calibration.json) records source/node counts and
RSS with an isolated tree alive after dropping its parser. It informed the weight;
it does not prove an allocation upper bound. Parser scratch space, ordinary job buffers,
immutable documents/facts, temporary over-limit trees, and allocator retention are
outside this cache quota. There is no new whole-process memory cap.

Sampled native process peaks, including fresh-parse/complete-capture verification,
were 5.872/5.937 GiB disabled/enabled for Linux and 1.340/1.273 GiB for PyTorch.
Point RSS before verification is also recorded but can retain allocations from earlier
verification. Lower enabled PyTorch RSS is not evidence of a memory-saving guarantee.
Sampling around every 50 ms can miss peaks. These cases do not replace the earlier
retained-reparse matrix or qualify the 16 GB laptop.

## Paired browser observation

Headless Chromium 156 uses the existing Canvas2D reference view at 1440×900 with
software graphics. The same native binary and profile run once with each setting:
one four-second steady and one refresh-motion run per condition, five ordinary edits,
and one large-source open/edit. Periodic recovery remains enabled. Earlier background
edits warm the enabled header state; this is not a first-edit claim.

| Browser observation | Disabled | 512 MiB retention |
| --- | --- | --- |
| Perfetto header opening | 192.8 ms | 188.9 ms |
| Header edit to matching DOM plus two rAF callbacks | 1,644.8 ms | 794.4 ms |
| Header edit maximum rAF interval | 50.0 ms | 33.4 ms |
| Ordinary edit median / maximum | 101.8 / 299.0 ms | 87.7 / 298.8 ms |

All four overview runs have a 16.8 ms rAF p99 and no rAF intervals over 50 ms.
Both header openings have no observed long animation frames. Header editing records
one 71.4 ms frame disabled (3.1 ms blocking) and one 63.8 ms frame enabled (zero reported
blocking). Source remains at 56 rendered lines and the outline at 21 controls.

The enabled header's sampled update reports one read/parse/tree reuse and 588.0 ms
engine time. Disabled sampled metadata is a later no-op full recovery, so it is not
used to attribute the causal parse duration. Both conditions have empty browser errors
and complete-capture agreement. Their first changed inputs are unique, preventing
persisted-cache replay. The enabled 299 ms ordinary outlier records full reconciliation,
one actual parse, and tree reuse; cheap parsing does not remove whole-scope recovery.
The proposed five-edits-below-200-ms gate therefore remains unmet in this probe.

[Disabled report](results/browser-0.json), [enabled report](results/browser-512.json),
[disabled screenshot](results/browser-0.png), and [enabled screenshot](results/browser-512.png)
preserve observations. Native timing, browser DOM/rAF timing, GPU completion, and
physical presentation are different endpoints. One header observation and five ordinary
edits per condition do not establish a tail or a general stutter-free guarantee.

## Correctness and mutation evidence

New interface/cache/probe tests first failed before their implementations existed.
The final suite has **165 Rust tests**, doctests, formatting, and Clippy passing.
**42 Node/browser checks** also pass with retention enabled in the native HTTP fixture.
Tests cover all four languages, structural/Unicode/multiline edits, recovery, source
and full-context matching, retention rejection, cancellation/recovery, old pins,
fresh-analysis equivalence, deletion/reassignment, live host reuse, admission boundaries,
LRU eviction, invalidation, and shared concurrent quota.

Mutation phases are deliberately reported separately:

| Scope/phase | Tested | Caught | Unviable | Timeouts | Survived |
| --- | --- | --- | --- | --- | --- |
| Broad affected Rust modules after initial gap fixes | 386 | 322 | 47 | 15 | 2 |
| Complete updated edit/state module | 70 | 56 | 4 | 10 | 0 |

The broad run's two remaining edit-boundary survivors led to insertion/deletion tests
and an equivalent simplification of the remaining suffix bound. Retesting the complete
changed module resolves both. The other nine module bodies match the broad-run hashes;
the phases overlap and cannot be summed as unique mutations. This is not a single
whole-scope zero-survivor run of every final file. There are no unresolved survivors.

The existing equivalent directory-prefix exclusion remains in the broad run; no new
exclusions were added. Timeouts include broken admission/diff loops and startup waits
when no complete result is produced; they are separate from assertion catches. A
manual deletion of tree-edit application fails the fresh-analysis test, and exact
source restoration is verified. The [mutation report](results/mutation.json) preserves
commands, phases, development gaps, hashes, non-caught outcomes, and limits.

The [source manifest](results/source-sha256.json) identifies the working tree and binaries
measured before commit. No dependencies or unsafe-code exceptions were added.

## Next work

Keep retention opt-in while assessing broader working sets and baseline hardware.
Necessary full fact extraction, validation, source/outline transport, linear snapshot
merging, and periodic full capture remain material. Capture memory overlap and bounded
source/projection delivery are the next architectural work; neither is solved by
retaining syntax trees. Compiler-backed contexts remain a separate later stage.
