# Review startup without a live index

Focused review can now open without waiting for whole-workspace analysis. In three
fresh-host sessions per case, HTTP URL availability falls from a median 1.84 seconds
to 4.7 ms for Linux and from 16.50 seconds to 4.1 ms for PyTorch. The first usable
review arrives about 1.03 / 0.81 seconds after process launch, including browser setup
and scripted navigation. All six sessions keep live readiness `not_started`, with
null snapshot/measurements, throughout review interaction and explicit refresh.

The [behaviour](../../docs/prototype/STARTUP_PRODUCT.md) and
[implementation contract](../../docs/prototype/STARTUP_TECH.md) define this lifecycle.
Exploration activates the same engine explicitly; initial failure and retry are visible.
Live watching/reconciliation starts after activation and continues for the host lifetime.
The existing eager native constructors, stdio and one-shot defaults remain available.
Opening exploration still incurs full live indexing; no partial observation or faster
whole-repository parser is claimed.

## Comparable scopes, different first-use paths

The [method](SCALE_PLAN.md) and [raw reports/summary](results/startup/summary.json)
preserve the same pins, full working trees/indexes, review scopes and language policy
as the [earlier eager cohort](SCALE_RESULTS.md). Each uses one aggregate parser worker,
fresh analysis caches, resident source and disabled syntax retention/input admission.
Filesystem caches are warm, and external host activity is uncontrolled. Linux reviews
54 baseline paths in `kernel/sched`; PyTorch reviews 109 in `torch/_inductor/codegen`.

| Observation | Linux eager | Linux deferred | PyTorch eager | PyTorch deferred |
| --- | ---: | ---: | ---: | ---: |
| URL available, median, ms | 1,844 | 4.7 | 16,505 | 4.1 |
| URL available, range, ms | 1,820–2,008 | 4.6–5.3 | 16,490–18,799 | 3.7–5.0 |
| First comparison after click, median, ms | 547 | 897 | 279 | 680 |
| First comparison after click, range, ms | 465–564 | 849–913 | 247–280 | 648–698 |
| Spawn to usable review, median (range), ms | Not recorded | 1,028 (961–1,074) | Not recorded | 811 (758–881) |
| Native host observed high-water RSS, MiB | 118.8 | 83.8 | 276.6 | 55.1 |

URL availability ends at the host's emitted URL. Spawn-to-review also includes browser
context creation, navigation, form input and matching DOM plus two rAF callbacks.
Comparison action timing starts at the trusted click, excluding automation actionability
waits; raw command elapsed time remains in the reports. None measures physical presentation.

The deferred comparison loses facts/cache warmup from the preceding live index, so its
first comparison is slower in isolation. These are different task paths on the same
host, not an isolated parser-throughput experiment. Memory also reflects the absence
of the live index; it is not a new process-memory cap. Initial live file counts of
1,075 / 5,079 in the eager cohort are absent observations here, rather than zero-file
completed repositories.

## Interaction after capture

| Deferred observation | Linux | PyTorch | Samples per case |
| --- | ---: | ---: | ---: |
| File selection p50 / p95, ms | 66.4 / 67.0 | 66.2 / 69.0 | 36 |
| Hunk selection p50 / p95, ms | 66.3 / 67.9 | 66.2 / 67.9 | 36 |
| Declaration selection p50 / p95, ms | 64.6 / 80.7 | 64.3 / 65.4 | 36 |
| Refresh to new view p50 / p95, ms | 332 / 381 | 180 / 198 | 15 |
| Refresh to edited source, including re-selection, p50 / p95, ms | 399 / 515 | 313 / 332 | 15 |
| Successful projection fetch p95 / max, ms | 3.9 / 8.9 | 3.8 / 6.1 | 726 / 738 |
| Observed rAF p99 / max, ms | 16.8 / 16.8 | 16.8 / 16.8 | 1,818 / 1,685 intervals |

