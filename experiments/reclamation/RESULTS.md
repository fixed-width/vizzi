# Retired source ranges return to the spool

The native spool now reuses retired ranges and truncates free tails while preserving
immutable source pins. The same source-history workloads complete with **8 GiB Linux /
1 GiB PyTorch backing allowances**, committing **9.018 / 1.124 GB cumulatively**. Peak
reserved spans are **7.006 / 0.873 GiB**. After every source is released and maintenance
runs, both backing files have **zero length while their spools remain open**.

The portable source/read interfaces, identities, default resident capture and dependency
versions are unchanged. No unsafe code was added. The [source reference](../../docs/prototype/SOURCE_READS.md)
defines capacity/maintenance diagnostics; the [design](../../docs/plans/spool-reclamation.md)
records ownership and publication-lock constraints. The [procedure](README.md) reproduces
the measurements.

## Ownership and reclamation

Each backing descriptor holds one allocation. Snapshot retirement queues a range only
when its last source/reader owner disappears; an expired external query pin keeps its
bytes allocated and readable. The queue uses a short separate memory lock. Queueing
does not take the backing IO lock or capture writer and performs no explicit backing IO.

Capture/commit drains retirements before allocating. `SourceSpool::reclaim()` supports
explicit idle maintenance outside publication locks. A private allocator uses offset
and size indexes for best-fit lookup, splitting, coalescing and tail release. Live ranges
never relocate. An unfinished copy remains reserved during concurrent maintenance.
Failed/cancelled copies return reservations without damaging neighboring pins.

Interior gaps become reusable but remain in the file until reused or joined to a free
tail. Physical tail cleanup is batched. Maintenance reports IO errors, retains retry
state and does not pretend failed truncation succeeded. Staging retains its separate
existing bound and best-effort cleanup. Final store/descriptor closure still closes
anonymous backing; this is ephemeral session storage, not crash recovery.

## Representative histories

The same pinned Linux/PyTorch scopes contain 66,066 / 10,578 selected files and
1,495,484,792 / 186,092,681 baseline source bytes. Each marked owned copy verifies a
baseline and unchanged sharing, creates four whole-scope prefix generations with an
expired external baseline pin, releases the pin, restores sources and releases history.
Source manifests match the preceding report; cumulative stores, committed bytes and
verified backing reads also match. Original archives remain unchanged and target hashes
are restored. All old-pin, source-byte, snapshot, sharing and weak-handle checks pass.

| Observation | Linux | PyTorch |
| --- | --- | --- |
| Backing span allowance | 8 GiB | 1 GiB |
| Peak reserved span / recorded file length | 7.006 GiB | 0.873 GiB |
| Cumulative committed bytes | 9.018 GB | 1.124 GB |
| Gap after baseline query pin release | 1.495 GB | 0.186 GB |
| Baseline capture/validation | 4,203 ms | 632 ms |
| Unchanged capture | 2,310 ms | 339 ms |
| Fourth changed capture/validation | 9,261 ms | 1,321 ms |
| Maintenance after baseline pin release | 14.8 ms | 1.8 ms |
| Final maintenance and tail truncation | **800.7 ms** | **122.0 ms** |
| Sampled process peak RSS | 2.267 GiB | 0.308 GiB |
| Final allocation/span/file/staging/retirement queue | 0 | 0 |

The released baseline gap supplies restoration without increasing the high-water span.
The restored phase also retires the next expired generation, leaving another reusable
gap. Final maintenance runs with the spool still open, demonstrating reclamation before
session closure. File lengths and spans differ from filesystem block allocation.

Maintenance is outside the application publication lock, but currently holds the backing
IO mutex while processing its batch and truncating. **Large cleanup can block backing
readers or captures**; the measured 801 ms final Linux cleanup occurred after all query
handles were released. This is not an interactive latency qualification or a hard cleanup
deadline. Keep this cost visible before adding idle/background maintenance scheduling.

The [preceding reuse report](../verified-reuse/RESULTS.md) recorded 2,172 / 263 ms unchanged
and 7,503 / 1,075 ms fourth-change phases. Current timings are higher; these are historical
observations, not a paired speed comparison. This stage establishes reusable capacity
and ownership correctness, not a throughput improvement. Maintenance before each phase's
inventory is measured separately; automatic capture maintenance is included in capture.
Source-storage phases also include independent byte/prefix validation and verified reads.
Parsers, facts, watcher scheduling and UI work are omitted.

The i7-13700H host exposes 20 logical CPUs and about 62.5 GiB RAM, with Rust 1.99.1.
No builds, tests, mutation jobs or other benchmarks compete during timing. OS caches are
warm. The 12 GiB child address-space limit constrains virtual memory, not RSS/physical RAM
or combined editor/browser/OS use. Process peaks include rewriting, inventory and verification.
Single sequential conditions do not estimate timing tails or baseline-laptop performance.
Admission/browser latency workflows were not repeated; integrated correctness checks ran.

## Correctness and mutation evidence

The first reclamation tests failed before the new maintenance interface/accounting existed.
Tests cover full-capacity reuse, final-owner/pin lifetime, empty captures, splitting,
best fit, adjacent coalescing, fragmentation refusal, 100 repeated churn cycles, store
release with a reader alive, and final truncation. An independent byte-occupancy model
checks 16,000 deterministic mixed allocation/release operations. Native failure tests
cover partial writes/cancellation in reused gaps, reserved extents during concurrent
maintenance, read-only write/truncation failures and retry. Descriptor retirement is
checked while the backing IO lock remains held, proving it does not wait for that lock.

Formatting, **258 Rust tests**, doctests, Clippy and **58 Node/browser checks** pass.
Complete affected allocator/spool scopes yield **117 final unique mutation outcomes:
88 caught by tests, 24 unviable, five timeouts, zero survivors**. All unmodified baselines
pass. Two initial survivors prompted a direct allocator tail-change signal instead of a
redundant comparison, and a regression test for failed writes whose backing prefix already
has the correct length. Complete final allocator and maintenance scopes replace their
earlier outcomes; unchanged spool methods retain the broad-run outcomes without duplication.

No new exclusions were added. Existing unrelated engine directory-prefix configuration
is unchanged. Timeouts break read/copy progress and are separate from assertion catches;
unviable substitutions fail compilation/build. Portable source logic is unchanged;
optional probes/Python, dependency and platform/browser internals are outside this mutation
scope. Passing checks and zero survivors are evidence, not proof.

## Remaining limits

Reclamation waits for maintenance or final backing closure; idle callers use the explicit
maintenance interface. Source pins can legitimately retain all capacity. Interior holes
and unused tail space may still fail a larger contiguous allocation through fragmentation;
compaction/relocation is deferred. Retired queue/free-index memory and source metadata
remain input-dependent. The backing span and equal staging allowance are separate and
their combined file lengths may reach twice the configured value. There is no resident,
parser/fact/response or whole-process memory cap, and no hard IO/cancellation deadline.

[Raw observations](results/observations.json), [summary](results/summary.json),
[corpus CSV](results/corpus-summary.csv), [mutation details](results/native-mutation.json),
and [source hashes](results/source-sha256.json) preserve code, binary and workload identity.
