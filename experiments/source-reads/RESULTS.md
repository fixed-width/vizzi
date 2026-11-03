# Verified source reads over private immutable backing

Captured source now supports fallible verified full-input guards and bounded range
sessions over resident bytes or private immutable backing. Summaries and line/declaration
positions use metadata. Native analysis holds its input permit through one verified load,
cache/parsing and fact validation; source inspection reports capacity/IO/integrity errors
instead of empty text or unsupported encoding.

With the opt-in backing adapter, the whole-source Linux history stress run's sampled
peak RSS changes from **9.156 to 2.339 GiB**, and PyTorch from **1.163 to 0.318 GiB**.
Old pins, source bytes, sharing, release and restoration checks pass. Capture/validation
and disk IO cost increase. This is the first ephemeral backing adapter, not a general
resident/process-memory cap or a change to default capture.

The [reference](../../docs/prototype/SOURCE_READS.md) defines guards, integrity blocks,
bulk/window limits, HTTP errors and spool lifetime. The
[delivery plan](../../docs/plans/bounded-source-reads.md) records the architecture.
The subsequent [streaming-capture report](../stream-capture/RESULTS.md) replaces the
whole-file temporary vector in opt-in capture; measurements below describe this earlier stage.

## Reader and native integration

The portable interface preserves content identity, byte length, encoding, line positions
and snapshot/document versions independently of backing representation. Resident loads
borrow existing bytes; backed loads own one verified input. Backed ranges validate captured
64 KiB integrity blocks with a two-block session cache, plus bounded output. Whole loads
also verify the complete content identity. CRLF terminators use compact metadata bits,
so line trimming needs no backing IO. Unsupported encoding stays a separate metadata fact.

`Document::capture_loaded` validates facts against that same input and rejects one from
another source identity. Native cache hits and parsing share the verified input; metadata
or deferred documents need no full load. Explicit resident diagnostics replace ambiguous
borrowed-only source access. Core view/window operations return typed read failures.

The private native spool uses one anonymous append-only file with synchronized seek/read,
so concurrent reads cannot mix file cursors. Captured descriptors keep it alive after
engine/store handles or project files are dropped. Existing safe IO APIs are used; no
unsafe mapping is introduced. Directory capture rejects a spool in the analysed root.
`--source-spool-mib N` enables a cumulative session disk allowance in the external cache
directory; `0` remains the default.

HTTP source/projection work runs outside the publication lock and async executor, with
four concurrent blocking operations. Bulk source loading has a 32 MiB allowance (413 on
excess); complete logical access remains through pinned windows. Busy source work is 429;
backing/short/integrity/internal range failures are 503. Malformed typed HTTP queries keep
400 and binding/expiry keep existing 404/409 rules. No read error becomes absent encoding.

## Current-code storage comparison

The [procedure](README.md) repeats the source-storage workflow on marked owned copies of
the same pinned scopes. Each profile has one resident and one 16 GiB session-spool process,
four whole-source prefix generations, a held expired baseline query handle, restoration
and release. These source-storage snapshots omit facts/parsers intentionally; they are
not end-to-end indexing benchmarks. Both modes use current code, CRLF and integrity metadata.
Original archive targets remain unchanged and all restored snapshot/source manifests match.

The i7-13700H reference host exposes 20 logical CPUs and about 62.5 GiB RAM, with Rust
1.99.1. Storage children use a 12 GiB virtual address-space limit. No competing builds,
tests, mutation jobs or benchmarks run during timing. OS caches are warm. Single sequential
processes with different UUID prefixes are observations, not randomized speedups or tails.

| Storage observation | Linux resident / backed | PyTorch resident / backed |
| --- | --- | --- |
| Sampled process peak RSS | 9.156 / 2.339 GiB | 1.163 / 0.318 GiB |
| Baseline owned buffers | 1.807 / 0.414 GiB | 0.226 / 0.053 GiB |
| Fourth version plus external pin, owned buffers | 9.085 / 2.079 GiB | 1.139 / 0.266 GiB |
| Baseline capture/validation phase | 3,481 / 3,905 ms | 523 / 535 ms |
| Unchanged phase | 1,436 / 1,463 ms | 236 / 217 ms |
| Fourth changed capture/validation phase | 3,817 / 7,611 ms | 606 / 974 ms |
| Session bytes appended | N/A / 9.018 GB | N/A / 1.124 GB |

Backed snapshots have zero permanent resident source-byte capacity; indexes, CRLF flags,
chunk digests and document metadata remain resident. Buffer totals cover source/index/
integrity capacities and deduplicate actual source allocation identity, excluding object
and allocator overhead. They do not equal RSS or live physical pages. All tracked handles
die after release; allocator retention can keep RSS high afterward.

