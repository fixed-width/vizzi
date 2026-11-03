# Shared captured-source reference

`vizzi_core::model::CapturedSource` owns immutable source bytes, content identity,
encoding availability, and a line index. It has no workspace/path, classification,
analysis context, filesystem metadata, or source-control identity.

## Capture and document binding

`CapturedSource::capture(Vec<u8>, Option<&Arc<CapturedSource>>)` hashes the newly supplied
bytes on every call. Matching content reuses the previous allocation. Otherwise it
adopts the owned buffer and builds its UTF-8 line index once. Safe
[`String::from_utf8`](https://doc.rust-lang.org/std/string/struct.String.html#method.from_utf8)
uses the vector's storage without copying its contents; invalid input keeps its original
raw byte vector. No unsafe conversion, global interning, or metadata shortcut is used.

Whole-file access is explicit and fallible: `read_all(limit)` returns a verified input
guard and `read_text(limit)` returns borrowed/owned UTF-8 or unsupported encoding.
`reader().read(range, limit)` provides bounded integrity-checked access. `len`, `encoding`
and line/declaration metadata do not load backing. Explicit `resident_bytes` and
`resident_text` diagnostics describe resident availability rather than encoding.
See [source reads/backing](SOURCE_READS.md) for errors, guards and spool lifetime.

`Document::capture_loaded` binds facts using the same verified input held during
analysis/cache validation. `capture_source` remains a resident convenience or a
metadata-only binding path for empty facts. `Document::resident_source` is an explicit
resident diagnostic; production inspection uses reads. `Document::captured` exposes
immutable metadata/backing independently of classification and facts.

Different analyses and classifications may share one source object. They still have
their own document versions and facts. Document composition v2 and snapshot composition
v4 remain; allocation identity is not observation identity. Analyzer fingerprints now
also include the captured-source implementation, conservatively invalidating old
analysis-cache contexts once after this update.

## Native capture and cancellation

`Directory::capture_reusing(previous, scope, cancelled)` returns a complete `Capture`,
no result on observed cancellation, or an I/O/configuration error. Existing complete
and hinted convenience operations remain. Full/partial selection, ancestor checks,
symlink rules, and conservative full-discovery fallback are unchanged.

The adapter reads a file, hashes its actual bytes, and then owns or shares its source.
Jobs and resulting documents hold the same `Arc`, rather than separate byte/text
buffers. Context changes can therefore reparse verified source while retaining old
published observations without copying all their text and line indices.

Checks occur before work, between directory entries/path components/file reads, and
after capture. Cancellation discards partial results; it cannot acknowledge dirty
hints or publish an incomplete snapshot. Blocking I/O, hashing, decoding, indexing,
and destructor work retain their cooperative-cancellation limits.

`shared_sources` counts reopened inputs sharing verified source storage with the
baseline. They still count as `read_files` and contribute all read `bytes`. This is
separate from unread `retained` documents, complete-document `reused` analysis,
persisted hits, and syntax-tree reuse. It never means source verification was skipped.

Source hashing/decoding/index construction now occurs inside native capture.
`capture_ms` includes that work; before this change some of it occurred later in the
engine/document phases. Compare total engine time rather than treating capture-phase
times as the same I/O-only endpoint across these implementations.

## Resource limits

The duplicate temporary read overlap becomes one file at a time; new source storage
is moved into the candidate and later observation. A cold or wholly changed workspace
still needs all its candidate source. Different old/new contents can coexist, analysis
facts may be duplicated, and the parser retains its ordinary working allocations.

This change is not a byte-admission cap, a bounded analysis queue, a different recovery
schedule, or paged source/outline transport. The opt-in syntax cache still owns its
separate source/tree state and admission quota. See the
[implementation plan](../plans/shared-source.md) for validation and measurement scope.

## Owned-buffer accounting

`CapturedSource::storage()` returns `SourceStorage` with logical `source_bytes`,
`byte_capacity`, stored `index_entries`, allocated `index_capacity`, and `index_bytes`,
plus compact CRLF `terminator_bytes` and block-digest `integrity_bytes`. Backed source has
zero permanent byte capacity; its index/digest metadata remains resident.
It reads existing byte/string and line-index capacities without copying, hashing,
changing recency or mutating source. Unsupported encoding retains the initial stored
zero index even though its display line count is zero; empty UTF-8 has one display line.
Index bytes use `index_capacity × size_of::<usize>() + terminator_bytes`. Integrity
capacity is separate; the current source-storage inventory includes all three buffers.

These values account only for owned buffers. Struct/Arc/document/fact overhead, parser
allocations, allocator metadata and resident-page behaviour remain separate. Inventories
across snapshots must deduplicate actual source allocation identity; equal content does
not prove shared storage. An external query handle can own data after publication
history expires its lookup identity, and releasing data does not force RSS to fall.

The [source-storage investigation](../../experiments/source-storage/RESULTS.md) measures
these sets. Resident capture remains the default. The
[bounded-read proposal](../plans/bounded-source-reads.md) records the next interface step;
this accounting operation does not impose a memory limit. The
[reader/spool follow-up](../../experiments/source-reads/RESULTS.md) evaluates opt-in backing.
