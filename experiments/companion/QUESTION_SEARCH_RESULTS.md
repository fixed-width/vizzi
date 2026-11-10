# Question-search companion trial results

Historical evidence: this browser workflow is retired. The [archived trial instructions](QUESTION_SEARCH_TRIAL.md)
identify the checkout required for UI reproduction.

For contributors deciding whether the question-first workflow is ready for a maintainer
trial. The opt-in `/investigate?trial=search` view connects a question to grouped source
candidates, captured previews, explicit saved evidence and verified VS Code handoff.
The default repository overview remains unchanged. No model is loaded or needed.

Use [the trial guide](QUESTION_SEARCH_TRIAL.md) to try another repository.
It need not be one of the three reference repositories. Fresh maintainer questions and
independent navigation usefulness remain unevaluated; the eight earlier reserved
real-source cases remain unrun. This slice establishes a working trial, not acceptance
of complete answers or whole-repository scale.

The subsequent [fresh Tract Plugin Pack trial](TRACT_PLUGIN_PACK_FRESH_RESULTS.md)
records the first maintainer-authored pair on another repository and its relevance and
retention failures. Fresh-question status statements above describe this earlier checkpoint.

## Delivered behaviour and architecture

One question input replaces literal name/file search in the trial. Up to eight finite
results are grouped under candidate headings; the combined excerpt allowance is 4,000
UTF-8 bytes. Inspection reuses the captured-source drawer. Keeping a location and keeping
the question are separate explicit actions. The comparison checkbox is disclosed under
**Trial comparison**. Existing saved locations, system/dark/light themes and editor
controls are retained.

Portable indexing/search now lives in `vizzi-search`. The QA runner retains acquisition
and compatibility exports, so the viewer can use the same search without a dependency
cycle. Native search reuses validated captured analysis, rather than reparsing documents
or pinning an entire retired snapshot in its cache. Same bytes with changed analysis
invalidate metadata; unchanged versions can reuse copied search data without reading
backing storage again. Native snapshot/file/version bindings remain distinct from
portable corpus hashes.

Search admits at most 4,096 UTF-8 files, 2 MiB per file and 32 MiB of text in path order.
Omissions are counted. Captured-storage failures fail the request. Updated source makes
results stale; superseded replies cannot replace a newer query. The save endpoint checks
the current observation before acquiring evidence, and the existing editor adapter
verifies working bytes. The [behaviour contract](../../docs/companion/SEARCH_PRODUCT.md)
and [architecture](../../docs/companion/SEARCH_TECH.md) define the guarantees and limits.

## Optimized-build observations

Measured on lpc-rs revision `1a4f5b65dd2a269298748df2dfd68092a169a8d9`. All 1,032
regular files in the prepared source were checked against the pinned archive before the
run. The viewer indexed and searched 552 selected source files with no additional search
omissions. This scope excludes unselected repository documents/languages.

Tests, builds and mutations had finished before measurement. The native host used one
indexing worker at nice 10. Each question/selector has 30 warm localhost/full-body samples
after five warmups; JSON parsing follows the timed request. Browser timings cover 30 warm
form submissions through DOM updates and two animation frames after five warmups.

| Known question | Focused median / p95 | Original median / p95 |
| --- | --- | --- |
| Transaction creation | 473 / 570 ms | 430 / 535 ms |
| Add an efun | 701 / 726 ms | 573 / 656 ms |
| Transaction commit checks | 370 / 400 ms | 296 / 346 ms |

Cold source activation took 2.26 s. The first query, including search-cache preparation,
took 1.50 s. Warm browser submission/rendering had a 498 ms median, 716 ms p95 and 731 ms
maximum, with no page errors. Observed process-tree peak PSS was 0.441 GiB; peak summed
RSS was 0.703 GiB. Memory was sampled every 250 ms in observation mode, without an enforced
cap. Browser/driver/host costs are included; this does not emulate a physical laptop or
qualify hardware frame times, GPU rendering, Linux-kernel/PyTorch scale, or arbitrary
repository workloads.

