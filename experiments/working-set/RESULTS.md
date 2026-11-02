# Retained syntax across mixed working sets

The current maximum-allowance reservation policy can discard useful trees even when
their idle weights fit the quota. In the controlled PyTorch sequence, **512 MiB retention
loses Perfetto before the final edit**, which takes 1,478 ms without tree reuse. At
1 GiB it remains available and the final edit takes 771 ms with reuse. Both enabled
budgets still bypass some three-file batch leases. These observations support
investigating smaller replacement reservations; increasing the default is not established
as a solution.

The [procedure](README.md), [admission explanation](../../docs/explanation/syntax-working-sets.md),
and [current reference](../../docs/prototype/SYNTAX_RETENTION.md) separate measurements,
policy reasoning, and interface semantics. Production admission, its disabled default,
context fingerprints, and host/browser behaviour are unchanged.

## Controlled conditions

Each pinned Linux/PyTorch profile has disabled, 512 MiB and 1 GiB conditions in separate
sequential processes. All use four shared parser workers, warm OS caches, compatible
prepared analysis caches, and one shared retention cache across four targets. The
per-entry allowance remains 256 MiB in both enabled conditions; the count ceiling is
eight. One process per condition does not establish tails or a randomized paired speedup.

Targets match the preceding [recovery report](../recovery/RESULTS.md): the three largest
selected files and picked ordinary file. Linux uses the AMD DCN header, two NBIO headers,
and Python `Documentation/conf.py`. PyTorch uses `perfetto.h`, `perfetto.cc`,
`SobolEngineOpsUtils.cpp`, and `.ci/docker/java/jni.h`.

There are 20 steps per condition: prime/repeat the primary header, touch the ordinary
file and return, edit the other large files serially and return, edit all three large
files together, fully reconcile, make ten ordinary edits, and return to the primary
header. Unique UUID comments force actual parsing. Original archives remain unchanged;
only marked disposable copies are edited and their targets are restored/hash-checked.
The native probe never executes or builds an analysed project.

Linux captures 66,066 analysed files; PyTorch captures 10,578 files, with 10,147 analysed
and 431 CUDA source-only inputs. All six initial captures parse zero files. Every one
of **120 steps matches complete capture**, and **126 changed-file analyses match fresh
parses independently of persisted facts**. Initial full snapshot identities remain
stable across restored conditions within each owned copy.

The i7-13700H reference host has 20 exposed logical CPUs and about 62.5 GiB RAM, using
Rust 1.99.1. No builds, correctness tests, mutation jobs, or competing benchmarks run
during timing. This probe uses the production engine directly, excluding native watches,
periodic host scheduling, transport and browser rendering. It cannot reconstruct the
precise cache history of the earlier browser's 1.34-second Perfetto edit.

## Tree availability and update times

Times below are individual engine wall times, excluding input writing and correctness
verification. Repeated steps have different comments, so they require new analysis.

| Profile / budget | Immediate header repeat | Final header edit | Final tree reused | Three-file batch reused / bypassed leases |
| --- | --- | --- | --- | --- |
| Linux / disabled | 555 ms | 589 ms | No | 0 / disabled |
| Linux / 512 MiB | 139 ms | 156 ms | Yes | 1 / 2 |
| Linux / 1 GiB | 139 ms | 142 ms | Yes | 2 / 1 |
| PyTorch / disabled | 1,804 ms | 1,641 ms | No | 0 / disabled |
| PyTorch / 512 MiB | 555 ms | 1,478 ms | No | 1 / 2 |
| PyTorch / 1 GiB | 600 ms | 771 ms | Yes | 2 / 1 |

With a hot isolated tree, both enabled budgets reuse the primary header after an
ordinary edit. The PyTorch header is already absent after the 512 MiB three-file batch;
it remains absent throughout the ordinary loop and full verification. The final edit
therefore starts cold. Linux retains its primary tree through that batch and ordinary
loop, while other large-file state is lost. Full verification leaves entry accounting
unchanged in every condition; it does not warm missing trees.

The enabled batches have distinct paths, fewer keys than the entry ceiling, and no
other concurrent engine/host. Their bypasses indicate reservation pressure. Endpoint
inspection and aggregate counters do not reveal each worker's interleaving. A bypass
can leave an older idle state in place; batch tree-reuse counts alone do not describe
future availability. One GiB removes observed evictions here but still has a bypass.

## Idle weights and old/new reservations

Observed admission weights include retained source and tree nodes. They are not heap
measurements. Individual-state weights are stable to the displayed precision across
steps; sums below combine those individually observed weights.

