# Adjacent function comparison architecture

Read [the behaviour contract](FUNCTION_COMPARISON_PRODUCT.md). Baseline:
`dc92ac40e90e82c01f74482bdf3a229b0ac8a124`.

## Context and changes

FunctionHistory owns single-file Git captures and accepted history metadata. Its window
route can reacquire an accepted historical capture. Workspace separates source bindings
from rectangles; SourcePane owns source layout and literal copying. The portable text-diff
engine currently compares full files through Comparison and already bounds bytes, lines
and work. The active review pair has a separate lifetime and must remain independent.

- Expose a range-aware text-diff interface over two immutable Documents. Validate UTF-8
  byte boundaries and byte/line/work/output limits before publishing a complete edit list.
  Reuse the existing line algorithm; return absolute original-file byte and line ranges.
- Add a child comparison module to FunctionHistory. Compare requests select either an
  exact current binding or an exact accepted historical binding. Obtain its adjacent older
  version through the existing history walker. Retain only the two selected single-file
  captures, their declaration spans, labels and edits; never a whole live repository.
- Comparison capture ownership is separate from both the history cache and review store.
  Bound it to 12 pairs and 32 MiB of logical captured source; each file remains limited to
  2 MiB. Compute bounded candidates before a short store admission, then evict oldest pairs
  as needed. Metadata, transient preparation and parser memory are separate from this
  allowance. Existing native engine/worker admission and HTTP work limits still apply.
- POST `/api/function-comparisons` creates a pair. GET `/api/function-comparisons/window`
  requires pair token, side, snapshot, file and version alongside WindowQuery. Windows
  use the shared bounded retry policy. New IDs are native provenance, not source identities.
  Responses include source frames, revision/history labels and absolute edit ranges.
- Prepare both frames and validate through a planned workspace before changing the model.
  Update the originating view in place and add one distinct representation; comparison
  views can share a symbol with another bubble. Persist optional per-view comparison
  metadata. Closing one side does not invalidate the remaining capture or its highlights.
  A successful live refresh or a new history selection removes that view's comparison
  metadata; unresolved live tracking retains it with the correct paging route.
- Use SourcePane and decorative row classes, leaving literal source untouched. Initial
  frames show the first edit and its declaration-origin row. Coordinated edit navigation
  prepares visible pair members before adopting reading state. Cancellation and selection
  checks guard the commit; paging never requests current call metadata for these captures.
  A new editor delivery invalidates mounted source-preparation tickets, aborts their
  history/comparison controllers and clears only loading state owned by those operations.
  Cancellation and ticket invalidation both apply, so uncancellable late responses cannot
  replace the newer editor navigation or report obsolete errors.
  Edit-navigation failures use the same mounted-record and operation-ticket checks as
  successful preparation. Returning live or closing either side also makes late failures
  obsolete while retaining the current workspace object.

## Validation

Use interface tests for range translation, untouched context, Unicode/CRLF, insertion and
deletion anchors, limits and cancellation. Native Git fixtures cover current/HEAD and
historical adjacency, frozen ownership, exact window pins, expiry and errors (1–8, 10).
Presentation tests cover atomic addition, independent closure/return, task membership,
malformed persistence and unchanged existing geometry (2, 6–9). Real Chromium exercises a
change after line 96, coordinated navigation, copying, reload, live edits and minimum-size
controls (3–10). Run the repository baseline and focused low-concurrency mutations after
a passing unmodified baseline; record outcomes separately. Keep optimized timing separate
from correctness and describe its repository/workload limitations.

## Observed validation, 2026-11-16

The later [complete browser qualification](../development/function-history-validation.md)
records final-PR and merge-commit CI separately from the focused checks below.

Linux checks passed formatting, workspace tests/doctests (769 tests, including 21 native
history/comparison tests and four range-diff tests), Clippy with warnings denied, all 268
browser model tests and nine packaged Chromium workflows. The new workflow verifies two
edits after line 96, original line numbers and copying, coordinated navigation, retry/reload,
independent frozen captures through later commits, historical adjacency, minimum-size
controls and cancellation. Existing history, live workspace, visual workspace, tasks,
dependencies, workspace controls, function margins and reflow workflows also passed.

