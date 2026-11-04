# Reading context after review refresh

All 30 measured working refreshes retain the edited file. The earlier startup cohort
needed another file selection after 6/15 Linux and 9/15 PyTorch refreshes; this cohort
needs none. The selected side, path filter and unchanged-context setting survive too.
Unchanged document versions retain independent pane positions, while changed source
uses a fresh declaration identity lookup or file start. Old hunk coordinates and
expired history tokens retain their existing limits.

The [contract](../../docs/prototype/REVIEW_REFRESH.md) and
[API reference](../../docs/prototype/COMPARISON_API.md) describe the implementation.
Native labels carry an opaque baseline-capture origin that survives working refreshes;
an explicit new capture gets a new origin. Inventory pages can independently return
one selected row through the existing index, so an off-page/filtered/now-unchanged
file gets fresh versions without scanning more pages. These remain SCM-independent
navigation concepts over immutable source.

## Scoped observations

The [method](SCALE_PLAN.md), [raw reports and summary](results/refresh/summary.json)
preserve the pins, full disposable trees/indexes, scopes and mixed-language policy from
the [startup cohort](STARTUP_RESULTS.md). Each case has three fresh-host/analysis-cache
sessions, one parser worker and five controlled comment replacements per session.
Filesystem caches are warm. The driver now asserts preservation rather than repairing
a reset by selecting the file again.

| Observation | Linux before | Linux after | PyTorch before | PyTorch after |
| --- | ---: | ---: | ---: | ---: |
| Refreshes requiring file re-selection | 6/15 | 0/15 | 9/15 | 0/15 |
| Refresh to new view p50 / p95, ms | 332 / 381 | 332 / 383 | 180 / 198 | 165 / 199 |
| Refresh to edited source p50 / p95, ms | 399 / 515 | 366 / 416 | 313 / 332 | 199 / 232 |
| Observed rAF p99, ms | 16.8 | 16.8 | 16.8 | 16.8 |
| Host observed high-water RSS, MiB | 83.8 | 83.5 | 55.1 | 53.6 |
| Renderer heap after interaction, maximum point sample, MiB | 8.0 | 11.4 | 7.4 | 10.2 |

Action timing starts at the trusted click and includes matching DOM, automation
observation and rAF waits. Edited-source verification has additional marker/paint
waits even when no further user action is required. The benefit is removal of that
extra action; no isolated parsing or general refresh-speed improvement is claimed.
Finite samples and uncontrolled external activity do not establish timing tails.

Both live indexes stay `not_started` throughout the six sessions. URL availability
remains around 3–4 ms; median spawn-to-first-review is 900 / 681 ms, including browser
setup and form automation. Both original checkouts' before/after status digests match.
Every refresh changes the token and working version while preserving the base version.
Independent source/side/version checks verify 576 visible rows per case.

The 1,772 / 1,623 observed rAF intervals have no interval above 33.3 ms. Long-task
observers are available and record no long tasks. There are no page/query errors or
capacity refusals. Sampled per-pane DOM maxima remain 56 source rows and 18 outline
controls; file-button maxima are 55 / 110, with one hunk control. Projection fetch p95
is 4.9 / 3.1 ms and maximum successful body size is 57,774 / 86,458 bytes. Source-window
requests retain the 96-row/32-KiB and 200-outline bounds.

The renderer heap point samples are higher than the earlier cohort. Context settings
now remain enabled after refresh, so the final view retains the unchanged file list;
GC timing and other session state are uncontrolled. These measurements do not isolate
the cause or qualify a memory envelope. Host RSS excludes Git subprocesses, and renderer
heap is not browser RSS. Headless Chromium 156 uses SwiftShader on the i7-13700H with
about 62.5 GiB RAM; physical presentation, GPU rendering and the 16-GB/editor baseline
remain unqualified.

## Functional and mutation evidence

[Validation provenance](results/refresh/validation.json) records source/binary hashes
and the final checks: 409 all-features Rust workspace tests, doctest checks (currently
no examples), formatting and all-target/all-feature Clippy pass. The complete Node/
Chromium suite passes 177 cases plus startup, review and review-refresh scenarios.
Both frozen review variants and the learning/history replay pass on the same binary;
their [reports](results/refresh/workflows/) preserve source/bounds evidence.

The [native context scenario](results/refresh/context.json) verifies independent
horizontal/vertical pane positions, moved/missing declarations, side/file loss,
new-capture reset and an expanded long-line fragment. The before/unchanged-refresh,
moved-declaration and retained-fragment screenshots are retained beside that report.
Fresh workflow videos/traces remain at the local output paths, with earlier visual
evidence preserved in the existing walkthrough record.

Browser regressions also found and fixed a typed filter being overwritten by an older
render, an expanded fragment shrinking under a regular multi-line byte allowance, and
UTF-8 end rounding omitting the final character. Fragment reuse requires identical
file/version and actual returned byte column; it adds no source to saved history.

| Mutation scope | Cases | Failing-test detections | Timeouts | Unviable | Assessed equivalent survivors |
| --- | ---: | ---: | ---: | ---: | ---: |
| Native application/projection modules and indexed lease lookup | 134 | 102 | 6 | 26 | 0 |
| Complete review controller and window reader | 943 | 915 | 4 | 0 | 24 |
| Affected inspector functions and public methods | 239 | 209 | 28 | 0 | 2 |
| Complete response/visible-source audit | 322 | 264 | 0 | 0 | 58 |

The linked reports retain raw cohorts, baselines, scope/source hashes and individual
survivor assessments: [native](results/refresh/native-mutations.json),
[controller](results/refresh/controller-mutations.json),
[inspector](results/refresh/inspector-mutations.json) and
[audit](results/refresh/audit-mutations.json). Broad module passes precede focused
survivor follow-ups. Production-source rollups combine only identical source and
matching mutations; overlapping cohort counts must not be added. A demonstrated
failing-test catch remains detected if a later longer run times out. Every timeout
still appears separately in its raw cohort.

Equivalent survivors include redundant guards under documented interface invariants,
internal non-hunk sentinels, identical cached-byte extents and assertion diagnostic
text. No exclusions were configured, and survivors are not counted as catches.
Mutation runs use one lower-priority worker; native compilation/jobserver and test
execution are serial. CSS/app glue are browser-tested rather than mutated. These
results are evidence rather than correctness, coverage or usability proofs.

Repeat the [measurement procedure](README.md#measure-focused-linuxpytorch-review-views)
with fresh outputs, then use `summarize-refresh.mjs OUTPUT.json LINUX_RESULTS
PYTORCH_RESULTS`. No tests, mutation jobs or other agent benchmarks overlap this final
cohort. Whole-repository review and independent unfamiliar-user acceptance remain
separate from the delivered guided tasks and focused measurements.
