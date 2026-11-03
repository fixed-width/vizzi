# Analysis input admission reference

Admission is an opt-in native scheduling limit on active captured input bytes. It is
independent of [retained syntax](SYNTAX_RETENTION.md), parser-worker limits and source
selection. It does not cap parser scratch/facts, retained parser allocations, snapshot
source/index storage, allocator retention, total RSS, or virtual memory.

## Shared capacity and priority

`Admission::new(capacity, file_limit, foreground_reserve)` requires positive capacity
and file limit, `reserve < capacity`, and `file_limit <= capacity - reserve`. That last
condition allows any admitted file to run during recovery when other work is quiet.
`Admission::for_budget(bytes)` reserves one quarter for foreground work and uses the
remaining capacity as the per-file limit, using integer division.

`Engine::with_admission(Arc<Admission>)` shares capacity across configured engines,
attempts and languages. The host marks private verification as `Priority::Recovery`;
initial/mandatory/event refreshes and recovery overlays use `Priority::Foreground`.
The existing worker budget still applies independently.

`acquire(input_bytes, priority)` charges the captured input size, or one byte for an
empty input. Total active charges never exceed capacity, and recovery charges never
exceed `capacity - reserve`. Foreground can use total free capacity. The reserve is
headroom for ordinary foreground jobs; it neither preempts active parsers nor promises
that every large foreground request fits immediately.

Acquisition never queues or waits for another analysis job. `Refusal::FileLimit` means
the input exceeds the per-file limit. `Refusal::Busy` means capacity is occupied or
reserved. A permit is acquired before persisted-cache loading, parser creation and
analysis and is released on completion, cancellation, error or unwinding. Complete
immutable document reuse needs no new permit; source is still reopened/hashed under
normal capture rules. Syntax leases and input permits charge different resources.

## Deferred coverage and retry

A refusal produces a valid captured document with the requested language/context,
exact source identity, `coverage: "deferred"`, empty facts, and a resource diagnostic.
Language classification and encoding are preserved. Deferral is distinct from
`source_only`, unsupported encoding, grammar recovery and completed syntax analysis.
No old facts are attached to new source. Unsupported encoding can coexist with deferral;
its existing text-preview limitation remains explicit.

Deferred records cannot contain facts. They are rejected by analysis-cache storage and
loading, and never reused as completed in-memory analysis. A later full verification
or path hint for that file retries admission without requiring a content change.
Over-limit files stay deferred until input or configuration changes. No project tooling
is executed, and no extra retry parse is added to the current attempt.

Resource contention can change intermediate analysis availability. Latest-ticket
publication, old immutable pins and complete-context validation remain in force.
Once contention settles, eligible deferred files can complete through periodic recovery.
Continuous mutation still needs a finite publication window; no hard latency deadline
or starvation-freedom guarantee is introduced.

`Measurements.deferred` and `View.deferred` count files with any deferred analysis over
the whole snapshot, independently of overview filtering. `analyzed` retains its existing
meaning of files with an analysis record, including unavailable/deferred records. The
browser reports deferred files separately from files with completed analysis, displays
the reason, and preserves source-window access. It does not invent an empty successful
outline or hide a file because admission declined it.

## Configuration and diagnostics

`--analysis-input-mib N` enables the host policy above; `0` disables it and is the default.
The value is converted with checked MiB arithmetic. At `32`, total active input capacity
is 32 MiB, recovery capacity/per-file limit is 24 MiB, and foreground headroom is 8 MiB.
These are input-volume limits, not a recommendation for a process-memory envelope.

`Admission::stats` reports configured limits, active/recovery charges, job count, peak
charge, and cumulative oversize/busy refusals. Reading it does not access source or run
analysis. Captured source is already immutable observation storage before job admission;
this stage does not bound capture buffers or total retained source/history. Those need
a separate storage/capture design.

See the [measurement procedure](../../experiments/admission/README.md),
[results](../../experiments/admission/RESULTS.md), and
[delivery plan](../plans/indexing-admission.md) for resource checks and remaining limits.

Analysis over backed source acquires its input permit before an explicit verified full
load and holds it through cache/parser/fact validation. Pinned inspection uses bounded
range sessions and remains available for deferred inputs. See [source reads](SOURCE_READS.md).
