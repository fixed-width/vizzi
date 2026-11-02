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

`bytes` returns the captured bytes; `text` returns UTF-8 text or `None`; `encoding`
distinguishes those cases. Unsupported input remains raw internally, while textual
HTTP/view responses remain unavailable. This changes internal binary-source retention,
without adding an encoding conversion or a binary transport endpoint.

`Document::capture_source` binds a shared source to workspace/path, classification,
and analyses. It validates content identity, contexts, ranges, keys, and encoding
coverage using the same rules as the borrowed-byte constructor. `Document::capture`
remains the convenience operation that copies its borrowed input into owned storage.
`Document::captured` exposes a shared immutable handle for further verified capture.

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
