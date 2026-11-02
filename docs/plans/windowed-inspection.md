# Bounded captured-document inspection

Implemented: [protocol](../prototype/WINDOWS.md) and
[validation/measurements](../../experiments/windows/RESULTS.md).

Before this change, the browser received complete selected text and outlines on every view
refresh, despite bounded DOM. Replace that delivery with a summary plus pinned windows.
Keep bulk schema-2 diagnostic operations available; the browser opts into schema-3
summary views and a separate schema-1 window operation.

## Native contract

Summary views reconcile selection and return document version, line/declaration counts,
context coverage, and a bounded active-declaration description. They copy no text or
complete declaration list. Outline identities are indexed lazily once per immutable
document when identity lookup is needed; ordinary range reads derive only returned IDs.

Window requests require snapshot, file, and document version. Expired snapshots or
mismatched versions return conflict, never current substitute data. Source has at most
96 rows and 32 KiB text; outline has at most 200 entries plus a cursor pin. Names/kinds
are UTF-8-clipped to 128/32 bytes with an explicit flag. Fixed metadata and JSON escaping
remain bounded; test adversarial control characters and generated names, not just rows.

Each source row describes one UTF-8-safe line fragment, offsets, total bytes, and left/
right continuation. A request can move through one long line without transferring it
whole. CRLF display and empty/trailing lines preserve existing reader conventions.

Optional old snapshot, focused entity, and top-row anchor permit identity-based recovery
across updates. Surviving IDs retain focus/scroll; removed focus prefers an old surviving
successor, then predecessor. If the old pin has expired, use a clamped current ordinal.
Content/version checks remain independent of source control and editor integrations.

## Browser contract

Prepare initial windows before accepting a new main view. Overview, metadata, selection,
source, and outline therefore change together. Scroll/keyboard page requests use the
accepted binding and their own abort/request tokens. Late replies, including replies
whose transport ignores abort, cannot mutate another view/version. Failure keeps the
last coherent view and restarts normal resynchronisation on pin loss.

Hold one bounded visible source/outline window and a focus pin, rather than a growing
page cache or a full outline index. Preserve logical Home/End/Arrow/Page navigation,
native button activation, source linking, old focus/scroll identities, and literal text.
Pending navigation must not accidentally activate the old focused declaration.

Long lines have explicit earlier/later/first/last fragment controls. This keeps complete
source reachable while bounding text in each row; truncation is never silent. Bound
viewport growth and scroll/resize scheduling as well as response and DOM sizes.

## Validation

Use TDD for line/byte limits, Unicode/CRLF/empty/binary input, huge lines/names, selection,
old pins, version rejection, focused-anchor recovery, out-of-order responses, rapid
scroll/navigation, and live edits. Run real Chromium checks and focused/full relevant
Rust/JS mutation scopes. Measure response sizes and large-file interaction separately
from indexing. Preserve raw before/after observations and distinguish transport bounds
from unavoidable indexing, lazy identity-index construction, and full reconciliation.
