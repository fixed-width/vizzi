# Task working-set validation

Contributor reference for caller/reference stacks, named groups and the task shelf,
2026-11-10. The [behaviour contract](../../docs/visual/WORKING_SETS_PRODUCT.md) and
[architecture](../../docs/visual/WORKING_SETS_TECH.md) define the read-only scope.
Editing, refactoring and debugging are excluded.

## Behaviour checked

- Caller stacks retain source context, preview the matched line even deep inside a
  long function, and open a caller with its captured call witness. Scrolling a stack
  scrolls its results rather than moving the canvas.
- Rust semantic references distinguish written calls from function-value uses. Native
  fixtures cover duplicate/outside locations, dependency provenance, name candidates,
  same-name method filtering, paging, result overflow and source changes during a query.
  A live `tract-plugin-pack` check of `LufsMeter::process_sample` returns 11 semantic call
  locations, including the plugin's `process`, with zero outside or omitted locations.
  Its first request takes 11.138 seconds including provider/mirror startup; this is a
  smoke check, not an interaction-performance benchmark.
- Task groups move and collapse source bubbles and result stacks together. Membership
  changes, closure, group bounds and opening a caller are validated through model interfaces.
- Tasks preserve independent layouts, names, groups, query bindings and reading/camera
  state. Browser checks cover reload, collapsed stacks, old-layout migration, storage
  failure and delayed query replies after switching tasks.

Regression tests first reproduced invalid persistence after collapsing a result stack,
previews that omitted a deep reference, and scrolling results that moved the canvas.
Each passes after its fix. Task switching retains the working layout if storage fails.

## Correctness checks

Formatting, workspace Clippy with warnings denied, the full native suite and full browser
suite pass. The native workspace run contains 651 passing tests, seven opt-in tests ignored,
and 92 successful result summaries including doctests. The new real rust-analyzer reference
test also passes explicitly. Focused native/model tests were strengthened after that workspace
run; their passing baselines are retained separately. No source editing or desktop editor
launch is involved in these checks.

The [compressed evidence](results/working-sets/validation) and
[mutation summary](results/working-sets/mutations.json) retain the outcomes. Mutation runs use
one worker/compiler task, serial tests and lower priority. Unmodified baselines pass first.

## Mutation evidence and limits

The native reference-host scope covers all 90 generated cases in `references.rs`. Its initial
full-file run and survivor follow-ups combine by mutation identity: **68 caught, 15 survived,
seven unviable/build failures, zero timeouts**. The final cache follow-up verifies that opening
another query preserves earlier query pages. This is cumulative evidence, not a fresh full-file
run with the final tests.

The focused core scope has 13 cases: **seven caught and six unviable**, with no remaining
survivor. The initially surviving availability-count field mutation is caught by the existing
module-identity tests; the source-position test alone did not exercise that count.

The final JavaScript run has **722 cases: 588 assertion kills, 134 survivors, zero timeouts
and zero runner errors**, covering complete `task-workspace.js` and `task-shelf.js` files
with the model tests. Earlier surviving group geometry, caller
opening, membership, persistence bounds and task-isolation cases led to stronger tests.
DOM event/rendering adapters and the Rust LSP adapter are exercised by browser, protocol and
real-provider checks rather than exhaustive mutation testing.

Remaining native survivors include semantic overrides of syntax bindings, external-target
candidate counts, long-line column/window budgets, cache rebuilding between observations,
the exact provider-result limit and unavailable resident text. JavaScript survivors include
placement/fit arithmetic, exact byte/identity boundaries, validation guards and redundant
checks. They remain coverage limits; no source exclusions suppress them and they are not
claimed equivalent or counted as test kills. Build failures are separate from assertion kills.

These checks establish fixture behaviour, not usability on large working sets. Semantic
references cover captured repository source and already admitted dependencies; other languages
currently receive written-call name candidates. The browser-local shelf is bounded and does
not provide cross-device persistence. A maintainer trial remains the next usability evidence.

## Reload regression follow-up

Module outlines still assumed every canvas item had a source symbol. Enabling the saved
outline preference with a result stack or collapsed group reproduced the reported
`Cannot read properties of undefined (reading 'module')` exception. The worker adapter
now admits these as neutral obstacles, including when matching a worker reply. Unknown
and legacy source-module metadata also remain valid. The failing reproduction, passing
checks and mutation reports are retained in [reload evidence](results/working-sets/reload-fix).

The browser regression now enables outlines throughout task switching, group collapse
and reload. It also replays unchanged pending/completed status notifications: caller
results remain enabled and the stream stays connected. A subsequent real file edit
still invalidates results. The message now says **Repository updated**, since reference
results depend on the whole repository rather than just the displayed function.

A 20-second observation of the live host recorded 21 events and ten pending checks,
all with the same snapshot identity. Background checks alone did not change source.
The full native/browser suites, formatting and Clippy pass after the production fix.

The full outline-adapter mutation run caught 76 of 134 cases, with 58 survivors and no
timeouts or runner errors. Additional unknown-module fixtures then caught all seven
optional-access mutations, including removal of each of the three new symbol guards.
That is a focused follow-up, not a fresh full-file run. The remaining broad-run gaps
include worker error/timeout cleanup, default worker construction, frame scheduling and
SVG details; they are not claimed equivalent or counted as assertion kills.
