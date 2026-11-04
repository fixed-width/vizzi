# First workspace technical contract

The implementation separates an immutable Rust model from native source access,
persistence, scheduling, and host transports. These interfaces are for the first
vertical slice; the performance experiment crate remains independent.

The [contextual-review contract](COMPARISON_TECH.md) extends this model with
SCM-independent immutable comparisons. Portable inventory/hunk computation is delivered;
native acquisition and host/browser comparison operations remain planned. Its
[behaviour contract](COMPARISON_PRODUCT.md) defines that review workflow.

## Modules

- `vizzi-core` owns typed identities, captured documents, source-backed syntax facts,
  snapshot queries, selection reconciliation, layout, and publication rules. It does
  not open files, watch directories, or know about HTTP and Git.
- Its language adapters own reusable Tree-sitter parsers and emit lexical declaration
  facts. Analysis carries a context and content identity. Source ranges use UTF-8 byte
  offsets; line ranges are derived from the captured source.
- Its portable source policy maps normalized relative paths to capture/assignment
  metadata. Analyzer factories register by language, independently of selection.
- Its capture plan freezes that policy and registered descriptors, validates documents,
  and marks compatible immutable baselines for incremental retention.
- The native `vizzi` host captures source, runs bounded analyzer workers, persists
  analysis, coordinates refresh requests, and owns published snapshots.
- Its `changes` module retains one versioned status for independent native subscriptions.
  Application mutations update it under the publication lock; HTTP serialisation and
  client queries happen outside that lock. Each application gets a fresh stream identity.
- HTTP and stdio diagnostics adapt the same application operations. Browser code owns
  input handling, painting, and transport state, with no source-analysis implementation.

## Identity and consistency

A workspace identity is opaque; the initial directory adapter derives it from the
canonical root. A file identity is scoped to that workspace and relative path. Content
identities are hashes of captured bytes, independently of source-control identifiers.

Documents retain captured source and can contain analyses for multiple contexts.
Analysis contexts identify language, analyzer, configuration, and implementation.
The host registers Rust, Python, C, and C++ syntax adapters through one source policy. Their
fingerprints conservatively include adapter/shared implementation sources and the
dependency lockfile. See [language and worker contracts](LANGUAGES.md).

The [source-policy contract](SOURCE_POLICY.md) defines the implemented selection seam,
classification metadata, and snapshot policy identity. The [C/C++ contract](C_FAMILY_POLICY.md)
also specifies future configured translation-unit inputs, sequenced
in the [implementation plan](../plans/c-family.md). Current analyzers remain byte-only
Rust/Python/C/C++; unassigned/unsupported C-family candidates remain source-only.

Snapshots sort documents and identify their actual captured contents and analysis.
They represent immutable observations, not an atomic filesystem transaction.
[Captured source](CAPTURED_SOURCE.md) shares verified bytes and line indices across
input jobs and documents, including changed analysis contexts. A source
query never reads a mutable file after choosing its snapshot.
Each validated document caches its content/classification/analysis version. Snapshot composition
hashes those fixed-size versions instead of serialising retained facts again. See
[identity composition](IDENTITIES.md) for inputs and versioning.
Snapshot composition also includes capture policy, enforcing complete rediscovery on
policy mismatch without changing workspace/file identity. Rebinding cached raw facts
supplies current classification; source-only files bypass parser creation and persistence.

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

