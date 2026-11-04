# Contextual local-change review: behaviour contract

Status: specified for implementation; the review interface is not delivered yet.
This reference defines milestone 2 in the [product plan](../plans/product-milestones.md).
The [technical contract](COMPARISON_TECH.md) defines identities, ownership and limits.

## Comparison scope

A developer selects an explicit Git baseline in the configured workspace and a
workspace-relative subtree, defaulting to the repository root. The host resolves the
baseline once and displays its resolved identity. Moving that ref later does not change
the comparison. The working side captures tracked paths and non-ignored untracked
additions as observed on disk, including staged and unstaged edits together. The index
can identify candidate paths; its staged bytes are not the working source.

Both sides are immutable observations. The interface labels the baseline, subtree and
working capture time. Filesystem capture is not an atomic transaction across files.
Ordinary workspace publication does not change a displayed comparison. **Refresh
working** explicitly captures another working observation against the same baseline;
selecting a new baseline or scope creates a new comparison. No continuous review capture
or automatic hunk movement is required in this slice.

The file filter only changes the projection. It does not narrow capture scope or hide
excluded/unassessed counts. Unchanged related source in the captured scope is available
on either side. Access outside that scope requires an explicit new comparison; the
interface does not silently read current files under an older comparison label.

## Linked views

The review workspace coordinates:

- a changed-file overview/list with added, removed, modified, type/mode changes and
  unassessed entries, including paths without language analysis;
- before/after source panes with raw textual hunks and an indication of which side is
  selected; selecting a hunk reveals its corresponding ranges;
- lexical declaration context on the selected side and navigation to relevant unchanged
  source, including the contract or test needed to evaluate the change.

Selecting a file, hunk or declaration preserves comparison identity and side across
these views. Deletions remain readable from the base. Missing sides are labelled absent;
an empty file remains a present file. Changes outside declaration spans remain visible
at file level. Declaration matches provide context only: duplicate/ambiguous keys have
no inferred correspondence. The interface makes no semantic-equivalence, call-impact,
symbol-move or rename claim. Path renames initially appear as an addition and a deletion.

Ordinary UTF-8 source, documentation and configuration can be inspected without an
analyser. Coverage, unsupported encoding/binary content, symlink targets, gitlink
pointers, executable-mode changes and explicit exclusions have distinct descriptions.
Symlink targets are displayed as literal targets without following them; submodules
are not recursively captured. Unreadable or unsupported paths remain visible as
unassessed. Their contents cannot be labelled unchanged merely because no diff exists.
Large files or exhausted diff limits retain source inspection where available and show
why textual hunks are unavailable. Paged results expose totals and remaining counts.

Raw comparisons preserve whitespace, line endings and final-newline differences. No
project build, hooks, content filter, diff driver, network fetch or external tool runs
to create a review. Git LFS pointers remain the captured pointer bytes. Reviewing,
publishing comments, approving and merging are separate actions; only read-only review
is in this milestone.

## Refresh, navigation and failure

During capture or a failed/refused refresh, the last coherent comparison stays visible
with its original labels. A successful refresh replaces the pair as one accepted view;
late responses from the previous pair cannot replace it. Selection may carry forward by
path and an unambiguous declaration key, with a notice and a fresh reveal when source
changed. Old line/hunk coordinates are never transferred to new bytes.

Review history records comparison identity and side as well as the bounded reading
metadata. Returns require that exact pair. A replaced, closed or host-expired comparison
produces an explicit expiry state, preserving the last coherent view, with an explicit
action to open the current comparison. This differs from the live-source return policy
in [investigation navigation](NAVIGATION.md); neither mode can silently switch to the
other. Review visits retain metadata, not source owners or long-lived baseline leases.
Keyboard selection, pane focus, accessible names and Back/Forward remain supported.

Invalid baselines, unsupported workspace states, incomplete capture and resource refusal
are actionable states. A failed initial capture shows no invented empty comparison.
An incomplete per-path observation can be published with unassessed counts; failure to
enumerate the scope prevents publication of a supposedly complete inventory.

## Acceptance evidence

Use the frozen [review fixture](../../experiments/review-workflow/README.md). Both neutral
variants must include the unstaged Rust edit and untracked Markdown document. A guided
walkthrough must identify the regression in the defective variant, accept the control's
lookup behaviour, and cite changed source plus unchanged reader/path/test evidence.
Record first-use delays, missing context and operator familiarity. Fixture preparation
alone does not establish UI acceptance or a comprehension improvement.

Correctness checks additionally cover the technical contract's inventory matrix, stale
requests, bounded ownership, moved refs, interrupted capture, source-only mixed-language
paths and a non-Git provider. Focused Linux/PyTorch observations follow a functioning
review path and measure the new views against the [budgets](../performance/BUDGETS.md).
