# Pinned reads continue during spool maintenance

Pinned backing reads no longer acquire allocation/maintenance state. In the Linux and
PyTorch cleanup observations, **319 / 57 reads complete during maintenance**, versus
zero with the preceding locks. Maximum observed overlapping read latency changes from
**736.826 / 108.293 ms to 0.120 / 0.131 ms**. All prefix/source/pin/restoration and
capacity checks pass; final file lengths remain zero after the reader pin is released.

The [design](../../docs/plans/readable-maintenance.md) records lock/handle ownership,
the [reference](../../docs/prototype/SOURCE_READS.md) defines the native contract, and
the [procedure](README.md) reproduces the comparison. Default resident capture, source
identity, retention policy and dependencies are unchanged. No unsafe code was added.

## Lock isolation

Allocation state owns a maintenance handle used only for metadata and truncation. It
stays locked through coalescing/truncation so new reservations cannot cross a stale cut
point. Existing reservations remain protected. Cursor reads/writes and their read counters
have a separate lock, held for individual seek/IO chunks. Readers never acquire allocation
state; cursor IO releases its lock before any state update. Diagnostics acquire state
before observing cursor counters. There is no cursor-to-allocation lock cycle.

The safe cloned handle shares a cursor with the backing handle. `metadata` and `set_len`
do not move it; all seek/read/write operations remain behind the cursor lock. One extra
handle is opened per spool, while the two anonymous files remain unchanged. Descriptor
retirement stays memory-only. Quotas, immutable pins, reuse, rollback and truncation-error
retry retain their existing behaviour.

This removes a Rust mutex dependency. Captures, diagnostics and maintenance needing
allocation state can still wait. Cursor reads compete with other cursor IO, and the OS
or filesystem can still delay reads during truncation. There is no hard IO deadline.

## Real cleanup with a retained prefix

Both builds use the same measurement harness. The baseline is committed spool code from
`0d0e20e`; the current build changes the locks and handles. The harness files match
byte-for-byte, and distinct executable hashes are verified before timing. An initial shared
Cargo output collision was detected and corrected using an isolated baseline target and
a clean current native build. No data from the colliding executables is included.

The source-only history workload repeats the pinned Linux/PyTorch scopes with 8 / 1 GiB
backing allowances. A separate 64 KiB prefix pin stays alive while all corpus snapshots
retire. A sampler performs fresh verified 64 KiB reads about every 2 ms, checks every byte,
and records intervals around maintenance. It has a ready handshake and a 4,096-sample cap;
no measured phase reaches that cap. The prefix is released in an additional final phase.

| Large suffix cleanup observation | Linux before / after | PyTorch before / after |
| --- | --- | --- |
| Maintenance duration | 738.608 / 682.141 ms | 110.348 / 123.874 ms |
| Reads completed during maintenance | **0 / 319** | **0 / 57** |
| Reads completed before maintenance midpoint | 0 / 159 | 0 / 28 |
| Overlapping read samples | 1 / 319 | 1 / 57 |
| Maximum overlapping read latency | **736.826 / 0.120 ms** | **108.293 / 0.131 ms** |
| Current overlapping read p95 | 0.090 ms | 0.113 ms |
| Sampled process peak RSS | 2.268 / 2.268 GiB | 0.308 / 0.308 GiB |
| Peak reserved backing span | 7.006 / 7.006 GiB | 0.873 / 0.873 GiB |
| Final file length after prefix release | 0 / 0 | 0 / 0 |

The preceding lock blocks its first overlapping read until cleanup ends. The new reads
complete throughout the interval, including well before its midpoint, demonstrating
progress beyond a boundary-timestamp race. Before each cleanup there is an initial
verified read; that non-overlapping sample is excluded from latency figures above.
The old case has only one overlapping read per profile, so it does not establish a tail
distribution. Current p95 and maxima describe these finite samples only.

Cleanup itself remains a long operation; PyTorch cleanup is slower in the current sample.
This change targets source-read isolation, not shorter truncation or capture throughput.
The probe uses warm backing-prefix pages and bounded core reads; it does not measure
HTTP, window projection, browser presentation or arbitrary cold-file reads.

All source manifests, cumulative corpus writes, per-phase owned buffer inventories,
old pins, unchanged sharing, restoration and weak-handle checks agree. The additional
64 KiB anchor write/store and every sampled backing read are accounted independently
against the preceding source workflow. Original archives remain unchanged; only marked
owned copies are edited and restored. The final anchor is released and its backing
truncated while the spool is still open.

The i7-13700H host exposes 20 logical CPUs and about 62.5 GiB RAM, with Rust 1.99.1.
No builds, tests, mutation jobs or other benchmarks compete during timing. OS caches
are warm. The preceding build runs first and the current build second, once per profile;
these are not randomised trials or general latency tails. The 12 GiB child address-space
limit is not an RSS/physical-RAM limit or baseline-laptop qualification. Parser/fact,
watcher and UI work are omitted; source rewriting and verification contribute to peaks.

## Correctness and mutation evidence

A regression test first fails because a pinned read waits on allocation state. It passes
after isolation. Deterministic paused-truncation tests verify reads complete before the
pause releases, allocations wait, successful and failed truncations preserve bytes,
retry remains valid, and subsequent writes are not cut off. Existing corruption, short
read, reservation, concurrent-cursor, retirement, fragmentation and allocator occupancy
tests pass. The optional sampler also has a source-history/prefix-release test.

Formatting, **261 Rust tests**, doctests, Clippy and **58 Node/browser checks** pass.
Complete affected spool/allocator mutation scopes produce **122 unique outcomes:
89 caught by tests, 28 unviable, five timeouts, zero survivors**. All unmodified baselines
pass. One initial survivor revealed that the read-only failure fixture needed both cloned
handles made read-only; an exact-mutant rerun catches it and replaces the earlier outcome
without duplication. No new exclusions were added; existing unrelated engine configuration
is unchanged.

Timeouts break input/copy progress and are separate from assertion catches. Unviable
substitutions fail compilation/build. Portable source logic, optional probes/Python,
dependency and platform/browser internals are outside the native mutation scope.
Passing checks and zero survivors are evidence, not proof. Windows/macOS runtime and
filesystem latency are not qualified by these Linux observations.

## Remaining work

Allocation and capture maintenance can still block, and cleanup is not preemptible.
Retirements remain deferred until maintenance or final closure. Fragmentation, compaction,
metadata/queue memory, parser/fact/response allocations and the combined process envelope
remain separate work. The equal staging allowance is still additional to backing capacity.
Broader interactive workloads and baseline hardware are needed before choosing defaults.
The subsequent [foreground edit report](../foreground-maintenance/RESULTS.md) measures
capture/publication delay at the allocation gate while real cleanup is active.

[Raw observations](results/observations.json), [summary](results/summary.json),
[read CSV](results/read-summary.csv), [mutation details](results/native-mutation.json)
and [source hashes](results/source-sha256.json) preserve workload, code and binary identity.
