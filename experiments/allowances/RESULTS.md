# Replacement allowances from retained-state observations

Known syntax state now requests its observed weight plus bounded growth headroom,
rather than the entry maximum. In the 1 GiB conditions, **both three-file batches reuse
all three trees with zero bypasses**, compared with two reuses and one bypass in the
fixed-reservation run. Observed batch wall time changes from 452 to 180 ms Linux and
1,712 to 675 ms PyTorch. Old/new charge remains within the configured quota.

This improves reservation efficiency. PyTorch's 512 MiB condition still loses Perfetto
and ends with a cold 1,503 ms header edit. Unknown state still requests the entry maximum,
and true old/new overlap remains. Retention stays opt-in and disabled by default.

## Policy and correctness

The [reference](../../docs/prototype/SYNTAX_RETENTION.md) defines the policy:

- Absent state requests the entry maximum, preserving first-attempt admission.
- Retained state of weight `W` requests `min(entry maximum, W + max(W / 4, 4,096))`.
- The lease charges the entire old weight plus its selected replacement allowance.
  Active reservations remain unevictable; idle eviction and immediate bypass use that
  selected allowance. Returned state must fit it and match content and complete context.
- Growth beyond the allowance can complete analysis without retaining new state. The
  consumed old lease is released; the next targeted attempt can request the full maximum.
  The current attempt does not parse twice to warm a tree.

The 25% or 4 KiB margin is provisional engineering headroom. It is not a measured syntax
growth bound, demand predictor or allocation limit. The cache uses only state it already
owns, with no extra history. Its public interface, entry/count maxima, worker ownership,
source verification, core analyser contexts, dependencies and fact-cache validity are
unchanged. The [delivery plan](../../docs/plans/replacement-allowances.md) records the
invariants and validation criteria.

Two tests first failed on fixed maximum allowances. The implementation then passed tests
for simultaneous replacement reservations, small-state headroom, new-key maxima, exact
limits, an oversized returned state below the entry maximum, invalidation, and exact
charge release. A four-language engine test grows Rust/Python/C/C++ inputs beyond their
selected allowance, checks complete facts without retained trees, reprimes on the next
attempt, and verifies later reuse. Every stage matches fresh analysis and full capture;
old published source remains immutable. Existing cancellation, failure, concurrency,
context/content and quota coverage remains passing.

## Controlled repository follow-up

The [procedure](README.md) repeats the unchanged 20-step
[working-set driver](../working-set/measure.py), selected files and pinned scopes from
the [fixed-reservation investigation](../working-set/RESULTS.md). Each profile has
separate disabled/512 MiB/1 GiB processes using four parser workers, warm OS caches,
prepared compatible fact caches, marked disposable copies, and unique UUID comments.
The entry maximum remains 256 MiB in both enabled conditions and the count ceiling is
eight. Linux includes 66,066 analysed files; PyTorch includes 10,578 files, with 10,147
analysed and 431 CUDA source-only inputs. All six initial captures parse zero files.

All **120 complete-capture comparisons** and **126 independent changed-file analyses**
agree. Original target hashes match the earlier run; copied targets are restored and
baseline snapshot identities remain stable across conditions within each owned copy.
No analysed project is built or executed.

The host remains the i7-13700H reference machine with 20 logical CPUs, about 62.5 GiB
RAM, and Rust 1.99.1. Timing runs have no competing builds, correctness tests, mutation
jobs or benchmarks. They use the production engine directly, excluding watcher/recovery
scheduling, transport and browser-visible latency. Verification is outside update timing.

| Profile / budget | Fixed batch reuses / bypasses | Selected batch reuses / bypasses | Fixed / selected batch wall time |
| --- | --- | --- | --- |
| Linux / 512 MiB | 1 / 2 | 2 / 1 | 468 / 487 ms |
| Linux / 1 GiB | 2 / 1 | 3 / 0 | 452 / 180 ms |
| PyTorch / 512 MiB | 1 / 2 | 1 / 1 | 1,890 / 1,955 ms |
| PyTorch / 1 GiB | 2 / 1 | 3 / 0 | 1,712 / 675 ms |

Linux's return to its primary header after serial large-file edits no longer evicts
two other entries at 512 MiB. Later batch pressure still evicts two entries and bypasses
one lease. In PyTorch's 512 MiB case, unseen third-file admission still evicts Perfetto
before the serial return, and batch pressure loses it again. The smaller reservations
reduce bypasses, while that condition retains the same four total evictions.

| Profile / budget | Fixed / selected peak admission charge | Fixed / selected final header time | Selected final tree reused |
| --- | --- | --- | --- |
| Linux / 512 MiB | 463.00 / 464.89 MiB | 156 / 169 ms | Yes |
| Linux / 1 GiB | 802.42 / 652.97 MiB | 142 / 121 ms | Yes |
| PyTorch / 512 MiB | 502.01 / 502.01 MiB | 1,478 / 1,503 ms | No |
| PyTorch / 1 GiB | 948.16 / 929.86 MiB | 771 / 593 ms | Yes |

Charges stay within their quotas and no leases remain at step boundaries. Reconciliation
leaves entry availability unchanged. Batch endpoint observations do not reveal worker
interleavings; a quota bypass can preserve an older idle state, and a new key can later
evict another state to obtain its full initial allowance. The lower bypass count alone
does not establish a universal latency improvement.

These comparisons use one sequential process per profile/budget/policy, with earlier
fixed-policy observations and different UUID comments. They are not randomized paired
speedups or tail estimates. Disabled controls also vary: the final Linux header is
716 ms here versus 589 ms earlier; PyTorch is 1,443 versus 1,641 ms. That variability
limits timing attribution. The repeatable accounting rules and observed reuse/bypass
changes provide the stronger evidence for this policy change.

## Memory and remaining limits

Sampled process peak RSS including verification is **4.375 / 4.581 / 4.577 GiB** for
Linux disabled/512 MiB/1 GiB and **1.228 / 1.402 / 1.165 GiB** for PyTorch. Process peaks
include independent fresh parses and full captures; point RSS before each verification
is retained separately. The shared sampler's phase labels do not isolate verification.
Allocator retention and single-run variation prevent inferring a monotonic relationship
between quota, admission charge and actual memory.

Old/new retained-state accounting is enforced. Ordinary input buffers, parser scratch
space, transient over-limit trees, immutable observations and allocator retention remain
outside it. In particular, the observed four-state PyTorch working set weighs about
436 MiB, and retaining it through a 244 MiB primary replacement exceeds 512 MiB even
with exact replacement demand. This policy does not remove that requirement or justify
increasing the default. Whole-process enforcement, growth-margin calibration, first-use
admission and 16 GB baseline-laptop qualification remain open.

Formatting, **200 Rust tests**, doctests, Clippy and **56 Node/browser checks** pass.
Complete production-cache mutation testing has a passing baseline and **98 tested:
82 caught, ten unviable, six timeouts, zero survivors**, with no exclusions. Timeouts
break admission/eviction progress and are separate from assertion catches; unviable
substitutions are build failures. Grammar/dependency internals, serialization macro
bodies, optional benchmark orchestration, OS delivery and browser execution are outside
this native mutation scope. No dependencies or unsafe-code exceptions were introduced.

[Raw observations](results/observations.json), [comparison summary](results/summary.json),
[CSV](results/summary.csv), [mutation details](results/native-mutation.json), and
[source hashes](results/source-sha256.json) retain the measured working tree before
commit, source/binary identities, original inputs, verification and non-caught outcomes.
Further calibration should include structural edits and first-use working sets while
keeping existing fallback and immutable-publication guarantees.
