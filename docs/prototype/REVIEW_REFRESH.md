# Preserve context across working-source refresh

This reference specifies review refresh for implementers and reviewers. The
[startup measurements](../../experiments/review-workflow/STARTUP_RESULTS.md) show extra
file-selection actions after 6/15 Linux and 9/15 PyTorch refreshes. This slice preserves
useful context while retaining the comparison's immutable source and expiry rules.

## Behaviour

Working refresh retains the last accepted file, selected side, path filter, unchanged
context setting and file page as the new publication is observed. Navigation and
scrolling can continue during capture against the old pair. A page beyond the new
matching total moves to the last valid page. A selected file can remain outside the
page/filter, including when it becomes unchanged. Verified absence from both sides
selects the first available row and explains the fallback. A removed side switches to
the remaining side; unavailable source remains explicitly unavailable.

Each pane restores its source/outline scroll, horizontal position, logical cursor and
text fragment only when that exact side document version is unchanged. Changed source
gets fresh coordinates. A selected declaration survives only if its exact lexical
identity is present in the new version of the same side; reveal its current location.
Missing/ambiguous declarations fall back to file start with a notice. Other changed
panes start at the file start. Hunk IDs and hunk/edit page offsets never cross a token
change; fresh hunk navigation remains available.

An expanded active fragment can reuse its current inspector bytes only for the same
file/version and actual returned byte column. A regular multi-line response must not
shrink that fragment. Source windows retain their existing per-request bounds; no source
is added to history. End-fragment requests allow for up to three bytes of UTF-8 boundary
rounding so that the final character remains accessible.

An explicit new baseline/scope capture resets context, even if its contents equal the
old pair. Reconnect/coalesced notifications can recognise refresh without relying on
whether the accepting HTTP reply arrived first. A failed/refused refresh keeps the
old coherent view. Failure to load the replacement also keeps that view with recovery
controls. Old history entries still require their exact token and expire normally;
preserved context is a new visit, never a revival of an old lease.

An older navigation render does not overwrite an unsubmitted debounced filter draft.
Explicit new capture, close and history/recovery actions cancel that pending draft.

Replacement source, versions, diff metadata and both inspector windows are accepted
together. Late responses cannot restore an old position or overwrite a newer intent.
Navigation on an already superseded displayed pair cannot cancel its replacement load.
History remains bounded metadata only. No source owner or declaration/diff array is
added to saved navigation state.

## Native and browser interfaces

Comparison labels add `origin`, the token of the initial baseline capture. A new create
gets a fresh origin; working refresh preserves it with the frozen baseline spec.
It is opaque native lifecycle metadata, independent of SCM and portable comparison ID.
It grants no access to retired tokens. Older metadata without origin follows the
existing reset policy.

`ReviewPageQuery.file` optionally requests one opaque file identity. `ReviewFiles`
adds `selected`, a bounded inventory row or null for verified absence, independently
of page/filter/unchanged projection. Use the existing pair inventory indexes, with
the same row projection as the list. No path matching, pagination scan, source IO or
unbounded response is needed to recover fresh document versions.

The browser controller owns refresh reconciliation. Matching origins permit file
lookup in the new pair; per-side version equality permits viewport reuse. Changed
declaration lookup uses the existing current-version window cursor, checks exact
identity and then loads a reveal from the returned current location. It never reuses
old declaration coordinates or substitutes an ordinal match. Rendering continues to
own inspector state. Native/core source identity and retention policy remain unchanged.

## Verification

TDD native origin continuity/new-create distinction, bounded selected-row lookup beyond
pages/filters, missing files and fresh versions. Test the public browser controller for
per-side restoration, changed/missing declarations, side/file loss, clean/no-change
refresh, page shrink, failed/refused/reordered work, reconnect and history expiry.
Real Chromium checks include independent pane positions and source versions. Repeat
both frozen review variants and the learning task, then the same optimized scoped
Linux/PyTorch measurements. Keep correctness/mutation checks outside performance runs
and preserve the earlier cohort. Run mutation jobs serially at lower priority.
