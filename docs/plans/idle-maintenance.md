# Schedule source cleanup after a quiet interval

Two backing files allow capture during physical cleanup, but the host does not yet drain
retired disk space while idle. Add bounded admission of source maintenance without
changing publication tickets or doing IO under coordinator/application locks.

## Scheduling and ownership

The directory adapter supplies an optional native maintenance interface with an in-memory
work hint and a blocking operation. A host without spooling supplies none. The scheduler
waits for 250 ms without foreground or recovery work, then admits at most one worker when
the hint reports queued retirements or charged tails. Coordinator checks are metadata-only;
source statistics/filesystem metadata are not polled there. Poll at most every 100 ms
with a source attached so external pin release can be noticed without another edit.

Foreground/recovery activity resets the quiet interval. New work can race admission;
already-running cleanup is not preemptible and uses the existing two-file safety policy.
The coordinator only joins finished workers. Shutdown stops further admission and joins
the current worker; no hard shutdown/IO deadline is promised.

Failures and worker panics retain disk charges through the backing interface and retry
after at least one second plus the idle rule. Report attempts, running/completed/failed
and the last error through a separate native host status getter; never overwrite foreground
error, observation identity or recovery status. A successful retry clears the error.
Thread creation failure follows the same retry path.

`HostOptions::idle_maintenance` defaults true but is relevant only with a provider. Native
embedders and controlled probes can disable it for caller-managed cleanup. Existing manual
truncation probes opt out explicitly. No new browser/HTTP fields, CLI switch, dependency
or unsafe code are needed in this stage.

## Verification and measurement

Use TDD for idle/active boundaries, no provider/no work, one worker, nonblocking collection,
failure/panic/backoff/success, pin release without new hints, unchanged publication on error,
foreground publication during paused work, shutdown joining, and explicit opt-out. Use
virtual clock inputs and controlled maintenance implementations for deterministic scheduling
tests, plus a real spooled host integration test.

Run workspace/doctest/fmt/Clippy/browser checks and complete affected scheduler/host/provider
mutation scopes. Use an observed real maintenance provider in qualification to record actual
cut intervals and foreground publication on pinned Linux/PyTorch copies. Verify source/facts,
old pins, restoration, idle release and accounting; keep setup/checks outside timing. Finite
warm native samples do not establish browser tails, physical RAM limits or laptop defaults.

## Delivery evidence

The [automatic runs](../../experiments/idle-maintenance/RESULTS.md) complete eight operations
per host without failure, and all six during-cleanup edits publish before actual truncation
ends. During-cleanup publication medians are 67.5 / 33.6 ms; capture gates remain below
0.01 ms. Source/fact/pin/restoration checks pass and final verification releases backing.
Full relevant mutation scopes have no survivors, with compiler failures/timeouts separated.

The implementation exposes native progress without changing application/HTTP status. Idle
admission can race new activity and a running syscall cannot be interrupted. Constants,
other contention patterns, watcher/browser latency, blocked shutdown and baseline hardware
remain qualification work rather than guarantees from these finite warm observations.
