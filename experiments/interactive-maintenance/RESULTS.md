# Watching and recovery preserve publication during automatic cleanup

The combined native configuration completes **18 edits with correct source, fresh facts,
expected declarations and retained baseline pins**. Native watches register successfully;
edits and restoration use no manual hints. All six edits during real cleanup publish
before truncation finishes. This qualification stage changes no production scheduler,
storage implementation, dependency or default.

The [procedure](README.md) reproduces the workload; the
[plan](../../docs/plans/interactive-maintenance.md) defines evidence boundaries. This
extends [isolated idle maintenance](../idle-maintenance/RESULTS.md) to watching and
periodic recovery together, without comparing backend implementations.

## Combined observations

The pinned 66,066-file Linux and 10,578-file PyTorch scopes use four parser workers,
native directory watches, two-second periodic reconciliation and automatic maintenance.
Source backing remains opt-in, with no syntax-retention/input quota. An observed provider
delegates ordinary spool hints and physical cleanup. Only real generated-tail cuts signal
entry; there is no artificial delay or manual cleanup thread.

Three alternating pairs edit after completed cleanup and after real truncate entry,
reversing order for the middle pair. Generated tails contain 4 GiB Linux / 768 MiB PyTorch
of bytes outside project source. Three additional edits follow sampled active recovery.
Targets remain `kernel/locking/mutex.c` and `c10/core/Device.cpp`; unique declarations
avoid reuse of old probe facts.

| Observation | Linux | PyTorch |
| --- | --- | --- |
| After-cleanup publication, median (range) | 54.1 ms (50.3–56.0) | 31.1 ms (31.0–31.2) |
| During-cleanup publication, median (range) | **56.0 ms (53.8–58.1)** | **31.2 ms (31.1–31.2)** |
| Recovery-observed burst publication, median (range) | 57.6 ms (53.8–58.2) | 31.0 ms (31.0–31.1) |
| Actual write overlaps / requested cleanup overlaps | 3 / 3 | 3 / 3 |
| Publications before truncation ends | **3 / 3** | **3 / 3** |
| During-cleanup truncate, median (range) | 406.3 ms (364.0–432.1) | 86.2 ms (86.0–91.3) |
| All large-tail retirement-to-entry observations | **2.49–2.70 s** | 68–621 ms |
| Maintenance attempts / completed / failed at final sample | 9 / 9 / 0 | 8 / 8 / 0 |
| Sampled process peak RSS | 2.835 GiB | 0.507 GiB |
| Coordinator shutdown / callback release observed by harness | 0.80 / 7.11 ms | 0.22 / 6.67 ms |

Foreground publication progresses during real cleanup. Physical cleanup can wait much
longer for admission than in the isolated run: Linux retirement-to-entry is several
seconds here. Admission yields to recovery; this experiment does not divide admission
time into recovery, quiet-time and polling components. It establishes no cleanup deadline
or freedom from starvation during sustained activity. Captures still require headroom.

Cleanup-overlap writes sample recovery inactive before and after publication. Recovery
bursts sample it active before and after all six publications. Each repository's three
burst writes share one observed recovery cycle: completed/discarded counters stay 7/0
Linux and 4/0 PyTorch. They are not three independent cycles. The
[earlier recovery experiment](../recovery/RESULTS.md) separately measured writes following
distinct passes under a different backing configuration.

No sampled foreground/recovery error or maintenance failure appears. Targets restore
original hashes and baseline identity between edits; final complete clean observations
agree. After host, pins and callbacks release, explicit verification cleanup returns
backing length/allocation to zero. Peak backing extents stay within shared 8 / 1 GiB
allowances, with separate equal staging bounds. Original archives remain read-only;
no analysed project tooling runs.

## Attribution and lifecycle

Native watching and periodic recovery can both discover edits. Registration is required
and polling fallback aborts qualification; notification latency is not isolated. Timing
starts before the write and ends when correct source is observed without foreground work
pending, including observation, capture, analysis, publication and roughly 2 ms polling
uncertainty.

Unlabelled capture tracing is disabled because recovery also captures; capture-overlap
flags are null. Latest accepted operation measurements remain diagnostics, even though
`read_files` happens to be one at each sampled publication. They do not identify which
operation first published an edit. Cleanup cases retain legacy `hint_ms` as a post-write
timestamp; `explicit_hint: false` means no hint was sent. Sampled recovery status does not
prove exact parser/capture overlap or simultaneous three-way work.

The first combined test exposed asynchronous native watcher ownership in the harness:
callback release can briefly outlast coordinator join. The probe now waits for its
application ownership to return before final pin/storage checks. `shutdown_ms` records
coordinator joining; `released_ms` also includes observed callback release. These
unblocked observations establish no shutdown deadline. Production host lifecycle and
maintenance joining are unchanged.

## Verification

The initial combined test fails before the mode/report exists, then catches premature
final storage sampling. The final fixture exercises watching without manual hints,
alternating pairs, recovery-observed publication, source/fact/declaration checks, old
pins and restoration. Pure tests cover registration/fallback, isolated-mode compatibility,
real-cut observation, cleanup readiness, new failures, restoration and timing units.

Formatting, **291 Rust tests**, doctests, Clippy and **58 Node/browser checks** pass.
The complete final qualification module and shared timing helper produce **49 mutation
outcomes: 43 caught, six timeouts, zero survivors, zero unviable replacements**. Passing
unmodified baselines precede interpretation. Exploratory survivors prompted tests of
work hints, alternating order, entry versus completion, restoration barriers, failure
counters and submillisecond timing. No new exclusions were added.

Timeouts stall hint submission, cleanup/publication readiness or callback-release
progress and are separate from assertion catches. Production scheduler/storage code is
unchanged; its broader evidence remains in the preceding report. Python drivers,
dependencies, OS/browser internals and unrelated Rust modules are outside this mutation
scope. Evidence is not proof of correctness.

## Limits and next qualification

The i7-13700H host exposes 20 logical CPUs, about 62.5 GiB RAM and Rust 1.99.1. No builds,
tests, mutation jobs, cache preparation or other benchmarks compete during timing.
Fact/OS caches are warm. Setup and independent verification are outside edit timing but
contribute to peaks. Small timing differences are sample variation, without a randomised
comparison or latency-tail claim.

The 12 GiB child address-space limit does not enforce physical RAM or qualify the 16 GiB
laptop baseline. Browser interaction, cold storage, other platforms, sustained retirement,
full-capacity pressure and blocked shutdown remain open. Next, qualify the combined
engine/browser/editor resource envelope under constrained memory before changing defaults.
Longer cleanup admission also motivates sustained-workload qualification; these finite
samples do not justify tuning quiet/recovery constants.

[Raw observations](results/observations.json), [summary](results/summary.json),
[sample CSV](results/samples.csv), [mutation details](results/native-mutation.json)
and [source hashes](results/source-sha256.json) retain code, binary and workload identity.
