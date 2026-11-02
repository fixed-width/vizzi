# Keep foreground edits independent of periodic recovery

The [windowed-delivery measurements](../../experiments/windows/RESULTS.md) retain a
301 ms ordinary-edit outlier when a hint is promoted into the periodic full pass.
Before this change, the pending queue merged both kinds of work into one full scope.

## Scheduling and consistency

Keep directory/error/rescan/overflow discovery requests mandatory. Periodic recovery
instead runs as one private candidate, without changing the foreground publication
ticket. Ordinary path events can publish from the current observation while that
candidate continues. Record a bounded union of every affected path during recovery,
including paths already published by foreground work.

Before publishing recovery, freshly read/hash that journal against its verified private
candidate. Never overwrite journal paths merely from an earlier published document:
that could lose a later missed edit. New events during this overlay make the result
stale and leave the journal/candidate available for retry. Hard discovery changes or
journal overflow invalidate the candidate and require a new full pass.

Only accepted latest-ticket results acknowledge foreground hints or finish recovery.
Private candidates never publish directly after intervening edits. Old source pins
remain immutable. Captures are observations, not atomic filesystem transactions.

## Resource and host interface

At most one background refresh and one foreground attempt run in a host. With at least
two configured parser workers, background uses one and foreground at most the remaining
workers while background is active. No independent per-language pool or unbounded work
queue is introduced. A one-worker host retains serial scheduling and its latency limits.
Normal buffers/parser allocations remain outside the syntax-cache retention quota.

Expose recovery progress separately from foreground `pending` and phase measurements.
Recovery metadata records accepted completions, discarded work, verification and overlay
measurements, and errors; do not add overlapping counters into a misleading single
phase. A configurable native host can deliberately use polling and accept explicit
scope hints, preserving an editor-host integration path and deterministic probes.

## Validation

Use TDD with an analyzer that holds a recovery parse: foreground edits must publish
before releasing it, then recovery must include missed changes without regressing those
edits. Test journal overflow, hard invalidation, stale overlays, retries, failure,
shutdown, same-size changes, silent edits, and parser concurrency. Retain real-watcher
coverage and compare final results with complete capture.

Run complete affected Rust-module mutation checks and normal workspace/browser checks.
Measure real Linux/PyTorch edits while recovery is active, with independent clean
agreement and separate foreground/recovery phases. A finite accepted-ticket window is
still required for publication under continuous mutation; this is no hard latency or
whole-process memory guarantee.

## Delivery

Implemented the private candidate, bounded journal, worker reservation, fresh overlay,
latest-ticket acceptance, separate status, configurable native observation, and explicit
hint entry point. Hard discovery remains mandatory; single-worker hosts remain serial.
The [reference](../prototype/RECOVERY.md) defines the resulting contract.

Held-parser tests cover foreground progress, worker width, completed-candidate discard,
interrupted overlay retry, silent edits, recovery errors, shutdown, no-op lifecycle,
and old source pins. A native-watch test checks newly discovered directories without
waiting for periodic recovery. [Measurement and mutation evidence](../../experiments/recovery/RESULTS.md)
records full Linux/PyTorch follow-up and remaining limits.