A post-run audit using the unchanged known-target coverage helper found 6/7 preferred
locators in focused results versus 2/7 in original matching. The normal creation entry
is fifth; the efun invocation/table remains absent. Scope matches can locate a declaration
whose full relevant code is outside the excerpt; these counts do not establish complete
workflow answers. The native indexed-source corpus differs from the earlier QA corpus,
so these numbers do not replace the earlier frozen experiment.

The source-aware functional replay inspected and kept the first two results from distinct
files for the creation question, saved the question explicitly and recorded one editor
handoff. Those locations were `Transaction` and a publication attempt helper, rather than
a human-selected complete creation path. This verifies the interaction and version binding;
it does not establish that the first two results are the best learning evidence. No real
editor window was launched. The owned host and browser stopped successfully.

## Verification and mutation evidence

Formatting, all-target Clippy, 507 workspace tests/doctests, all 20 QA Node test files and
the full headless browser suite pass. Browser checks cover one initial input, candidate
groups, partial scope, explicit saves, recording-only editor arguments, original matching,
late replies, stale-source actions, dismissed errors, empty results and narrow/dark layouts.
Final review also reproduced saving being enabled before initial investigation state loaded,
on search cards and in the shared source drawer. Save controls and handlers now require
that state; trial, default discovery and saved-investigation browser checks passed after
the final DOM-only fixes, including the delayed-response regression.
Native/corpus cases include same-path/same-size edits, supplied-analysis reuse, changed
analysis with unchanged bytes, unavailable storage, current-only saves and exact byte,
file-count and window-count boundaries.

Across the new adapters and affected corpus methods, Rust mutation testing generated 86
variants: 78 were caught by failing tests, two were reviewed as equivalent and six were
unviable build failures. There were no timeouts. The initial pass was interrupted after
55 variants when gaps in boundary tests became clear; completed iterative passes retained
previous catches/build failures and exercised the rest with stronger tests. New tests
cover lexical path/body ranking, length normalization and the 65,536-window limit.

The equivalent variants remove an unused outline-count hint during pinning, and allow an
empty range that supported inputs cannot produce. They are reported as surviving reviewed
equivalents, separately from test catches. The audit gives the invariants. No variant was
excluded from execution; cargo-mutants still generated the outline field deletion when an
exact exclusion was attempted. The remaining six variants did not compile, including
invalid default values and an invalid let-chain operator mutation.

All 249 generated mutations in the browser query-state module were caught, with no
survivors, errors or timeouts. The preparatory 255-variant run exposed 23 survivors; stronger
interface tests and removal of redundant state checks produced the final 249-variant run.
Mutation jobs used one worker/compiler/jobserver task, serial tests and nice 10. This does
not newly qualify every relocated ranking module, the DOM adapter or HTTP routing in full;
those retain previous evidence or integration-test coverage. No mutation score proves
correctness.

Earlier successful profiles preceded the final loading-state corrections; they are retained
as `validation/pre-loading-fix-*` and `validation/pre-drawer-fix-*`. The table and memory figures above use the final binary.
Variation between these runs is not evidence of an improvement or regression from that
small UI change; this is an uncontrolled developer machine.

The first measurement attempt stopped before any query samples because the measurement
helper expected HTTP 200 for the normal HTTP 202 indexing-start response. The helper was
corrected; that rejected attempt is archived separately and contributes no timing/memory
claims above.

## Artifacts

- [Raw request/browser samples and saved replay](results/question-search/report.json),
  [process-tree memory](results/question-search/memory.json.gz), and
  [measurement log](results/question-search/measurement.log.gz).
- [Verified source manifest](results/question-search/source-audit.json),
  [known locator audit](results/question-search/known-locators.json), and
  [captured trial view](results/question-search/known-question.png).
- [Check inventory](results/question-search/validation/checks.json),
  [Rust mutation audit](results/question-search/validation/rust-audit.json), and
  [final browser-state mutations](results/question-search/validation/node-final.json.gz).
  Preparatory/boundary/expanded mutation outcomes and logs are retained alongside them.

The next acceptance step is a frozen set of fresh maintainer questions, preferably on a
repository outside retrieval development, comparing navigation and retained context with
the editor-only workflow. Keep the trial opt-in and inference optional while gathering
that evidence.
