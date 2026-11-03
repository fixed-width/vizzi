# Keep pinned reads independent of spool maintenance

The [reclamation report](../../experiments/reclamation/RESULTS.md) measures an 801 ms
Linux final cleanup while holding the backing IO mutex. Separate cursor IO from allocation
and maintenance so live prefix reads do not wait for that mutex during coalescing or truncation.

## Lock and handle ownership

Keep allocation, reservation, cumulative-write and staging accounting under the existing
state lock. It remains held through tail truncation, so a new reservation cannot be
created above the selected cut point. Already-reserved writes remain within that cut
point, including during concurrent maintenance.

Place the backing cursor and read counters under a separate IO lock. Reads and writes
hold it only for each seek/read or seek/write chunk. No backing operation holding this
lock subsequently acquires allocation state. Readers must not access allocation state
or the retirement queue. Snapshot metadata and source identities remain unchanged.

Create one additional file handle per spool using safe `File::try_clone`. The maintenance
handle shares the cursor, so use it only for metadata and `set_len`, which do not move
the cursor. Every seek/read/write uses the cursor lock. Truncation holds allocation
state but never that cursor lock. Keep descriptor retirement memory-only and retain
all existing quota, rollback, retry and pin-lifetime rules. Diagnostics may wait for
maintenance, as may captures needing allocation/state; this stage targets pinned reads.

The OS/filesystem can still serialize or delay reads during truncation. Removing a Rust
mutex dependency is not a hard IO deadline or proof of UI latency on all platforms.
No unsafe code, new dependencies, per-source file handles or background workers are needed.

## Validation and measurement

Use TDD to reproduce a pinned read waiting on allocation state, then verify reads during
a deliberately paused real truncation. Check that allocation cannot pass the truncation
gate, bytes remain correct after it, errors remain retryable, cursor reads stay isolated,
and concurrent maintenance never cuts an existing reservation. Keep the allocator's
independent occupancy model and existing source integrity/ownership tests.

Run workspace/doctest/fmt/Clippy/browser checks and full affected native mutation scopes.
Repeat pinned Linux/PyTorch histories and add concurrent bounded reads around real tail
cleanup, recording overlap, latency distributions, byte checks, disk/accounting data and
source/binary hashes. Separate scheduler/filesystem observations from deterministic lock
isolation tests; report timeouts and compiler failures separately from caught mutations.

## Delivery evidence

The [comparison](../../experiments/readable-maintenance/RESULTS.md) uses the preceding
spool with identical new instrumentation and a verified distinct current executable.
During large cleanup, Linux/PyTorch complete 319 / 57 pinned reads versus zero before;
maximum observed overlapping latency is 0.120 / 0.131 ms versus 737 / 108 ms. All pin,
source, reservation, retry and final release checks pass. Full relevant mutation scopes
have no survivors, with compiler failures/timeouts reported separately.

Cleanup still takes hundreds of milliseconds and freezes allocation state. This stage
isolates pinned reads from its Rust mutex; capture/state diagnostics, filesystem latency,
broader interactive workflows and baseline hardware remain separate concerns.
