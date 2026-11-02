# Bounded syntax retention implementation plan

Implemented, opt-in: [contract](../prototype/SYNTAX_RETENTION.md) and
[validation/measurements](../../experiments/retention/RESULTS.md).
This implements the next stage of [parser responsiveness](parser-responsiveness.md).
Parsers remain worker-local and disposable. Only an owned tree, its exact source, and
its complete context travel between refreshes. Published documents contain no mutable
syntax state. Existing analyzer implementations keep working without retention.

## Portable analysis

Add one state-aware analysis operation with an owned optional previous state and a
retention allowance. Built-ins derive a single replacement from the common byte prefix
and suffix of old/new captured source, expand to UTF-8 boundaries, and supply correct
byte and row/column edits to Tree-sitter. Retained source is authoritative for this
comparison, independently of filesystem hints or a published baseline's age.

State is opaque and nonserialisable. Full context mismatch discards it. Cancellation
consumes/discards the leased state, resets an interrupted parser, and returns no facts.
All extraction still runs on the new tree; unchanged subtrees do not imply reusable
declaration identities or positions. Complete results must match fresh analysis.

## Shared native cache

Use one explicitly shareable cache across languages and engines. Keys include workspace,
normalised path, and the complete context descriptor. A nonblocking lease removes an
idle entry and reserves its old weight plus the new-state allowance. A busy key or
unavailable quota falls back to ordinary analysis. No parsing, source copying, or tree
disposal runs under the cache lock. Drop releases reservations on cancellation/error.

Limit entries as well as admission weight. Evict idle entries in least-recently-used
order; active leases remain charged and cannot be evicted. Invalidation removes idle
state and prevents an active lease from reintroducing an invalidated entry. Validate
returned state against the requested context/content before admission. Promote targeted
edits; complete repository scans may reuse existing entries but do not populate a cache
with every cold-indexed file. Persistence hits never require parsing just to warm trees.

## Initial policy and evidence

Weight is `source bytes + 128 × descendant nodes + 4,096`, with checked arithmetic.
The source charge covers the cache's own source copy; the active reservation covers
old and new retained state together. Parser scratch space, ordinary job buffers,
immutable documents, and temporarily constructed over-limit trees remain outside this
cache quota. This is a retained-state admission policy, not an exact heap measurement
or a whole-process memory cap.

An exploratory isolated parse on this host found 807,655 nodes in Linux's 24.0 MB AMD
header, 1,935,897 in PyTorch's 8.4 MB Perfetto header, and 645,802 in `perfetto.cc`.
Resident increases with the tree alive and parser dropped were roughly 63, 164, and
57 MiB respectively; source was already resident. The proposed weights are about 122,
244, and 82 MiB including source. Allocator retention and scanner state prevent treating
that calibration as a proof of an allocation upper bound.

Start with an opt-in host configuration: 512 MiB total weight, at most 256 MiB per new
state, and eight entries. Keep a disabled configuration for comparison and conservative
deployments. Measure repeated first/last/interior and Unicode/multiline edits through
production extraction, resource observations, cancellation, and clean captures before
considering a default. The 16 GB laptop remains unqualified.

## Verification

Use TDD for source/context mismatch, full-result equivalence in all four languages,
Unicode and line-coordinate edits, recovery, deletion, rename, retention rejection,
cancellation/reuse, leases, eviction, invalidation, and shared quota under concurrency.
Run complete affected-module mutation tests and manually omit edit application if the
tool does not generate that deletion. Keep timing runs separate from correctness and
mutation jobs. Record admission weights and actual RSS separately.
