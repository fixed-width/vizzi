# Function-history participant observations

Formative maintainer trial for [issue #6](https://github.com/fixed-width/vizzi/issues/6).
The participant completed the learning and review attempts and the remaining reading
probe. Opening a called function preserved the original comparison panel's reading
position. The [preparation record](PREPARATION.md) describes the handoff and readiness
checks.

## H01: the supplied qualified name returns no functions

The participant reported: “Task::run_entry has 0 matching functions”. This is a participant
report of an entry-search obstacle; there is no screen recording or direct observation
of the action time, exact typing sequence or other attempts.

A separate, read-only reproduction against the running trial host confirmed:

| Search | Matching functions | Returned declaration |
| --- | ---: | --- |
| `Task::run_entry` | 0 | None |
| `run_entry` | 1 | `Task<STACKSIZE>::run_entry` in `src/interpreter/task/mod.rs` |
| `Task<STACKSIZE>::run_entry` | 1 | Same declaration |

The host was source-ready at snapshot
`93de03228a63eba9af7101f9a37568b627d765a1d3b35d523305618dc72705cd`,
with 529 captured files, on the pinned task checkout and prepared executable. The
production `/api/structure` inventory request used `functions_only: true`, an empty
area and offset zero. Full responses remain in the private `facilitator/search-incident.json`
session artifact. This API check is source-informed reproduction, separate from the
participant's report.

The lexical search retains the generic parameter list in the parent name and cannot
match `Task::run_entry` across it. The declaration is admitted to the inventory and
reachable by its simple name. The readiness route used that simple name, so it missed
the mismatch with the spelling supplied in the participant brief.

The facilitator supplied **search `run_entry` and choose `src/interpreter/task/mod.rs`**,
and explained the displayed generic-qualified name. Record this as navigation assistance
after an initial unaided search failure. The participant later read and cited the target
function. The exact recovery action is unknown.

## H02: section breaks interrupt a single function

During the learning task the participant paged back and forth to understand code that
continued beyond the visible section. They described the interruptions as awkward and
questioned why a single function needed separate sections.

A separate Chromium replay on the pinned source rendered the 200-line declaration in
three ranges: 442–537, 538–633 and 634–641. The first break separates a call from its error
handling in the same expression. The second cuts through a multiline condition. Each
navigation replaces the previous source lines in the bubble. This confirmed the breaks
and page replacement. The effect on reading comes from the participant's report.

The outcome to pursue is a continuous scroll surface within a declaration, with source
loading and rendering still bounded. Test that the reader can cross a load boundary
mid-expression while retaining line numbers, call actions and reading position. Follow-up
is tracked in [issue #15](https://github.com/fixed-width/vizzi/issues/15).

## H03: the newest historical version looks like the end of the route

For review, the participant stepped through earlier versions and reported being stopped
at `873e71114814` by a disabled **Next change** button. The newer source needed for the
comparison was still available as live source. The review answer was deferred at this point.

A separate replay confirmed that stepping back twice and forward once returns the bubble
to history index zero at `873e71114814`. The commit label tooltip showed the frozen tip
`457446c870ca`. **Next change** was disabled and **Return to live** was enabled, as specified
by the [history contract](../../../docs/visual/HISTORY_PRODUCT.md). **Return to live → Compare**
opened the requested pair while preserving the bubble rectangle. Playback evidence for both
H02 and H03 is in the private `facilitator/reading-obstacles.json` session artifact.

The facilitator supplied that route after the interruption. Record it as review navigation
assistance. The participant later used Compare. The steps they took before opening it
remain unknown. The outcome to pursue is a clearer distinction between the newest
historical capture and live source, with a visible onward action where forward history
stops. Follow-up is tracked in
[issue #16](https://github.com/fixed-width/vizzi/issues/16).

## H04: the review base has no distinct function-history entry

The participant reported that `2a6a23ad1be7` could not be reached by navigating back
through changes. The task brief named this exact Git commit as the review base. A local
Git check confirmed that `2a6a23ad1be7` and the visible `873e71114814` reference the
same blob for `src/interpreter/task/mod.rs`. Function history offers distinct declaration
versions, so it represents the base's source at `873e71114814`.

The facilitator identified `873e71114814` as the **Before** side of the comparison
from the live task checkout. Record this as further navigation assistance. The earlier
replay verified that pair. The participant reported using Before/After to see the diff.

The brief gave an exact Git range without explaining how the function comparison labels
its sources. For a future brief, state that the base commit and displayed Before label
can differ when the declaration is unchanged. Exact revision selection for a function
bubble is an existing feature request. [Issue #8](https://github.com/fixed-width/vizzi/issues/8)

## H05: a caller result opens an extra view of a known function

After the review, the participant tried to get back to `run_entry` by using **Find callers**
from an `efun_of` bubble. Selecting the result opened another `run_entry` bubble. They
expected to reuse their earlier view and clarified that they knew where it was. The
exact revision badges visible in their workspace were not reported.

An independent Chromium replay confirmed the distinction. The `run_entry` result in the
`efun_of` caller stack points to the call at `src/interpreter/task/mod.rs:595`. Selecting
the result reused an existing live `run_entry` bubble. With that original bubble pinned
as After in a comparison, selecting the same result opened a separate live view and
retained the comparison bubbles. Selecting the original bubble directly preserved its
scroll position. The replay is recorded in the private `facilitator/caller-return.json`
session artifact.

Caller results should make it clear when selection will add a live bubble for a function
already open in a comparison. Consider offering the existing comparison view as a
separate choice when the result refers to that function. Keep the source and relationship
evidence tied to the revision that produced it. Follow-up is tracked in
[issue #18](https://github.com/fixed-width/vizzi/issues/18).
`efun_of` is in another file, so this route does not exercise following a same-file
captured call or following a cycle within one capture.

## H06: Return to live briefly shows an unavailable-source warning

The participant reported that **Return to live** reached the correct live version. A
warning appeared and disappeared before they could read or capture it.

A separate Chromium replay captured the workspace banner beginning “Some functions
cannot be followed in current source.” It appeared while the bubble footer said
“Returning to live…” and disappeared 66 ms later when the footer changed to “Live”.
The workspace warning counts a bubble that is still resolving as unavailable. The replay
provides a plausible explanation for the flash. The participant's exact message is
unknown. The replay output is in `facilitator/return-live-warning.json`.

Show the pending state while the current declaration is resolving. Reserve the
unavailable-source banner for a completed lookup that cannot resolve a declaration.
Follow-up is tracked in [issue #17](https://github.com/fixed-width/vizzi/issues/17).

## H07: comparison controls and the reading jump are unclear

The participant reported that **Compare** opened Before, updated the original panel to
show the diff, and scrolled it to the top of the displayed source. Where the revision
controls had been, the two edit buttons and **History** appeared. Their actions were
unclear to the participant. This report concerns entering comparison mode, before
following a captured call.

In a separate Chromium replay, Compare replaced the original `run_entry` panel's
loaded section starting at line 442 with one starting at line 514, just before the
first change. Its scroll offset reset from 190 px to zero. **◂ edit** and **edit ▸**
advanced both sides between `1/2` and `2/2`. **History** was available on committed
sides, including After when comparing from history. It advanced the selected panel
to an older function version and removed its comparison presentation. These behaviours
match the [comparison contract](../../../docs/visual/FUNCTION_COMPARISON_PRODUCT.md).
The replay is recorded in `facilitator/comparison-controls.json`.

Make the comparison controls describe their effects on visible code and on the selected
version. Make the first-change jump legible as part of entering comparison mode.
Follow-up is tracked in [issue #19](https://github.com/fixed-width/vizzi/issues/19).

## Captured call and reading position

In the guided probe after the review, the participant opened a called function from
After and reported that the reading position in After stayed the same. After uses the
same-file captured source for call destinations. The participant did not name the call
or report whether a chooser appeared. An independent replay had already opened
`runtime_error` in that capture and retained the original `run_entry` panel. The
participant's report supplies the reading-position observation. Entering Compare
earlier in the route moved to the first change, as recorded in H07.

## Learning answer

The participant used line references to explain how `run_entry` selects a callback and
what can interrupt execution. Their answer identified the owner and target liveness
checks, the simul-efun lookup errors and the two frame paths. It distinguished some
destination choices and left others open. The frozen rubric assessment and full answer
are in the private session record.

The participant found only test callers of `begin_attempt` and wondered what the efun
reference check does. A source check located a production call through the generic
`AttemptBody` trait in a different file. The participant's search method and any editor
switches are unknown, so the report does not establish a references or editor defect.

## Review answer

The participant identified the change as restoring `this_player` for scheduled callbacks
when `CALL_OUT_PRESERVE_PLAYER` is enabled. They found the change acceptable overall and
would ask why the callback also clears `entry_player`. This was an assisted review after
the learning task. The verdict identifies the intended change and a specific line to
question. The participant supplied no Before/After citations or retry evidence in the
answer. In the debrief, they confirmed using the Before/After comparison to see the
diff, then looked up `entry_player` outside Vizzi to read its definition and commentary.

The facilitator traced the open question to `this_interactive`, which reads `entry_player`
to identify the connected command giver who initiated the task. The call-out contract
specifies an empty `this_interactive()` even when a connected scheduling player is restored
to `this_player`. A test in the inspected commit checks that distinction. The normal
scheduler also constructs the callback with empty player context, leaving a legitimate
review question about which path needs the explicit clear in `run_entry`. The contract,
test and scheduler evidence come from the facilitator's cross-file source check after
the verdict. The participant reported the purpose of their external lookup without
identifying its tool or source files.

## Decisions and limits

The search defect is tracked separately in
[issue #14](https://github.com/fixed-width/vizzi/issues/14), published after explicit
participant approval. The report requests generic-argument-independent qualified lookup
while retaining full labels, source identities and candidate ambiguity. Future readiness
checks should include the declaration spelling supplied to the reader. The participant also
approved publication of the separate paging and history navigation reports.

Retain the spatial call trail and reading-position behaviour observed in the captured
call probe. Prioritise the reported lookup, continuous reading and navigation friction
before expanding the feature. Keep opening live source distinct from revealing a
retained comparison and clarify which action a caller result takes. Make comparison
edit navigation and the transition to older history legible.

The `entry_player` question gives a concrete reason to inspect a field definition and
commentary outside the selected function. The participant did that lookup outside Vizzi.
Use it to inform later cross-file reading design. The trial recorded no further details
about their external tool, chooser use or lineage needs. There was a single maintainer
participant with navigation assistance and no editor-only baseline. Comparative benefit
remains unmeasured.

The maintainer trial is complete and issue #6 is closed. The final summary records the
participant outcome and links the six follow-ups. [Final findings on #6](https://github.com/fixed-width/vizzi/issues/6#issuecomment-6958687746)

## Follow-up report: module outlines disappear across revisions

After closure, the participant reported that module outlines disappear when bubbles show
non-live revisions. An independent Chromium replay on the pinned lpc-rs source opened
`run_entry` and `open_attempt`, which shared the `lpc_rs::interpreter::task` outline.
**Pin version** retained the outline. **Previous change** cleared `run_entry`'s label and
outline, and **Return to live** restored them. **Compare** cleared the label on both
sides, including After from the saved working tree.

The history and comparison responses had `symbol.module: null`. Those captures contain
the selected file without the manifests and declaring modules used to establish its Rust
membership. The remaining live peer retained its label. With only one member left in
the original module, the outline disappeared as designed. Preserve module identity when
the captured source establishes it, especially when creating After from a live source
snapshot. Historical membership needs evidence from the selected revision. Follow-up is
tracked in [issue #20](https://github.com/fixed-width/vizzi/issues/20).
