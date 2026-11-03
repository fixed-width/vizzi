# The engine/browser workload completes inside a 12 GiB memory scope

All four resident/spooled Linux/PyTorch cases pass with verified **12 GiB memory maxima,
zero swap use and zero OOM events**. Matching uncapped controls also pass. Across the
final eight cases, all 80 ordinary recovery-observed edits reach the browser, and final
complete clean captures agree with live observations.

Spooling substantially lowers mapped-process PSS, but **does not lower controller peak
memory in this matrix**. This distinction matters when choosing source-backing policy.
The [procedure](README.md) reproduces the runs; the
[plan](../../docs/plans/memory-envelope.md) records scope. Production defaults and
application dependencies are unchanged.

## Controller-enforced observations

Each condition runs in a fresh user systemd scope. The Rust observer reads actual
memory.max=12884901888 and memory.swap.max=0 before launch and during sampling, validates
sampled descendant membership and compares OOM counters. Every successful case reports
cap_verified=true; all sampled group swap values and memory.events high/max/oom/oom_kill
counters remain zero. The workload fits the budget without exercising its pressure boundary.

| Capped condition | Controller peak | Sampled aggregate PSS peak | Ordinary DOM p50 / max | Large edit DOM |
| --- | --- | --- | --- | --- |
| Linux resident | **7.016 GiB** | 5.130 GiB | 116.7 / 139.2 ms | 753 ms |
| Linux spooled | **7.226 GiB** | 2.306 GiB | 123.7 / 133.2 ms | 900 ms |
| PyTorch resident | **1.919 GiB** | 1.656 GiB | 100.9 / 122.4 ms | 1,694 ms |
| PyTorch spooled | **1.976 GiB** | 1.313 GiB | 99.2 / 108.8 ms | 1,823 ms |

The workload uses four parser workers, native watching, two-second recovery and no syntax
retention/input quota. Each case has three steady/refresh motion pairs, picking, ten
ordinary edits following distinct observed recovery passes, and a large-header opening/edit.
Browser edit timing ends after matching visible source plus two rAF callbacks.

Spooling uses an 8 GiB Linux / 1 GiB PyTorch disk allowance per host with separate equal
staging bounds. The independent clean-verification host uses the same selected policy
and runs alongside the live host. **Two native processes appear at every process-PSS
peak**; these are whole-workload peaks, including verification. Controller and PSS peaks
are separate observations at potentially different instants, not additive components.