The native coordinator carries verified dirty-path hints into capture and periodically
reconciles the whole source scope. Foreground attempts retain unread documents from
the published baseline; cancelled and superseded jobs do not consume pending hints. Directory or
ambiguous changes fall back to complete discovery. New watch registration closes its
discovery gap with another full pass. See [incremental capture](INCREMENTAL_CAPTURE.md).
The [verified capture plan](CAPTURE_PLAN.md) avoids rescanning unchanged classification
metadata, while snapshot merging and version hashing remain linear in file count.
[Periodic recovery](RECOVERY.md) uses one private verification worker when the budget
permits, reserving remaining workers for foreground hints. Its final fresh journal
overlay prevents recovery from replacing newer foreground source. One shared worker
budget and bounded invalidation queue cover all languages; parser creation is lazy
and ownership remains local to a bounded worker. [Cooperative cancellation](ANALYSIS_CANCELLATION.md)
checks during built-in parsing and extraction as well as between files. It does not
impose a hard deadline on all analysis/capture operations. Parsing never holds the publication lock.
[Opt-in syntax retention](SYNTAX_RETENTION.md) transfers an owned tree and exact source
between workers through exclusive, weight/count-bounded native leases. Published
documents remain immutable and carry no mutable syntax state.

[Opt-in analysis admission](ADMISSION.md) additionally shares active input capacity
across foreground/recovery attempts. Refused jobs retain verified source with explicit
deferred coverage and no stale facts. Deferred records are retried rather than persisted
or reused as complete analysis. Input capacity does not bound captured source or parser
heap; it complements worker and syntax-retention limits.

[Source reads](SOURCE_READS.md) now expose verified loaded-input guards and bounded
range sessions with explicit capacity/IO/integrity failures. Resident capture is the
default; optional private spooling removes permanent source bodies from snapshots.
Session disk quota, index metadata, transient capture and parser memory are distinct.

## Host interface

- `GET /api/status` returns generation, snapshot identity, pending state, and last
  analysis/cache measurements, plus separate private recovery progress and phase measurements.
- `GET /api/view` returns a bounded overview, reconciled selection, and selected source
  from one snapshot. File and declaration identities are opaque query parameters.
- `GET /api/source` supports an explicitly pinned snapshot and file identity. Unknown
  or expired snapshots fail explicitly.
- `GET /api/events` adapts the native subscription to SSE. Every connection starts at
  current state; slow clients can skip intermediate metadata without an event log.
  View requests can also pin their snapshot, returning 409 if it has expired.
- The diagnostic host accepts JSON lines carrying an id and request URI, and returns
  the HTTP status, content type, and text body from those same handlers.

The browser consumes versioned status notifications and reloads a pinned view on
publication or user action. Opaque request/connection tokens reject obsolete results;
new notifications coalesce while a fetch is pending. Fetch failure or stream loss
initiates bounded reconnection, without a periodic status poll. Explicit stream closure
unblocks active HTTP responses during shutdown. See the [notification contract](CHANGES.md).
Source lines and [declaration controls](OUTLINE.md) are virtualised; selected-source
transport uses [pinned windows](WINDOWS.md), with summary metadata in the linked overview. Map geometry is supplied by Rust,
with visible-item painting and hit testing in the host adapter.

The browser [navigation module](NAVIGATION.md) stores at most 50 location records without
source bodies or snapshot leases. Request intent passes through the controller's pinned
loads/retries; a return can reject a missing-file fallback before publishing a view.
The inspector captures/restores its viewport, and changed-context returns explicitly
reveal current source rather than reuse old coordinates. Browser history contains only
opaque session/visit markers. These are host navigation concerns, independent of SCM.

## Rendering and validation scope

The initial browser uses DOM controls and a Canvas2D reference overview. This validates
coordinated views and the transport contract without settling the production renderer.
Rust/Wasm with wgpu remains the leading GPU candidate. Playwright exercises the actual
release host over loopback, with software-rendered headless and hardware-enabled headed
Chromium measurements. These qualify a bounded Canvas2D reference workload, not the
production renderer or the 16 GB laptop baseline. HTTP handlers are also tested in
process; stdio supports socket-free diagnostic use. See the
[browser validation procedure](../../experiments/browser/README.md) for reproducible
checks. Browser Wasm, WebGPU/WebGL comparison, and broader hardware qualification remain
follow-up work.

The UI uses a cool light workspace palette, blue selection, amber partial analysis,
and green change feedback. The overview is the primary visual object; source and its
outline stay adjacent. System-local sans and monospace fonts keep the prototype offline.
