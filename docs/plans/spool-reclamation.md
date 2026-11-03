# Reclaim retired captured-source extents

The [verified reuse stage](../../experiments/verified-reuse/RESULTS.md) avoids unchanged
metadata work, but cumulative append capacity eventually fills even after source handles
die. Reuse retired ranges in the existing anonymous backing file without relocating live
bytes, multiplying file descriptors or changing portable source identities.

## Ownership and maintenance

Each native backing descriptor owns one allocation. Its last release queues that allocation
for retirement using a separate short memory-only lock. Publication may drop snapshots
under its application lock, so descriptor destruction must not acquire the backing IO
mutex, truncate files or wait for the capture writer.

`SourceSpool::reclaim` drains queued retirements, coalesces adjacent free extents and
truncates a free tail. Capture/commit paths invoke maintenance before allocating. An idle
caller may invoke it explicitly outside publication locks. Reclamation is deferred until
maintenance or final backing closure, not tied merely to publication history expiration.
External source/query pins keep their descriptors and ranges alive.

Use a private allocator with offset and size indexes: best-fit lookup/splitting and adjacent
coalescing take logarithmic time in free-range count. A reserved range remains charged
through copying, so maintenance cannot truncate or reuse an unfinished write. Cancellation
or IO failure retires the reservation and runs cleanup before returning. Published ranges
are never overwritten; readers retain existing block verification and synchronized IO.

## Capacity and diagnostics

The configured backing capacity bounds the reserved address span; the existing staging
file retains its separate equal bound. Interior holes become reusable but are not physically
punched out. Free tails shrink on successful maintenance. Fragmentation can refuse a
contiguous allocation even when total free bytes suffice; compaction/relocation is deferred.

Keep `written_bytes` and store/read counters cumulative for diagnostic compatibility.
Expose allocated bytes (including queued retirements and in-flight reservations), reserved
span, reusable gaps, peak span, pending retirement count and actual file length separately.
Maintenance reports IO failure and retries truncation on later calls; diagnostics must not
claim failed physical cleanup succeeded. Final descriptor/store closure still releases
anonymous backing. Free-index and retirement-queue memory remain outside a process cap.

## Validation and measurement

Use TDD for last-owner release, external pins, split/coalesced holes, best fit, fragmentation,
empty captures, repeated churn beyond cumulative capacity, final truncation and both resident
persistence and streaming. Check cancellation/failure in reused holes and active reservations,
concurrent reads/retirements and maintenance failure/retry. Compare allocation layouts with
an independent occupancy model under deterministic mixed operations.

Run workspace/doctest/fmt/Clippy/browser checks and full affected native allocator/spool
mutation scopes, separating timeouts and compiler failures. Measure bounded churn and pinned
Linux/PyTorch source histories after validation, recording per-phase disk/accounting data,
old-pin checks, restoration, release, timing, memory and source/binary hashes. No analysed
project tooling, unsafe code or new dependencies are required.

## Delivery evidence

The [measurements](../../experiments/reclamation/RESULTS.md) complete both histories
below their cumulative committed-byte requirements. Peak backing spans are 7.006 GiB
Linux and 0.873 GiB PyTorch, and maintenance truncates both files to zero after release.
All pin/source/restoration checks pass. Workspace/browser checks and complete relevant
mutation scopes have no survivors, with timeouts and compiler failures separated.

Final cleanup takes about 801 / 122 ms while holding the backing IO mutex. Publication
eviction only queues retirements, but explicit/automatic maintenance can delay source
reads or captures. The next design should isolate or subdivide maintenance while retaining
reservation/pin safety; no hard filesystem IO deadline is established here.
