# Captured-document window protocol

The browser requests schema-3 summary views and schema-1 document windows. Bulk
schema-2 view/source operations remain available for explicit diagnostics and existing
consumers. Status/notifications keep schema 1; identity composition is unchanged.

## Summary views

`GET /api/view?projection=summary` accepts the existing snapshot, file, entity, filter,
and overview-limit parameters. It returns the normal bounded overview and reconciled
selection with view schema 3. Selected source retains identity, version, path, encoding,
classification, context coverage, and diagnostics, but has null `text` and empty
declaration vectors.

Its `summary` contains logical `lines`, global `declarations`, and bounded selected
`active` entry or null. Active selection is independent of the loaded outline window.
A missing selected entity falls back to file selection. Empty source selection is null.

Ordinal range access projects only returned entries. Entity/anchor lookup creates a
derived identity index lazily once per immutable document. That first lookup remains
linear in declaration count and retains its index with that document. It does not
copy full source or every declaration name into each response.

## Pinned windows

`GET /api/window` requires `snapshot`, `file`, and `version`. The response identifies
all three plus schema 1. Unknown/expired snapshots and version mismatch return **409**;
a missing file returns **404**; missing/invalid typed parameters return **400**. A request
never substitutes current source for its requested pin.

| Parameter | Meaning / bound |
| --- | --- |
| `line_start` | One-based source line, clamped to the document |
| `line_count` | 1–96 rows, default 96 |
| `reflow` | Optional boolean; `true` includes display-break hints and original line terminators |
| `column_line`, `column` | One line's UTF-8 byte-column start; rounded down to a character boundary |
| `outline_start` | Zero-based ordinal across captured analysis contexts |
| `outline_count` | 1–200 rows, default 200 |
| `cursor_index` | Logical focus ordinal, clamped to the current outline |
| `cursor` | Focus identity, preferred over its fallback ordinal |
| `anchor`, `anchor_offset` | Stable visible identity and its offset from the requested window start |
| `previous_snapshot` | Optional old pin for removed-focus successor/predecessor recovery |

Source replies have `line_total`, actual `line_start`, and `lines`. Each line contains
its one-based `number`, byte `column`, exclusive `end_column`, complete `line_bytes`,
literal `text`, and `left`/`right` continuation flags. LF/CRLF delimiters are excluded
from display fragments; an empty final line is retained. Unsupported encoding has zero
display lines. The aggregate unescaped text limit is 32 KiB, divided among actual rows.
Requesting one row permits a larger fragment of a generated long line.

Outline replies have `outline_total`, actual `outline_start`, `outline`, and a separate
`cursor` entry or null. Entries include global `index`, analysis ordinal, entity ID,
name/kind, truncation flags, and inclusive one-based source line bounds. Entries now
also carry `depth` in the emitted lexical declaration forest and nullable `scope`,
the nearest emitted parent's display name (at most 24 UTF-8 bytes including ellipsis).
Parents are supplied by the syntax adapter, not inferred from overlapping spans.
Depth is per analysis context and counts emitted named scopes, not every source block.
Legacy adapters without parent links remain flat. Names/kinds
are at most 128/32 UTF-8 bytes, respectively. The cursor can lie outside the page.

These count/text/name limits also bound JSON escaping and fixed metadata. Adversarial
control-character/name tests verify an ordinary document-window body below 512 KiB. This bound
applies to the document-window reply, not the overview or explicit bulk diagnostics.
Truncated labels do not replace full source: fragments retain complete source access.

With `reflow=true`, each line also has `ending` (`"\n"`, `"\r\n"`, or `""`). Eligible lines
have `reflow:{leading,points:[{offset,anchor,priority}]}`. Offsets and nullable delimiter
anchors are fragment-relative UTF-8 bytes; `leading` ends the initial spaces/tabs. Lower
priorities prefer structural breaks; eight is a whitespace fallback. The projection admits
at most 256 points per line and 1,024 per window, atomically per line. Unknown language
profiles, limited/unavailable styles and left-clipped fragments omit hints. This optional
metadata adds less than 64 KiB to the ordinary reply bound and does not change source or
snapshot identity. [Source reflow](../explanation/source-reflow.md) explains display policy.

## Captured syntax styles

Windows add `coloring:"ready"|"limited"|"unavailable"`. Each source fragment can carry
`styles:[{start,end,kind}]`; offsets are relative UTF-8 bytes, sorted and non-overlapping.
Missing styles mean plain text. Vocabulary: `keyword`, `type`, `function`, `string`,
`comment`, `number`, `constant`, `attribute`. These are lexical categories, without
semantic name resolution. Older plain window producers remain accepted by the client.

Built-in grammar queries run over the existing captured parse once per analysis. The
immutable sparse index retains at most **1 MiB** of packed boundaries per document and
persists as base64 little-endian u64 data. Adjacent equal styles merge; outer captures
take precedence over nested captures. Admission exhaustion discards the whole style
index while preserving source and declarations. Custom adapters may omit it.

Projection looks up the requested byte range with binary search. A complete window
admits at most **1,024** style spans; exhaustion returns all its source as plain text
with `limited` status. UTF-8 boundaries and source/version binding remain mandatory.
Style metadata participates in document identity and built-in implementation/cache
fingerprints. The same rules apply to resident and spooled source and both review sides.
The browser validates styles and renders literal text nodes and spans; it never parses
source during scrolling or treats captured text as HTML.

## Identity recovery and client application

Surviving focus and top-row IDs map to current ordinals. Removed focus prefers the next
old logical declaration still present, then an earlier survivor. If the optional old
pin expired, use the clamped current ordinal. Old pins are recovery evidence only;
all returned text/facts belong to the requested current snapshot/version.

The browser prepares an initial window before applying a summary view. Overview,
metadata, selection, source, and outline change together. Subsequent viewport requests
use that accepted binding with their own abort/token state. Rebinding invalidates old
requests, including late replies whose transport ignored abort. Pin loss invokes the
existing resynchronisation path and keeps the last coherent view until replacement.

The client retains one finite viewport window and cursor, rather than full source,
a growing page cache, or a complete declaration index. Home/End/Arrow/Page keys navigate
logical ordinals. Activation while a target page is pending waits for that target;
it never activates the old focused button. Updates preserve surviving controls/focus
and anchored scroll without stealing outside focus.

Generated long lines expose First/Earlier/Later/Last text controls. They replace a
fragment rather than append unbounded text. Source and outline rows remain virtualised
and panes cap extreme viewport growth. DOM, data delivery, index creation, full analysis,
and periodic reconciliation retain distinct costs; this protocol does not impose a
whole-process memory cap or a presentation-time guarantee.