The full suite exposed a fixture that waited for any replacement publication and could
observe an intermediate file write. It now waits for the exact final saved Rust-file
contents; the passing results above use this deterministic readiness condition.

Scoped cargo-mutants runs covered the complete range adapter, comparison window validation,
capture-size accounting and initial-frame projection. Across 62 unique mutations, 60 were
caught by failing tests, one could not compile, and one equivalent case was excluded. There
were no timeouts. The invalid generated replacement requires TextEdit: Default, which the
type does not implement; this is not counted as caught. Initial survivors led to tests for
the inclusive change-count allowance, every window pin, and complete declaration indentation
on later pages. Their behaviour-changing variants were caught on rerun.

The narrow equivalent case deletes outline_count from the origin-only WindowQuery in
function_history/comparison.rs. Only that query's first source line is returned; its outline
metadata is discarded and does not affect line projection. Deleting line_count is different:
it can clip a long indentation prefix, which the new origin regression catches. No broader
projection or range mutations were excluded as equivalent. Runs used nice 10, one mutant
job, one jobserver task, CARGO_BUILD_JOBS=1 and RUST_TEST_THREADS=1, with passing unmodified
baselines and targeted reruns after strengthening tests. These results cover the selected
native interfaces, not full-repository mutation coverage or browser controller mutations.

A separate optimized observation on Vizzi itself, with 271 captured source files and one
native worker, compared Structure::new with its previous version in 220.22 ms. Subsequent
owned-source reads took 1.53 ms for Before and 1.10 ms for After. Initial indexing is excluded.
This single local observation is not a large-repository latency qualification; the complete
browser suite and Linux/PyTorch history-comparison qualification were not run.

### Newer editor intent regression

A delayed comparison previously survived Explore Call because delivery advanced call
navigation intent without invalidating the comparison ticket. A local Chromium regression
reproduced two comparison bubbles appearing after the newer call had applied. It now checks
normal transport cancellation and late success/failure from a transport that ignores abort,
including the same existing caller, selected destination, camera, live actions, loading
controls and silent cancellation. The regression is included in the default browser runner.

Two targeted browser mutations were caught by failing assertions: removing comparison-ticket
invalidation recreated the stale pair, and removing owned-loading cleanup left the caller
disabled. There were no timeouts, build failures or exclusions in these two checks. Native
behavior did not change, so the native mutation results above were not repeated. Workspace
tests/doctests, Clippy, formatting and all 308 browser model/delivery checks passed; packaged
comparison, history and editor-delivery browser regressions also passed.

### Superseded edit-navigation failures

The comparison-edit catch previously checked only workspace identity, allowing a delayed
non-AbortError to appear after newer source intent within that workspace. Failure reporting
now also requires every original view to remain mounted with its operation ticket intact.

The default Chromium runner includes comparison-edit-cancellation.test.mjs. Its first
regression reproduced an obsolete error after Explore Call; it now checks uncancellable
failures after editor navigation, Return to live and closing either comparison side.
A current-operation failure must still be reported. The fixture compares two edits beyond
line 96, then checks unchanged edit positions, selection, camera and usable remaining
controls after the late failure. An initial unmodified fixture timed out because it waited
for live refresh before releasing the paired source request; it now releases the request
before awaiting live resolution.

After that baseline correction, both targeted browser mutations were caught by failing
assertions: removing the ticket check exposed the editor case, and removing the mounted-view
check exposed independent closure. The final mutation run had no timeouts, build failures
or exclusions. These two checks cover the new failure guard; native mutation results above
remain applicable because native behavior did not change. All 309 model/DOM checks and the
five packaged comparison-edit, comparison-delivery, comparison, captured-call and history
workflows passed. Large-repository qualification and the complete browser suite were not
repeated for this correction.
The final workspace baseline passed 774 tests/doctests, with 15 opt-in checks ignored;
formatting and Clippy with warnings denied also passed.