| State | Linux | PyTorch |
| --- | --- | --- |
| Primary header | 121.49 MiB | 244.35 MiB |
| Second large file | 85.13 MiB | 81.96 MiB |
| Third large file | 83.42 MiB | 108.19 MiB |
| Ordinary file | 0.38 MiB | 1.66 MiB |
| Four-state total | 290.42 MiB | 436.16 MiB |

The serial sequence isolates two different pressures:

- In PyTorch's 512 MiB condition, admission of the third large file reserves 256 MiB
  while about 327.97 MiB is idle. The 583.97 MiB request exceeds the quota. LRU eviction
  removes the ordinary state and Perfetto. The actual third state is about 108.19 MiB;
  the four resulting states could have been idle together at about 436.16 MiB.
- In Linux's 512 MiB condition, returning to the retained primary header adds a
  256 MiB replacement reservation to 290.42 MiB. Two other entries are evicted. Using
  the observed primary replacement weight instead would total about 411.91 MiB under
  the existing old/new accounting model. This is accounting arithmetic, not a measured
  alternative-policy implementation.

Smaller reservations cannot preserve every working set indefinitely. Updating a
244.35 MiB Perfetto state while retaining all four PyTorch states would require about
680.51 MiB even if the replacement allowance exactly matched the observed new weight.
That exceeds 512 MiB through unavoidable old/new overlap in the current charging model.
A policy must still evict other state, decline admission, or schedule work differently.

For three concurrent replacements at 1 GiB, the current policy requests 768 MiB beyond
idle charges: about 1,058 MiB Linux or 1,204 MiB PyTorch. Those requests explain why
one lease can still be bypassed. Using individually observed replacement weights would
produce smaller accounting totals, but demand prediction, source growth, real allocation,
and scheduling require separate validation before changing the policy.

| Aggregate outcome | Linux 512 / 1 GiB | PyTorch 512 / 1 GiB |
| --- | --- | --- |
| Total evictions | 2 / 0 | 4 / 0 |
| Total bypasses | 2 / 1 | 2 / 1 |
| Peak charged weight | 463.00 / 802.42 MiB | 502.01 / 948.16 MiB |

All recorded charges stay within their configured quota and all leases are released
at step boundaries. A deterministic small fixture additionally demonstrates a tiny
edit evicting a large idle tree solely to reserve its maximum allowance. Per-key
inspection does not change recency, take ownership, or modify this policy.

## Memory and validation limits

Sampled process peak RSS, including independent fresh parses and complete verification,
is about **4.392 / 4.567 / 4.599 GiB** for Linux disabled/512/1 GiB and
**1.357 / 1.299 / 1.298 GiB** for PyTorch. PyTorch's disabled lifetime high-water counter
is slightly higher than its sampled peak (1.359 GiB). Point RSS before verification is
also retained in the raw cases. The shared sampler's phase labels include verification;
its per-phase peaks are not indexing-only measurements.

These are not comparable to production browser-host peaks or a controlled allocation
study. One process per condition, allocator retention, verification work, and sample
resolution prevent inferring a monotonic memory relationship. Retention weights are
neither parser scratch/input caps nor whole-process limits. The 16 GB laptop remains
unqualified, and raising the default is not justified by these runs.

The inspection/probe tests first failed before their interfaces existed. Tests check
non-mutating absence/idle/leased observations, invalidated reservations, unchanged
LRU ordering, disabled retention, mixed Rust/Python batches, input restoration, invalid
configurations, independent facts, and clean snapshots. A batch expectation initially
assumed that two maximum reservations could coexist with existing state in a tiny
quota; the fixture now selects one worker for deterministic complete reuse. Concurrent
pressure is measured separately and remains covered by the cache's concurrency tests.
Formatting, **195 Rust tests**, doctests, Clippy, and **56 Node/browser checks** pass.

Complete production-cache mutation testing has **94 tested: 78 caught, ten unviable,
six timeouts, zero survivors**, with a passing unmodified baseline and no exclusions.
Timeouts break admission/eviction progress; they are separate from assertion catches.
Build failures are also separate. The cache body tested includes the diagnostic method;
the added reservation-pressure fixture passed with the full normal checks afterward.
Grammar/dependency internals, serialization macro bodies, optional probe orchestration,
Python driver, OS delivery, and browser execution are outside this mutation scope.
No dependencies or unsafe-code exceptions were added.

[Raw observations](results/observations.json), [summary](results/summary.json),
[CSV](results/summary.csv), [mutation details](results/native-mutation.json), and
[source hashes](results/source-sha256.json) preserve configurations, original target
hashes, available-state weights, steps, source/binary identity, and verification evidence.
The next admission design should select and validate replacement allowances using
observed demand, retain strict old/new accounting and nonblocking fallback, and test
source growth and mixed working sets. Enforcing parser-input and process limits remains
separate work.
