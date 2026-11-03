# Idle source cleanup runs independently of foreground publication

Live hosts with source spooling now admit at most one maintenance worker after a quiet
interval. Each observed Linux/PyTorch host completes **eight operations with zero failures**.
All **six during-cleanup edits publish before real truncation ends**, and their source,
facts and old pins verify afterward. No manual cleanup thread admits these operations.

The [design](../../docs/plans/idle-maintenance.md) records admission/lifecycle rules;
the [native reference](../../docs/prototype/RECOVERY.md#idle-source-maintenance) documents
options/status; the [procedure](README.md) reproduces the workload. No unsafe code,
dependency, browser schema or CLI switch was added. Default resident capture is unchanged.

## Admission and lifecycle

The directory adapter supplies optional source maintenance. Its work hint reads queued
retirements and allocation metadata only. A host without a provider spawns no worker and
keeps its previous wait schedule. With a provider, a 100 ms wake ceiling notices external
pin release even without another source hint; foreground/recovery work resets the 250 ms
quiet interval. Only an idle coordinator with no outstanding worker admits cleanup.

Filesystem work runs outside coordinator/publication locks. The coordinator joins only
finished tasks and continues foreground work while cleanup runs. New work can race idle
admission, so a running syscall is not cancelled; the two-file store preserves reservations
and charges while capture proceeds elsewhere when space permits.

IO failure, worker panic and thread creation failure update separate maintenance status
and back off retries by at least one second, subject to idle eligibility. Successful
cleanup clears the last error. Foreground error, snapshot identity and recovery counters
are not overwritten. Native callers inspect `Host::maintenance_status()`; the application
and HTTP/browser status schemas are unchanged. `HostOptions::idle_maintenance` defaults
true and can be disabled for caller-managed cleanup.

Shutdown stops admission and joins the current worker exactly once, including scope-drop
cleanup. It does not start another pass to force externally pinned data out. Blocking IO
can delay shutdown indefinitely; neither cancellation nor shutdown has a hard deadline.
One-shot and non-host callers retain explicit maintenance.

## Automatic cleanup observations

The real host runs the same pinned 66,066-file Linux and 10,578-file PyTorch scopes.
An observed provider delegates the ordinary spool hint and physical operation. A minimum
removal filter reports the generated large tail rather than unrelated small cleanup.
There is no artificial delay at truncation entry.

Three alternating condition pairs use ordinary targets `kernel/locking/mutex.c` and
`c10/core/Device.cpp`. Both conditions stream pressure outside project source: 4 GiB Linux /
768 MiB PyTorch. The control waits for completed automatic cleanup before writing, while
the overlap condition writes after observed truncate entry. The host uses four parser
workers, explicit hints, no syntax/input quota, native watching disabled and an hour-long
recovery interval after initial verification. Cleanup is admitted by the real idle policy.

| Observation | Linux | PyTorch |
| --- | --- | --- |
| Native attempts / completed / failed | 8 / 8 / 0 | 8 / 8 / 0 |
| Retirement to actual large-cut entry | 13.3–97.2 ms | 0.7–92.1 ms |
| After-cleanup publication, median (range) | 82.7 ms (64.5–102.3) | 45.4 ms (33.2–47.6) |
| During-cleanup publication, median (range) | **67.5 ms (67.4–76.9)** | **33.6 ms (33.5–47.2)** |
| During-cleanup writer/initial-maintenance gate | below 0.01 ms | below 0.01 ms |
| During-cleanup truncate, median (range) | 519.7 ms (503.9–537.9) | 104.2 ms (93.4–111.3) |
| Actual capture overlaps / requested samples | 3 / 3 | 3 / 3 |
| Publications before truncation finishes | **3 / 3** | **3 / 3** |
| Sampled process peak RSS | 2.824 GiB | 0.504 GiB |
| Observed host shutdown, no blocked syscall | 0.275 ms | 0.230 ms |

The eight operations include the six large-tail operations and additional retirement work.
The quiet interval can already elapse during pressure setup, so retirement-to-entry mostly
observes polling/admission, not a fresh 250 ms user-idle wait. Virtual-clock tests check
the quiet boundary and reset rules independently. Lower during-cleanup medians are sample
variation, not evidence that cleanup improves edit throughput. The useful result is
publication progressing while the real large syscall remains outstanding.

Every edit reopens/parses one file, adds the expected declaration, matches an independent
fresh analyser and preserves the old source. Source and baseline identity are restored
between attempts; final full clean observations agree. After host/pins close, explicit
verification cleanup returns aggregate backing length/allocation to zero. Original archives
remain read-only and marked owned copies restore target hashes. No analysed project tooling
is run. Shared 8 / 1 GiB backing allowances and separate equal staging bounds are unchanged.

## Correctness and mutation evidence

The first real-host test fails before the option/status interface exists. Integration tests
prove idle reclamation without another source hint, unchanged publication, retained-query
lifetime, opt-out, foreground publication during paused work, and maintenance errors separate
from foreground errors. Scheduler tests use virtual clock inputs and controlled work for
quiet boundaries, busy resets, missing providers/work, one worker, retry/backoff/panic,
success clearing errors, nonblocking polling, terminal shutdown and explicit/drop joining.

Formatting, **285 Rust tests**, doctests, Clippy and **58 Node/browser checks** pass.
Complete affected scheduler/host/directory/spool mutation scopes yield **236 final unique
outcomes: 138 caught by tests, 67 unviable, 31 timeouts, zero survivors**. Passing unmodified
baselines precede interpretation. Initial survivors prompted the explicit idle-admission
truth table and a drop-only worker-join case. Final full constructor/admission scopes and
the exact drop rerun replace superseded outcomes without duplication.

No new exclusions were added; existing unrelated engine directory-prefix configuration
is unchanged. Timeouts break bootstrap, capture, coordinator or worker progress and are
separate from assertion catches. Unviable replacements fail compilation/build. Thread
creation failure is handled but not forced through OS resource exhaustion in tests.
Optional measurement code/Python, portable source/allocator/dependency and OS/browser
internals are unchanged or outside mutation scope. Evidence is not proof of correctness.

## Limits and next qualification

The i7-13700H host exposes 20 logical CPUs and about 62.5 GiB RAM, with Rust 1.99.1.
No builds, correctness tests, mutation jobs, preparation or other benchmarks compete during
timing. Fact/OS caches are warm; setup and independent verification are outside edit
latency but contribute to process peaks. Publication polling adds roughly 2 ms uncertainty.
Three samples per condition are not latency tails, randomised speedups or browser results.
The 12 GiB child address-space limit is not RSS/physical-RAM enforcement or laptop proof.

Idle means no foreground/recovery work at admission, not global machine or disk inactivity.
Typing can race admission, metadata batches may briefly contend, and full retained capacity
can refuse capture. Generated tails are controlled pressure rather than representative
retirement frequencies. Watcher delivery, active periodic recovery, cold storage, other
platforms and blocked shutdown timing remain unqualified. Quiet/retry/poll constants are
provisional; broader interactive and baseline-hardware evidence is needed before defaults
for source backing or memory policy change.

[Raw observations](results/observations.json), [summary](results/summary.json),
[sample CSV](results/samples.csv), [mutation details](results/native-mutation.json)
and [source hashes](results/source-sha256.json) retain code, binary and workload identity.