Storage elapsed phases include byte-for-byte prefix validation, which performs verified
backing loads. These extra integrity/validation reads contribute to the reported cost;
the table is not a pure capture-throughput comparison. Probe source rewriting, inventory
collection, final pin checks and restoration also contribute to process peaks. The Linux
spool reports 27.080 GB read, PyTorch 3.376 GB, including those probe checks. No claim is
made that production capture requires every one of those additional reads.

The disk allowance is cumulative, not live-source usage. Old/cancelled successful captures
consume append capacity until the final store/descriptor is released. There is no compaction
or per-version reclamation. A full spool fails new capture safely while old pins remain
readable. Private backing is session storage, not crash recovery or a durable engine choice.

## Admitted workflows over backing

The structural/paste/unseen/batch workflow repeats with four workers, 512 MiB retained
syntax, periodic recovery, and 32 MiB Linux / 12 MiB PyTorch input admission. Fact caches
are prepared outside timing for the changed analyser fingerprints. Each probe child has
an 8 GiB virtual address-space limit. Source/fact/settlement/restoration checks all pass.

| Write to matching published source | Linux backed | PyTorch backed |
| --- | --- | --- |
| Structural edit | 74.7 ms | 45.6 ms |
| Large paste | 141.8 ms | 61.1 ms |
| Unseen file | 905.7 ms | 2,007.0 ms |
| Three-file batch | 914.8 ms | 1,719.8 ms |

The paste exceeds the per-file limit and stays explicitly deferred, with readable backed
source. Linux initially defers two batch inputs and PyTorch one; all eligible inputs later
complete through recovery without another edit. Timings end at source publication, not
completed-analysis/browser presentation. Recovery is observed before writes; a sample does
not prove every IO/parse ordering. Whole-snapshot counts can include a prior oversized paste.

Peak RSS including probe verification is 3.231 GiB Linux and 1.287 GiB PyTorch. Source
verification constructs independent resident capture buffers and fresh parses, so these
values are not isolated production memory. These virtual-address-space limits do not
qualify physical RAM, combined editor/browser/OS memory, or the 16 GB baseline laptop.

## Correctness and mutation evidence

Reader and backing tests first failed before their interfaces existed. Coverage includes
resident zero-copy reads, exact loading/range/spool limits, wrong ranges, empty/unsupported
source, corrupt and short backing, IO failure, UTF-8 block boundaries, multi-line CRLF
bits, long-line fragments, two-block cache behaviour, loaded-input fact validation,
concurrent file cursors, source deletion/rewriting and old-handle lifetime. HTTP tests
exercise metadata-only summaries with broken backing, explicit 503 failures, and both
sides of the 32 MiB bulk boundary while windows stay accessible. Native browser coverage
runs deferred/read/resume/pin checks with spooling enabled.

Formatting, **230 Rust tests**, doctests, Clippy and **58 Node/browser checks** pass.
Complete affected native/core mutation scopes generate a final **522 unique outcomes:
402 caught, 105 unviable, 15 timeouts, zero survivors**. Passing unmodified baselines
precede interpretation. The aggregate combines unchanged broad-run modules with complete
updated native and core module scopes; one CRLF allocation-overgrowth survivor is replaced
by its verified focused rerun after adding a compact-bitset bound. Overlapping outcomes
are not double-counted.

Development gaps included CLI flag/conversion, exact capacity acceptance, IO accounting,
compact CRLF sizing and byte-boundary Unicode labels. A redundant UTF-8 floor bound was
removed using the verified-text/lookbehind invariant rather than adding an exclusion.
HTTP invalid core ranges are internal read failures; query ranges remain validated/clamped.
The existing equivalent engine directory-prefix exclusion is unchanged; no new exclusions
were added. Timeouts break bootstrap/capture/read-loop progress and are separate from
assertion catches; unviable substitutions are compiler/build failures.

Grammar/dependency internals, serialization/hash macro bodies, optional benchmark scripts,
Python, allocator/filesystem internals, JS/CSS painting and browser internals are outside
native mutation scope. Final reader/projection reruns use core tests; workspace/native HTTP
and browser checks separately exercise the integrated final implementation. Coverage and
zero survivors are evidence, not proof. No dependencies or unsafe-code exceptions were added.

## Remaining limits

The default stays resident. Opt-in capture still transiently reads one entire project file
before hashing/indexing/spooling; it is not streaming capture admission. Resident metadata,
parser scratch/facts, retained parser arenas, syntax state, response serialization and
allocator retention need their own limits. The session spool needs reclamation/disk policy
before becoming a production storage decision. A bounded input permit or private backing
alone does not impose a whole-process memory cap or hard cancellation deadline.

[Raw observations](results/observations.json), [summary](results/summary.json),
[storage CSV](results/storage-summary.csv), [mutation details](results/native-mutation.json),
and [source hashes](results/source-sha256.json) preserve measured code/binary identity,
original inputs, configuration, verification and non-caught outcomes.