PSS apportions mapped shared pages; controller charges include memory beyond those mapped
pages. Shared page ownership also affects attribution. The controller peak is slightly
higher for spooling here, despite its lower PSS. No memory.stat breakdown was collected,
so the difference is not assigned to a particular cache/kernel category. Neither metric
alone establishes the complete physical laptop envelope. See the primary
[procfs](https://docs.kernel.org/filesystems/proc.html) and
[cgroup](https://docs.kernel.org/admin-guide/cgroup-v2.html) accounting references.

## Matching controls and interaction

Both modes use the same browser configuration and release binaries. Condition order is
Linux resident/spooled, then PyTorch spooled/resident; the capped matrix precedes the
uncapped matrix. There is one complete workflow per condition/mode, without randomisation
or a causal estimate of cap overhead.

| Uncapped condition | Sampled aggregate PSS peak | Ordinary DOM p50 / max | Large edit DOM |
| --- | --- | --- | --- |
| Linux resident | 5.130 GiB | 115.9 / 140.8 ms | 768 ms |
| Linux spooled | 2.262 GiB | 124.7 / 135.6 ms | 886 ms |
| PyTorch resident | 2.030 GiB | 102.4 / 111.4 ms | 1,777 ms |
| PyTorch spooled | 1.281 GiB | 97.9 / 114.0 ms | 1,762 ms |

The PyTorch resident peak differs between modes, illustrating why finite sampled peaks
are not stable capacity requirements. Large-file observations also vary: the spooled
capped edit is slower in both profiles, but uncapped PyTorch is similar. These individual
observations do not establish a general slowdown. The Linux target is the DCN register
header; PyTorch uses Perfetto. Retention stays disabled in both policies.

All final steady/refresh rAF p99 and maximum observations are near 16.8 ms. Recorded
Chromium uses SwiftShader; rAF and DOM visibility do not measure GPU completion or
physical presentation. The 66,066 / 10,578-file scopes and original manifest hashes agree
across all policies/modes. All 24 refresh status observations report native watches;
ordinary-edit completed recovery counters increase between writes. Payload audits, visible
source and final clean captures pass. Only disposable copies are edited; original archives
remain unchanged and no analysed project tooling runs.

## Execution boundaries discovered during qualification

Direct managed-shell probes expose cgroups read-only and fail to connect to the user bus.
The approved npm benchmark entry point can reach that manager and successfully creates
scopes. The [capability record](results/capability.json) distinguishes these contexts;
the initial direct failure does not establish that the real runner lacks access.

The first actual capped attempt completes the browser workload but is correctly rejected:
Chromium's main process leaves the measured scope. Its
[upstream scope-registration implementation](https://raw.githubusercontent.com/chromium/chromium/main/components/dbus/xdg/systemd.cc)
registers the browser PID separately; the related
[Chromium issue](https://issues.chromium.org/issues/580525223) discusses the process-tree split.
The envelope route now isolates only the benchmark browser's session D-Bus connection,
preventing this migration. The launcher retains its manager access. A real browser test
checks inherited cgroup membership before and after workspace loading. Ordinary browser
launches preserve their existing desktop integration. The observer's membership guard
remains intact. Preliminary/rejected evidence is retained separately under results/preliminary.

## Sampling and correctness evidence

A separate safe Rust process samples itself and descendants independently of the Node
event loop, including during synchronous clean verification. It records simultaneous
window sums, PIDs/start times, RSS/PSS, sampling duration and vanished/error diagnostics.
It never adds separate process high-water marks. The outer npm/Python launchers are outside
the sampled ancestry; controller charges include the launchers inside each scope.

The final matrices retain **1,966 samples with no sampling/observer errors**. The nominal
250 ms sleep follows each read: median read duration is 18.2–23.5 ms across cases, with a
61.5 ms maximum. Actual cadence is slower and instrumentation can affect timing. Samples
are non-atomic estimates and can miss short spikes, short-lived children or orphaned
processes. Peak samples contain no disappeared member. Controller peak values supplement
these ancestry observations, but are not a whole-machine measurement.

TDD first exposes the missing observer API, then Linux ESRCH during successful child exit.
Vanished children become explicit disappeared observations; permission/other I/O failures
remain errors. Fixtures cover parsing, ancestry, sums, required units, scope boundaries,
escapes, exact limits, OOM counters, incomplete evidence, child failure, exclusive output
and CLI errors. The live rejected run independently exercises the escape guard.

Formatting, **302 Rust tests**, doctests, Clippy and **59 Node/browser checks** pass.
Complete observer/CLI mutation scopes yield **83 outcomes: 76 caught, seven unviable,
zero survivors, zero timeouts**, after passing baselines. One equivalent delimiter-boundary
mutation is narrowly excluded because distinct opening/closing characters cannot occupy
the same byte position. Consolidating the conditional entry point removes an inactive
non-Linux stub mutation. Python/Node orchestration, controller internals, non-Linux execution
and forced OOM behaviour remain outside this mutation evidence. No unsafe code or new
application dependency is introduced.

## Limits and next work

The i7-13700H reference host has 20 logical CPUs and about 62.5 GiB RAM. Fact/OS caches
are warm; builds and preparation finish before timing, with no competing tests, mutation
jobs or benchmarks. The 12 GiB scope leaves a provisional 4 GiB outside the group in a
16 GiB model, but does not emulate a loaded editor, OS, integrated GPU or physical laptop.
Existing shared-page ownership and warm caches affect controller charges.

This qualifies the finite combined workload within the configured scope. It does not
qualify reclamation under sustained pressure: no controller limit event occurs. Next,
exercise sustained edits/retirement at real pressure and record charge breakdowns before
choosing source-backing defaults. Cold storage, retained working sets, physical hardware,
full-capacity capture refusal and hard filesystem/shutdown deadlines remain separate work.

[Combined summary](results/summary.json), [capped evidence](results/capped/summary.json),
[uncapped controls](results/observed/summary.json), [capped sample CSV](results/capped/samples.csv),
[mutation details](results/native-mutation.json) and [source/binary hashes](results/source-sha256.json)
link the results to their inputs. Each mode directory retains compressed raw browser/memory
reports, uncompressed hashes, screenshots, samples and matrix provenance.
