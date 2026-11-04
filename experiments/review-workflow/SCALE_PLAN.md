# Focused review-scale measurement method

This reference defines measurements of the delivered review task. It is
separate from correctness/mutation checks and from full-repository qualification.

| Case | Pinned source | Review subtree | Changed code |
| --- | --- | --- | --- |
| Linux | `cbbddb6b08cc9900a87cccaa9b22bbe06aa4f50a` | `kernel/sched` (54 paths, 2,253,404 bytes) | `core.c`, `fair.c` |
| PyTorch | `1f4d2d5e820dd870626dca4f9603e3a49879dbde` | `torch/_inductor/codegen` (109 paths, 3,645,816 bytes) | `wrapper.py`, `aoti_runtime/interface.cpp` |

Preparation creates a new local shared-object clone with the original commit,
full working tree and Git index. The pilot found that the current capture adapter
explicitly refuses sparse-checkout mode, so the measured fixtures use full checkouts.
The original checkout's status is checked before/after. No fetch, submodule checkout,
project build, hooks, content drivers or analysed-project scripts run. Two unique
comment probes and an untracked Markdown note form the small review. Independent
baseline files and hashes stay outside the workspace.

Linux uses explicit raw C assignment for its subtree, including headers. PyTorch
uses explicit raw C++ assignment for C-family candidates; Python remains Python.
These are syntax views with unknown build configuration, not compiler-backed or
cross-language semantic results.

Use the normal optimized host, one shared parser worker, default resident source,
and disabled syntax retention/input-admission options. Run three sequential fresh-host/
analysis-cache sessions per case. Working-tree checkout and earlier Git reads warm
filesystem caches, so startup is not cold-storage throughput.

Record:

- host startup and first comparison trusted-click-to-matching-DOM plus two rAF callbacks;
- file, hunk and declaration selections on the captured pair;
- working-pane scrolling before/while explicit refresh, rAF intervals and main-thread
  long tasks, and bounded source/outline/file/hunk DOM;
- five comment replacements followed by explicit refresh, preserving the old pair
  until the refresh action and then verifying the new source/token and pinned base;
- successful projection resource timings and body sizes, errors/refusals, coverage,
  host RSS/high-water RSS and point-in-time renderer heap.

The [eager cohort](SCALE_RESULTS.md) waited for a live observation before emitting the
URL. The [task-driven startup contract](../../docs/prototype/STARTUP_PRODUCT.md) changes
the HTTP path. Its repeated cohort records null initial live measurements and explicit
`not_started` readiness, then checks the live index remains dormant after review
interaction/refresh. First-comparison analysis no longer benefits from preceding live
indexing. Also record spawn-to-first-review elapsed time, including browser setup,
navigation/form automation and DOM/paint waits; do not equate URL availability with a
completed review. Preserve both cohorts rather than overwriting the earlier results.

Source/version/content correctness is checked outside timed interaction groups.
The page uses review SSE; it must not poll status. Passive Resource Timing provides
fetch duration/body sizes without instrumenting the production client. Browser capture
listeners mark trusted clicks so Playwright actionability waits are outside primary
latency; keep command elapsed durations separately. Automation observation/paint-wait
overhead remains, so these are instrumented DOM observations. Capture exact
sample counts and raw durations; small percentiles remain descriptive observations.

Compare the delivered bounds to the [provisional budgets](../../docs/performance/BUDGETS.md).
The 50-ms core-map query and 200-ms live ordinary-edit targets do not directly qualify
a full scoped review refresh. Review intentionally stays pinned after disk changes;
measure explicit refresh action-to-DOM separately. Record any misses as findings,
without silently changing the budgets or adding infrastructure first.

No concurrent agent tests/mutation runs/benchmarks during measurement. External host
activity remains uncontrolled and is reported with hardware/load information. Host
RSS excludes Git children; renderer heap is not browser RSS. These focused scopes,
finite samples and a non-baseline host do not establish whole-repository interaction,
tail latency, a 16-GB loaded-editor envelope or unfamiliar-user comprehension.
