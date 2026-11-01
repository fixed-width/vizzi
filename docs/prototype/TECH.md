# First workspace technical contract

The implementation separates an immutable Rust model from native source access,
persistence, scheduling, and host transports. These interfaces are for the first
vertical slice; the performance experiment crate remains independent.

## Modules

- `vizzi-core` owns typed identities, captured documents, source-backed syntax facts,
  snapshot queries, selection reconciliation, layout, and publication rules. It does
  not open files, watch directories, or know about HTTP and Git.
- Its Rust analyzer owns a reusable Tree-sitter parser and emits lexical declaration
  facts. Analysis carries a context and content identity. Source ranges use UTF-8 byte
  offsets; line ranges are derived from the captured source.
- The native `vizzi` host captures source, runs bounded analyzer workers, persists
  analysis, coordinates refresh requests, and owns published snapshots.
- HTTP and stdio diagnostics adapt the same application operations. Browser code owns
  input handling, painting, and transport state, with no source-analysis implementation.

## Identity and consistency

A workspace identity is opaque; the initial directory adapter derives it from the
canonical root. A file identity is scoped to that workspace and relative path. Content
identities are hashes of captured bytes, independently of source-control identifiers.

Documents retain captured source and can contain analyses for multiple contexts.
Analysis contexts identify language, analyzer, configuration, and implementation.
The first host configures one Rust syntax analyzer. Its fingerprint conservatively
includes core implementation sources and the dependency lockfile.

Snapshots sort documents and identify their actual captured contents and analysis.
They represent immutable observations, not an atomic filesystem transaction. A source
query never reads a mutable file after choosing its snapshot.

Declaration keys use lexical qualification and kind. Unique keys remain stable across
position-only changes. Duplicate keys receive content-scoped identities, so edits do
not silently move a selection to another ambiguous declaration. Renames and moves are
not inferred in this slice.

Refresh tickets increase when new work is requested. Publication checks the ticket
under the same short metadata lock used to request refreshes. Obsolete results cannot
overwrite a newer request. Unchanged snapshots keep their generation. A small retained
history permits pinned queries; expired versions return a conflict instead of mixed data.

## Cache and native scheduling

The analysis-cache key combines content and context identities. Cached facts are
independent of workspace paths and are rebound to captured files on load. Records have
a schema version and payload checksum. Temporary files and atomic replacement protect
writes. Invalid records are misses; write failures are reported as cache warnings while
valid analysis remains usable. JSON is a prototype implementation, not a storage decision.

The native coordinator watches source directories and periodically reconciles. It
captures inputs before analysis, reuses unchanged documents, and keeps parser ownership
local to a bounded worker. Cancellation is checked between files. A long individual
parse is not interruptible in this slice. Parsing never holds the publication lock.

## Host interface

- `GET /api/status` returns generation, snapshot identity, pending state, and last
  analysis/cache measurements.
- `GET /api/view` returns a bounded overview, reconciled selection, and selected source
  from one snapshot. File and declaration identities are opaque query parameters.
- `GET /api/source` supports an explicitly pinned snapshot and file identity. Unknown
  or expired snapshots fail explicitly.
- The diagnostic host accepts JSON lines carrying an id and request URI, and returns
  the HTTP status, content type, and text body from those same handlers.

The browser polls small status records and reloads a view on publication or user action.
A request epoch rejects older responses. Source lines are virtualised; map geometry is
supplied by Rust, with visible-item painting and hit testing in the host adapter.

## Rendering and validation scope

The initial browser uses DOM controls and a Canvas2D reference overview. This validates
coordinated views and the transport contract without settling the production renderer.
Rust/Wasm with wgpu remains the leading GPU candidate. The current sandbox lacks an
installed browser Wasm target, blocks listeners, and prevents Chromium startup through
its IPC restrictions. HTTP handlers can be tested in process, and the stdio host can
exercise the live native engine without sockets. Browser layout, frame-time tails,
WebGPU/WebGL comparison, and hardware GPU qualification remain explicit follow-up work.

The UI uses a cool light workspace palette, blue selection, amber partial analysis,
and green change feedback. The overview is the primary visual object; source and its
outline stay adjacent. System-local sans and monospace fonts keep the prototype offline.
