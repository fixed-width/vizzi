# Focused Linux/PyTorch review measurements

The review views remain responsive in these scoped cases: successful projection fetch
p95 is under 4 ms and observed rAF p99 is 16.8 ms during scrolling/refresh. The larger
first-use cost is host startup, particularly PyTorch's initial live-workspace index.
Refresh also requires another file-selection action when it resets away from the edited
file. No product algorithm or resource policy changed for this measurement.

This is the eager-startup cohort at `b116f4b`. The
[startup follow-up](STARTUP_RESULTS.md) records the later deferred host and its separate
measurements; the observations below are preserved as the earlier comparison.

The [method](SCALE_PLAN.md), [raw reports and summary](results/scale/summary.json) preserve
pins, hardware, policy, samples and limits. These are three sequential optimized-host
sessions per case, one shared parser worker, fresh analysis caches and full disposable
working trees/indexes. Filesystem caches are warm. The originals' status digests match
before/after; only the disposable copies receive comment probes and a Markdown note.

| Input | Linux | PyTorch |
| --- | --- | --- |
| Git pin | `cbbddb6b08cc9900a87cccaa9b22bbe06aa4f50a` | `1f4d2d5e820dd870626dca4f9603e3a49879dbde` |
| Full index paths | 97,345 | 22,762 |
| Review scope | `kernel/sched`: 54 baseline paths, 2.25 MB | `torch/_inductor/codegen`: 109 baseline paths, 3.65 MB |
| Changed code | `core.c`, `fair.c` | `wrapper.py`, `aoti_runtime/interface.cpp` |
| Observed analysis | Raw C, partial grammar coverage | Python syntax; raw C++, partial coverage |
| Initial live index files | 1,075 | 5,079 |

The host's default Rust/Python discovery remains active beyond the C-family subtree.
This is why a focused review does not imply a focused initial live-workspace index.
Raw syntax has unknown build configuration; no compiler-backed or cross-language
semantic claim is made.

## Timings

Primary action timing begins at the browser's trusted click, after Playwright's
actionability waits, and ends after matching DOM, automation observation and two rAF
callbacks. Raw command elapsed time is retained separately. These timings include
observer overhead and do not measure physical presentation.

| Observation | Linux | PyTorch | Samples per case |
| --- | ---: | ---: | ---: |
| Host ready, median (range), seconds | 1.84 (1.82–2.01) | 16.50 (16.49–18.80) | 3 |
| First scoped comparison, median (range), ms | 547 (465–564) | 279 (247–280) | 3 |
| File selection p50 / p95, ms | 66.0 / 68.0 | 66.1 / 67.8 | 36 |
| Hunk selection p50 / p95, ms | 65.9 / 67.6 | 65.9 / 67.6 | 36 |
| Declaration selection p50 / p95, ms | 64.3 / 65.6 | 64.5 / 65.6 | 36 |
| Explicit refresh to new view p50 / p95, ms | 266 / 315 | 149 / 181 | 15 |
| Refresh to edited source, including re-selection, p50 / p95, ms | 315 / 448 | 266 / 315 | 15 |
| Successful projection fetch p95 / max, ms | 3.8 / 6.2 | 3.6 / 6.2 | 726 / 738 |

The first comparison follows initial host indexing and can reuse its facts. It is not
cold analysis of both review sides. The larger Linux index coincides with slower scoped
capture, but these cases do not isolate Git discovery, parsing or language-policy costs.

Disk changes leave the old pair/version/text pinned until **Refresh working**. Every
sample publishes a new token and working version while preserving the base version.
Refresh resets to the first changed file: 6/15 Linux and 9/15 PyTorch samples need a
second selection to reveal the edited file. That extra action is included only in
the edited-source row. The 200-ms live ordinary-edit budget does not directly apply
to full scoped comparison refresh or to that additional action.

## Rendering, payload and memory observations

All six sessions finish without page/query errors, capacity refusals or retired-query
conflicts. There are 87/34 scroll inputs submitted while the old comparison token is
observed during Linux/PyTorch refresh; this is observed overlap, not an instrumented
native IO/parse ordering guarantee.

| Observation | Linux | PyTorch |
| --- | ---: | ---: |
| Observed scroll/refresh rAF p99, ms | 16.8 | 16.8 |
| Observed rAF intervals over 33.3 ms | 0 | 0 |
| Main-thread long tasks, with observer available | 0 | 0 |
| Source rows / outline buttons per pane, maximum sampled | 56 / 18 | 56 / 18 |
| File / hunk buttons, maximum sampled | 55 / 1 | 110 / 1 |
| Largest successful projection body, bytes | 57,774 | 85,592 |
| Observed native-host high-water RSS, MiB | 118.8 | 276.6 |
| Renderer heap after interaction, maximum point sample, MiB | 8.0 | 7.4 |

Source/side/version and byte-fragment audits run outside timed groups, before and after
interaction. Sampled DOM stays within the existing 96-source-row/200-outline bounds.
These are phase endpoints/checkpoints, not continuous all-frame completeness checks.
The 32-KiB source-text allowance differs from total JSON body size, which includes
outline and inventory metadata. Successful projection fetches exclude POST-acceptance
timings; those control timings remain in the raw data.

The i7-13700H host has 20 logical CPUs and about 62.5 GiB RAM. Headless Chromium 156
uses SwiftShader; no GPU execution or display-presentation qualification follows.
Native RSS excludes Git children. Renderer heap is not browser RSS. No 16-GB cgroup,
loaded-editor envelope, sustained pressure or whole-repository comparison is qualified.
Small 3/15/36-sample action percentiles remain descriptive rather than timing tails.

## Setup and verification

An initial sparse-checkout pilot stopped before publication: the current adapter
explicitly refuses sparse-checkout capture. The retained
[diagnostic](results/scale/sparse-admission.json) records that refusal. The final cases
use supported full checkouts; sparse support was not added. Full-tree pilot sessions
validated the harness and are excluded from the final timing cohort.

The fixture test first failed against its stub and then against the former partial-tree
preparation expectation. It checks the full index/tree, exact original commit, mixed
comment probes, original dirty working bytes, normalized scopes, refusal to overwrite,
and rejection before creating output inside the original checkout. Five targeted
[fixture mutation controls](results/scale/fixture-mutation.json) are detected; this is
not exhaustive Python mutation coverage. Existing statistics/source audit modules
retain their earlier full-scope mutation evidence.

Native workspace tests (397), the complete Node/Chromium suite and review integration,
fixture tests, formatting, JavaScript syntax and workspace Clippy pass after measurement.
No agent tests or mutation runs overlap the final measurements. External host activity
is uncontrolled; load/hardware are retained in the reports.

## Next decision

The clearest first-use issue is opening a focused review while the host initializes
the full live-workspace index. The current URL becomes available only after initial
publication. A task-driven or progress-first startup slice should be designed against
that observed wait, preserving the shared engine/worker budget and explicit readiness
states. The one-worker data is not a regression comparison with earlier four-worker
profiles, and does not establish a new startup deadline.

Selection continuity across refresh is a second recorded task friction; carrying it
forward must still use fresh pins and never reuse old hunk coordinates. Independent
operator evaluation remains open. Keep the next work tied to those observed user-task
needs and the existing architectural boundaries.
