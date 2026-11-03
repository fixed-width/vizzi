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

`CapturedSource::verifier()` returns a constant-space `SourceVerifier` over a captured
identity/length. `update(bytes)` feeds ordered actual bytes without retaining them or
reading backing. Consuming `matches()` at logical EOF requires the exact byte count and
matching existing namespaced, length-prefixed identity. Callers must feed the complete
input, including any suffix; reaching the expected count alone is not EOF. Oversized
input permanently rejects reuse. Verification accepts binary input independently of
captured text availability and does not rebuild metadata.

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

`SourceSpool::new(directory, capacity)` creates two private anonymous committed files and
one staging file. Each committed file has cursor/maintenance handles, giving five handles
per spool. `persist` stores resident captured bytes in an owned range and moves metadata
onto a read-only descriptor bound to its file index.
The descriptor exposes no project filename or writable file handle. Concurrent readers
use serialized seek/read operations, so file-cursor sharing cannot mix captured ranges.
Already-backed sources passed to `persist` keep their original backing.

Each file's cursor reads/writes and counters have a separate lock from allocation state.
Backing reads acquire only their file's cursor lock. Explicit maintenance serializes physical
cleanup, marks a file unavailable under allocation state, then releases that state before
metadata/truncation. It uses a safe cloned handle only for metadata and `set_len`, which do
not move the shared cursor. Existing reservations remain inside the selected cut; new
reservations use the other file. Completion restores availability and reduces charge only
after successful cleanup. Failure/unwind restores availability while retaining charge for
retry. Handles are fixed per spool, not per captured source. No unsafe IO is used.

Pinned reads and captures with aggregate headroom can proceed during physical cleanup.
No headroom or contiguous fit returns capacity refusal rather than waiting on that cleanup.
The OS/filesystem, another capture using staging, or metadata work can still delay IO;
diagnostics and other explicit maintenance may wait. No hard deadline is established.

`capture` reads actual project input through EOF in requests of at most 64 KiB and stages
all bytes. With a previous capture, it also verifies those actual input slices using its
captured length as a candidate hash prefix. Reuse requires observed EOF, exact length,
matching identity and cancellation checks; project metadata alone never proves equality.
A verified match keeps its original source/backing without scanning staged metadata or
appending, even at full committed quota. Otherwise it builds identity, encoding, line/CRLF
and integrity metadata from private staged bytes using actual length and bounded scratch.
UTF-8 validation carries at most three incomplete bytes between chunks. Changed captures
append staged bytes in bounded chunks before returning an immutable descriptor. A separate writer gate
serialises staging/commit work; old sources remain readable between committed-file IO
operations. Capture and reads use no publication lock.

`Directory::with_spool` rejects a spool inside the analysed root. CLI
`--source-spool-mib N` creates it in the required external cache directory; `0` disables
it and is the default. Capacity conversion is checked. This capacity bounds the reserved
aggregate charged spans of both backing files, including reservations, gaps and retired
tails. A cleaning file keeps its full pre-cleanup charge until IO succeeds.
The reusable staging file is separately bounded by the same capacity, so combined file
lengths can reach twice the configured value. Unchanged verification needs staging space
but consumes no committed quota. Failure aborts capture, preserving the last published
observation and dirty hints. Free ranges use best-fit allocation, split remainders and
coalesce adjacent retirements. Fragmentation can refuse a contiguous capture despite
sufficient total free bytes. Live ranges never move; compaction is not implemented.

Each backing descriptor owns its range. The final descriptor release queues retirement
using a separate memory-only lock, without acquiring the backing IO lock or writer gate.
Publication expiration alone does not free an externally pinned source. Capture/commit
drain retirements into allocation indexes before allocating, without physical cleanup.
`SourceSpool::reclaim()` provides explicit maintenance outside publication locks. It visits
at most both files once, cleans eligible tails and keeps allocation state free during IO.
Interior holes and retired tails in available files can be overwritten within their existing
charge. Captures never initiate physical truncation. File lengths can stay at a prior
high-water mark until explicit maintenance or final backing closure; no background schedule
is introduced. An unfinished copy retains its reservation during concurrent cleanup.

Observed cancellation/read failure discards staged candidates. Failed/cancelled copies
retire reservations into allocation indexes before returning; their physical space remains
charged until overwritten or explicitly cleaned. Neighboring live ranges remain intact.
Maintenance errors are reported and failed physical cleanup can be retried.
Staging cleanup remains best effort if truncation fails. Late cancellation may retire a
completed private capture; its space returns on subsequent maintenance. Blocking IO,
mutex waits and cleanup have no hard cancellation deadline.

`SpoolStats::written_bytes` remains cumulative committed bytes and can exceed capacity
after reuse. `allocated_bytes` includes in-flight reservations and queued retirements;
`extent_bytes` is the aggregate charged span, `free_bytes` is charged but unallocated space,
and `peak_extent_bytes` is the largest aggregate charge. `cleaning_files` and `cleaning_bytes`
identify temporarily unavailable files and their retained charges. Free bytes in a cleaning
file are not available for admission. `file_bytes` reports total actual file lengths or `None`
on metadata IO failure. Diagnostics are observational under concurrency, not an admission
transaction. `pending_retirements` counts queued descriptors. Store/read counters,
staging lengths and maximum input request remain separate. File lengths are not filesystem
block allocation, resident memory or process caps; index/retirement-queue memory also scales.

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
[verified reuse measurements](../../experiments/verified-reuse/RESULTS.md). The
[capture design](../plans/streaming-source-capture.md) and
[reuse design](../plans/verified-source-reuse.md) record the implementation rationale.
The [reclamation report](../../experiments/reclamation/RESULTS.md) and
[design](../plans/spool-reclamation.md) describe owned ranges and maintenance. Compaction,
metadata admission and a calibrated resident/process envelope remain subsequent work.
The [maintenance-read report](../../experiments/readable-maintenance/RESULTS.md) measures
concurrent prefix reads and documents the remaining filesystem/scheduling limits.
The [independent capture design](../plans/independent-capture.md) and
[comparison](../../experiments/independent-capture/RESULTS.md) cover allocation during cleanup.
