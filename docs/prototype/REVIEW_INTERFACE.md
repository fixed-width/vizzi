# Browser review interface contract

This reference describes the browser implementation of the
[review behaviour](COMPARISON_PRODUCT.md), using the existing
[comparison API](COMPARISON_API.md). It serves implementers and reviewers.

## Layout and selection

`/review` has an explicit baseline/scope form, capture labels, a paged file list,
hunk navigation and two captured-source inspectors. The workspace links to this page;
review links back to exploration. Each mode owns its own session navigation policy.

The visual vocabulary follows the existing workspace: slate ink `#183044`, white
panes `#ffffff`, blue selection `#265dc5`, pale background `#f3f6f9`, removed-source
red `#9e383b` and added-source green `#226147`. Existing sans-serif controls and
monospaced source remain. Change labels and markers accompany colour. The useful
distinction is the paired source; surrounding controls stay quiet and left aligned.

```text
baseline / scope / compare / refresh / close
resolved baseline and working observation interval
files + filter | hunks and change-page controls
              | base source + outline | working source + outline
```

The file projection includes unchanged context when requested. Filtering never
changes capture scope or full inventory totals. File pages and hunk/edit metadata
pages cap at 200. Both inspectors retain the established 96-row/32-KiB source and
200-entry outline bounds, including fragment and keyboard navigation.

File, hunk, declaration and side selection carry token, comparison ID and exact side
document versions. Hunk selection reveals both ranges; declaration selection reveals
only its selected side. Empty edit ranges remain anchors, not invented changed lines.
Only returned edit metadata colours source. Further change pages are explicit.
Byte-range details make line-ending/final-newline edits visible even when displayed
source text looks identical. No declaration correspondence is inferred.

## Requests and lifecycle

The browser controller owns intent, independent review SSE, bounded history and
atomic acceptance. Rendering owns DOM and inspector viewports. A review window reader
validates token/comparison/side before the shared document-window validator. It never
uses the live workspace endpoint or snapshot history to satisfy a review request.

Diff queries finish before the two side windows are loaded. Source scrolling retains
at most one active request per inspector. Obsolete completions cannot publish after
a newer intent or capture. A failed navigation keeps the coherent previous view.
Unavailable diff/source is described explicitly; absent, empty, binary, literal
symlink/reference, executable and unobserved evidence remain distinct.

Initial capture is explicit. Review SSE supplies pending/error/active state without
polling. Failed/refused refresh keeps the old pair. A successful new capture resets
selection and reading positions with a notice; no old hunk coordinates cross into
new bytes. Close retires the pair. Disconnection preserves the displayed source and
offers reconnection/inspection recovery rather than inventing an empty result.

## History

At most 50 visits retain comparison labels, bounded file evidence, side/version pins,
hunk/declaration anchors, filter/page state and the two inspector positions. No source
windows, declaration arrays, diff arrays or source owners are retained. Browser
history contains only a session marker and visit key.

Back/Forward requires the exact token and comparison. A superseded, closed or
host-expired target keeps the displayed view and gives an explicit **Open current
comparison** recovery action. Fresh navigation branches from the requested history
position. Filters and scrolling update the visit without adding duplicate locations.
Keyboard controls, focus and native browser history work within each mode.

## Validation

Test the controller and window reader with deterministic transports before wiring
the DOM. Exercise stale completions, failed/refused refresh, expiry, paging, absent
sides and exact anchors. Real Chromium checks cover linked selection, independent
side scrolling, literal rendering, bounded DOM, narrow layout and toolbar/native
history. Mutate the new request/navigation logic with one worker at lower priority.

Repeat the frozen good/bad review tasks through rendered controls and record source
evidence, first-use delays and missing context. Guided acceptance does not establish
unfamiliar-developer usability or large-repository tail latency. Keep the existing
learning workflow as a regression.