There are no sampled rAF intervals above 33.3 ms, main-thread long tasks, page/query
errors or capacity refusals. Source/side/version audits verify 576 visible rows per
case against independent baseline/working bytes. Every refresh changes the token and
working version while preserving the baseline. Both original checkouts' status digests
match before/after.

Sampled source/outline DOM maxima remain 56 rows / 18 controls per pane, within existing
96/200 bounds. File-button maxima are 55 / 110, with one hunk control; largest successful
projection bodies are 57,774 / 85,592 bytes. Renderer heap point samples peak at 8.0 /
7.4 MiB. RSS excludes Git children, and renderer heap is not browser RSS.

Refresh still resets file selection: 6/15 Linux and 9/15 PyTorch samples need another
selection to reveal the edited source. Refresh is slower than in the earlier cohort
(medians 266 / 149 ms); no refresh-speed improvement is claimed. Preserving useful
location across explicit refresh is the remaining measured workflow friction.

## Correctness and mutation evidence

[Validation provenance](results/startup/validation.json) records source hashes and checks:

- 407 all-features workspace tests pass, with doctest checks (currently no examples),
  formatting and all-target/all-feature Clippy with warnings denied.
- 144 Node/Chromium cases plus complete startup/review scenarios pass. Inspector-only
  fixtures explicitly activate and wait for SSE publication; this also replaces an
  existing async polling predicate whose Promise the installed Playwright poller treated
  as truthy before its boolean result resolved.
- Both frozen review variants and the learning/history replay pass on the measured
  binary. Their [reports](results/startup/workflows/) retain source/bounds checks; fresh
  screenshots/recordings remain at the local output paths. Earlier visual evidence is
  preserved in [the rendered walkthrough](WALKTHROUGH_RESULTS.md).
- Controlled native tests cover held initial parsing, passive metadata, duplicate and
  retry activation, full initial discovery after a path hint, native watch registration,
  independent scoped review, and shutdown in dormant/initializing/cancelled states.

The [full native module pass](results/startup/native-mutations.json) examines host,
application and HTTP server: 54 caught by failing tests, 54 unviable, 12 timeouts and
7 survivors across 127 generated cases. An expanded integration follow-up catches six
survivors. The remaining allocation-hint mutation changes scheduling within the same
enforced aggregate cap and source/publication semantics; its narrow
[functional exclusion](startup-mutants.toml) makes no performance-equivalence claim.
Recorded cohorts include that survivor; the exclusion was not applied retroactively.
Their overlapping counts must not be added.

The [complete browser-module pass](results/startup/browser-mutations.json) covers
startup, request control, notifications and source descriptions: 477 failing-test
catches, 23 timeouts and no survivors across 500 cases. Both baselines and a known-mutant
activation control pass. Timeouts are separate from assertion catches. Native runs use
one lower-priority mutant job, one compiler/jobserver task and serial tests; browser
mutation commands are serial. App orchestration, CSS and benchmark harnesses are outside
these mutation scopes. These results are evidence, not correctness or coverage proofs.

## Reproduction and limits

Repeat the [focused measurement commands](README.md#measure-focused-linuxpytorch-review-views)
with new outputs. Aggregate the new three-session reports with:

```sh
node_modules/node/bin/node experiments/review-workflow/summarize-startup.mjs \
  /tmp/startup-summary.json /tmp/linux-results /tmp/pytorch-results
```

The aggregator checks pins/scopes against the recorded eager cohort, complete sessions,
dormant live status, original-checkout preservation and a shared measured binary. Its
duration-summary `over33ms` fields are arithmetic counts; only rAF summaries compare
frame cadence. No tests, mutations or other agent benchmarks overlap final collection.

The host remains the i7-13700H with about 62.5 GiB RAM; Chromium 156 uses SwiftShader.
Finite scoped samples do not qualify timing tails, full-repository review, physical
presentation, GPU rendering, a 16-GB/editor envelope or unfamiliar-user comprehension.
Once exploration activates, its work and review share the configured budget without
new preemption. Independent-operator usability remains a separate acceptance step.
