# Readable, navigable source and explicit comparison inputs

This contract addresses the first maintainer observations recorded in
[the trial findings](../../experiments/usability/FINDINGS.md). The audience is
implementers and reviewers. No design mock is supplied; retain the current local-font,
light source-workspace visual vocabulary.

## Source and declarations

Both exploration and paired review show syntax colours for analysed Rust, Python,
C and C++ source. Colours distinguish keywords, types, callable names, literals,
strings and comments without claiming compiler/semantic resolution. Colouring is bound
to captured document versions and UTF-8 byte ranges, including clipped/long-line
fragments and multiline constructs. Text remains literal and readable when colours
are unavailable or reach a stated resource limit. Scrolling never reparses a file.

Declaration entries distinguish kinds with a labelled colour badge. The number reads
`Line N` and the accessible name explains that selecting the entry reveals its source.
Colour supplements text. Selection and diff colours remain distinct.

File-map tooltips use one clear coverage phrase, without duplicate `syntax` text.
Counts collapse redundant shown/matching numbers when the complete result is visible.
Limited/page results still expose the visible range, total and a way to refine/page.
Remove the development footer from ordinary use; retain actionable capture errors,
availability and operation status where they help the reader.

## Layout and navigation

Pack directory groups into a balanced map rather than a long vertical strip. Wheel
zoom remains available. Add explicit zoom/overview controls and an overview navigator
that can jump to another region without repeated large pans. Selection/hit testing
stays linked to the same native geometry and source identities.

Users can resize the main map/source split, file-list/content split, paired source
panes and each outline/source split. Separators support pointer and keyboard use,
have accessible names/values and clamp to usable dimensions. Save bounded numeric
layout preferences locally; storage refusal must not break the interface. Narrow
stacked layouts remain usable without horizontal page overflow.

## Comparison inputs and errors

The default mode compares a base revision with the observed working tree. A separate
mode compares two existing revisions. Both resolve once; later ref/disk changes do
not retarget the displayed pair. Revision comparisons do not offer working refresh.
Both modes use the same source policy, analysis/admission budgets and bounded views.

Label the scope as an optional repository path (file/directory), not another revision.
Unknown revisions, malformed paths and non-root paths with no inventory on either
side produce an actionable visible error beside the form. An empty valid repository
root can still be a coherent comparison. Failed/refused capture retains the prior
pair; errors never disappear merely because no view exists yet. Retrying valid inputs
clears the error on success. Keep exact token/side/version and history-expiry rules.

## Verification

TDD the visible labels/counts, source-style ranges, balanced geometry, layout controls,
error states and two-revision capture. Tiny repositories cover moved refs, dirty disk,
invalid targets/paths, deleted-only paths and equal commits. Browser tests cover real
colour spans, declaration badges, input failures, resize/overview interaction and both
comparison modes. Keep source/payload/DOM bounds and the learning/review tasks as
regressions. Use serial lower-priority mutations and optimized scoped observations.
