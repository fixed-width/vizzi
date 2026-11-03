# Captured source read and backing reference

Captured identity, byte length, encoding and line coordinates are immutable metadata.
Data access is fallible and independent of project paths. Resident memory remains the
default; private spooling is opt-in. No unsafe mapping or new dependency is introduced.

## Portable access

`CapturedSource::read_all(limit)` returns a `SourceInput` guard over exact verified bytes.
Resident input is borrowed without copying; backed input owns one load. `bytes`, `text`
and `content` on that guard describe its verified input. The guard's bytes cannot be
mutated or constructed by callers. `read_text(limit)` returns borrowed/owned UTF-8 text,
`None` for captured unsupported encoding, or a typed error. IO failure never becomes
absent/empty text.

`reader().read(range, limit)` enforces captured bounds and an output allowance. Empty
ranges at EOF work without backing IO. Backed reads verify 64 KiB integrity blocks captured
with the original bytes. A session caches at most two blocks, evicting before allocating
a replacement. Output and aligned verification scratch are distinct: a small range can
read whole integrity blocks. Per-window row/output limits also remain in force.

Whole backed loads additionally check the complete content identity. Unsupported bytes
are available to analysis as bytes; existing textual preview limitations remain explicit.
Summary views, layout, line counts and declaration positions use metadata, not full loads.
CRLF terminators are tracked in compact bits so line trimming does not read backing.
UTF-8 fragment flooring preserves existing byte-column/continuation semantics.

`resident_bytes`, `resident_text`, and `Document::resident_source` are explicit diagnostics
for resident data. Absence is not an encoding decision. Production source inspection
uses fallible reads. `Snapshot::view` and `Document::window` now return results; in-memory
fixtures and native callers must handle their errors.

`Document::capture_loaded` validates facts against the same verified `SourceInput` used
for parsing/cache validation and rejects an input with another content identity.
`capture_source` remains a resident convenience or metadata-only binding operation;
backed facts with spans require loaded input. Source class/context/content validation,
coverage rules and immutable observation identity remain independent of representation.

## Native analysis and HTTP

The engine acquires its optional input-admission permit before loading bytes. It holds
the permit across source load, cache/parsing, fact validation and cache storage; loaded
bytes drop before the permit. Cache hits use that same input for range validation, and
errors/cancellation release both. Deferred analysis needs no full backing load. Syntax
retention remains separately charged and owns its own source/tree state.

Source handlers run blocking reads/projection work outside Tokio's async executor and
the publication lock. A process-wide four-operation semaphore bounds concurrent source
work. A cancelled HTTP request cannot interrupt arbitrary blocking IO, but its worker
keeps its permit until work ends. This is not a response-memory or hard latency cap.

Bulk view/source inspection has a 32 MiB source-load allowance. Larger source remains
reachable through pinned windows. JSON/fact serialization costs are separate from this
source allowance. HTTP results are:

| Condition | Response |
| --- | --- |
| Malformed typed HTTP query | 400 |
| Unknown/expired snapshot or wrong document pin | Existing 409/404 rules |
| Bulk source allowance exceeded | 413 |
| Concurrent source work capacity occupied | 429, retryable |
| Backing IO, short read, integrity or internal range failure | 503 |

Core read errors remain typed (`InvalidRange`, `Capacity`, `ShortRead`, `Integrity`,
`Io`) for native/editor callers. HTTP query ranges retain existing clamping; an internal
invalid range indicates unavailable backing/projection state, not unsupported encoding.
All successful replies remain bound to the requested snapshot/file/version.

## Private session spool

`SourceSpool::new(directory, capacity)` creates private anonymous committed and staging
files. `persist` appends resident captured bytes and moves metadata onto a read-only descriptor.
The descriptor exposes no project filename or writable file handle. Concurrent readers
use serialized seek/read operations, so file-cursor sharing cannot mix captured ranges.
Already-backed sources passed to `persist` keep their original backing.

`capture` reads actual project input through EOF in requests of at most 64 KiB. It builds
identity, encoding, line/CRLF and integrity metadata from private staged bytes with bounded
scratch, preserving the existing identity algorithm. UTF-8 validation carries at most
three incomplete bytes between chunks. A matching previous capture keeps its original
source and backing, even at full committed quota. Changed captures append staged bytes
in bounded chunks before returning an immutable descriptor. A separate writer gate
serialises staging/commit work; old sources remain readable between committed-file IO
operations. Capture and reads use no publication lock.

`Directory::with_spool` rejects a spool inside the analysed root. CLI
`--source-spool-mib N` creates it in the required external cache directory; `0` disables
it and is the default. Capacity conversion is checked. This capacity bounds cumulative
committed session appends, including old/cancelled captures, rather than live-source bytes.
The reusable staging file is separately bounded by the same capacity, so combined file
lengths can reach twice the configured value. Unchanged verification needs staging space
but consumes no committed quota. Failure aborts capture, preserving the last published
observation and dirty hints. There is no
per-version disk reclamation or automatic compaction in this adapter.

Observed cancellation/read failure discards staged candidates; incomplete append tails
are truncated without changing earlier committed ranges. Cleanup is best effort if
truncation itself fails. Cancellation after commit may leave an unreferenced committed
capture. Blocking IO, mutex waits and cleanup have no hard cancellation deadline.
`SpoolStats` reports committed bytes, stores, backing reads/bytes, current and peak staging
length and maximum input read request. These observations are separate from resident memory.

Backing lifetime follows its directory/store handle and all captured descriptors. Dropping
the engine, deleting/rewriting project files, or expiring publication history cannot
change a retained query's bytes. The final store/descriptor release closes the anonymous
files. This is ephemeral session storage, not crash recovery or a chosen durable engine.

Default resident capture reads one complete file into a transient vector before hashing
and indexing. Opt-in spooled capture bounds transient input, metadata scanning and append
byte buffers independently of file size. A metadata scan uses a 64 KiB buffer and, only
across incomplete UTF-8 boundaries, a join of at most 64 KiB plus three bytes. Staging and
append buffers are released between those phases. This does not bound index/integrity
metadata, parser scratch/facts, syntax retention, response allocations, allocator retention
or whole-process RSS.
`SourceStorage` adds CRLF terminator and integrity-digest capacities to its accounting.

See [reader validation](../../experiments/source-reads/RESULTS.md),
[streaming capture measurements](../../experiments/stream-capture/RESULTS.md), and the
[capture design](../plans/streaming-source-capture.md). Reclamation and a calibrated
resident/process envelope remain subsequent work.
