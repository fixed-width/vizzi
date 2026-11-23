# Function history in the call workspace

For readers comparing an individual function with earlier committed versions while
keeping their call trail visible. History uses the existing bubble design; no separate
mock was supplied.

## Behaviour

1. A current repository function offers **Previous change**. Its first use freezes
   local HEAD and follows first-parent history for that file. Commits that leave the
   captured declaration unchanged are skipped, including changes elsewhere in the file.
   Consecutive identical versions collapse to their earliest known commit, so later
   unrelated file edits do not receive credit for changing the function.
   Uncommitted saved changes can therefore step back to HEAD. Unsaved buffers are outside
   this history.
2. A historical bubble shows its commit, **Previous change**, **Next change** and
   **Return to live**. Next change retraces accepted versions. At the newest historical
   version, **Return to live** takes its place in the history row and resumes current
   saved-source tracking. The bubble shows a pending status while retaining the capture.
   If the refresh fails, the reader can keep stepping through the retained history or
   retry **Return to live**. If the lookup cannot identify the current function, show
   the unavailable message and retain the displayed revision and its source binding.
   Later publications can resolve it again. When a successful refresh removes the
   focused history-row return control, move focus to **Compare** or **Previous change**.
3. Changing revision preserves the bubble identity, rectangle, task membership, camera
   and other bubbles. A reading section retains its relative position where possible;
   changed text resets scrolling within that section. Source and revision labels change
   together, only after successful preparation.
4. Historical bubbles are read-only. Calls follow declarations in the exact captured file
   through [captured call exploration](CAPTURED_CALLS_PRODUCT.md). Callers, references, Save
   and Editor remain current-source actions. Existing arrows become historical; captured
   links retain their own revision/evidence rather than adopting working-tree facts.
5. Ordinary matching requires an unambiguous declaration in the same file and lexical
   scope. Missing, renamed, moved or ambiguous declarations stop automatic traversal.
   The last readable source remains. At a committed boundary, readers can inspect and
   accept a supported [rename or move](HISTORY_LINEAGE_PRODUCT.md).
6. History uses local objects, without checkout, builds or fetching. A non-Git directory,
   unborn HEAD, unavailable/shallow history, unsupported capture or exceeded bound produces
   a useful message without replacing the bubble. A failed step can be retried.
7. History is bounded to the latest 64 file revisions. A limit or shallow boundary is
   labelled separately from reaching the beginning of available complete history.
8. Loading disables conflicting bubble source actions. Closing a bubble, switching tasks
   or returning live invalidates pending results. Saved-file changes in other bubbles
   continue normally once the operation completes. Text selection defers replacing source.
9. Historical bindings and layout persist with tasks. Native history sessions can expire
   or be lost on restart; unavailable captures are explicit, with Return to live available.
   Pinning and browser storage do not promise a durable source archive.
10. Controls use normal keyboard-focusable buttons, existing theme tokens and an announced
    status. Dependency-source bubbles retain their existing behavior without Git history.

## Choose an exact revision

The reviewer in the [maintainer trial](../../experiments/usability/function-history/FINDINGS.md#h04-the-review-base-has-no-distinct-function-history-entry)
could not reach a named review base through distinct changes because the function was
unchanged at that commit. [Issue #8](https://github.com/fixed-width/vizzi/issues/8)
adds exact selection using Vizzi's existing bubble and dialog design.

1. Choose **Choose revision…** on a repository function with an unambiguous source
   binding. In a comparison bubble, the command is in the bubble's actions menu.
   The dialog accepts a local commit ID, branch or tag up to 256 bytes. Enter
   submits, and Escape or **Cancel** dismisses the dialog even during preparation.
   The input receives focus on open and focus returns to the bubble control on close.
2. The chosen commit appears even when the function is unchanged there. A branch or tag
   resolves once. Revisiting or reloading the bubble shows the same immutable commit.
   The short ID in the bubble has the full ID in its accessible description and tooltip.
3. The chosen commit starts a first-parent timeline for that bubble. **Previous change**
   finds the next distinct older declaration, and **Next change** retraces earlier visits
   back to the exact selected commit. At that starting point the onward action is **Return
   to live**. Choosing another revision starts another timeline.
4. **Compare** pairs the selected source with its previous distinct version. Choosing a
   revision in one comparison bubble exits the diff in that bubble. The other side remains
   readable. Captured calls use the selected commit's file and preserve their existing
   source evidence and revision labelling.
5. Selection preserves bubble identity, geometry, task membership, camera and independent
   views. It preserves reading position in unchanged declaration text and keeps the relative
   reading section when the declaration changes. Source, revision and links change together
   once the selected declaration and source are ready. A newer source intent supersedes a
   pending selection, including a late success or failure.
6. Invalid or unavailable commits, missing or ambiguous declarations and resource refusal
   leave the existing bubble readable. The dialog reports the failure and supports retry.
   Selection uses local objects and the same file and lexical scope as the source binding.
   Canceled preparation preserves retained history and captured calls. Existing history
   limits, capture expiry and Return to live apply after adoption.
