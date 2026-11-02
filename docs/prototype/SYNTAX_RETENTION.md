# Bounded syntax retention reference

Syntax retention is opt-in: `--syntax-cache-mib 512` enables weighted admission;
`0` disables it and is the default. This quota is not a native heap or process-memory
cap. Source capture and published observations keep their existing contracts.

## Portable state and analysis

`Analyzer::analyze_with_state(bytes, previous, retain_weight, cancelled)` consumes an
optional owned `SyntaxState`. Completed `AnalysisResult` contains analysis, optional
replacement state, and `tree_reused`. Interruption returns `None`. Existing adapters
default to cancellable analysis without state; the analyzer trait still needs no `Send`.

Built-ins retain an opaque tree, exact UTF-8 source, content hash, and complete context.
State has no public constructor, clone, mutable tree access, or serialisation. It can
transfer between workers; parsers stay worker-local. Published documents never reference
mutable syntax state.

Reuse requires the complete context to match. Built-ins compare retained/current source,
derive one replacement from their common prefix/suffix, expand to UTF-8 boundaries,
and apply byte and row/byte-column edits before parsing. Diff/position scans check
cancellation in 64 KiB chunks. Events and metadata are not input proof. State may be
newer or older than a published snapshot: its own exact source/context determine reuse.

The full new tree is traversed to recompute facts, ambiguity, and positions. Tree reuse
does not reuse declaration facts. Interrupted state is discarded; interrupted parsers
reset. Unsupported encoding or an exceeded allowance returns no replacement state.
Zero allowance permits consuming an old tree without retaining its replacement.

## Native admission and leases

`SyntaxCache::new(total_weight, entry_weight, entry_count)` requires positive values
and `entry_weight <= total_weight / 2`. `SyntaxCache::for_budget` supplies host policy.
`Engine::with_syntax_cache(Arc<SyntaxCache>)` shares one quota across all languages and
engines using that cache. Unrelated library-created caches have independent quotas.

Keys include workspace, captured path, and every context descriptor field. A lease
exclusively removes idle state and charges its old weight plus the new-state allowance.
Least-recently-used idle entries are evicted for weight/count limits. Active reservations
cannot be evicted. Busy keys and unavailable quota immediately use ordinary analysis.

Parsing, source copying, and tree disposal happen outside the admission lock. Drop
releases a lease on cancellation, validation failure, or worker unwinding. Returned
state must match content/context and fit its allowance. Known removals invalidate
entries; completed captures prune removed/reassigned paths. Invalidated active leases
cannot reintroduce old entries.

Targeted changes can promote new entries. Full scans reuse existing state without
populating the cache with a whole repository. Unchanged documents and persisted hits
do not parse just to warm trees. Complete superseded work may warm this private cache;
publication independently checks its ticket.

## Weights and observations

Weight is `source bytes + 128 × descendant nodes + 4,096`, with checked arithmetic.
Over-limit state is discarded before copying source for retention. Reservations cover
old/new retained state together. Parser scratch space, ordinary input buffers,
immutable documents/facts, temporary over-limit trees, and allocator retention remain
outside this quota. Admitted weights and entries are bounded; allocation is not exact.

For host budget `N` MiB, each new-state allowance is `min(N MiB / 2, 256 MiB)` and
the entry ceiling is eight. The [implementation plan](../plans/syntax-retention.md)
records calibration and the remaining baseline-laptop qualification.

`SyntaxCache::stats` reports configured weights, current/peak charge, idle entries,
idle source/node counts, active leases, evictions, and bypasses. Status `tree_reuses`
instead counts files receiving compatible old trees in that completed engine attempt.
They still count as `parsed`, separately from document reuse and persisted analysis hits.

`SyntaxCache::entry(&SyntaxKey)` observes one complete key: `None` for absent state,
`SyntaxEntry::Idle` with retained weight/source-byte/node counts, or `SyntaxEntry::Leased`.
Invalidated active reservations remain leased until released. Inspection does not lease,
promote, copy source/tree state, or change LRU order. Availability may change immediately
after return. This diagnostic interface does not alter admission or add host/status fields.
The [mixed working-set procedure](../../experiments/working-set/README.md) uses it to
distinguish isolated reuse from pressure across files. The
[working-set explanation](../explanation/syntax-working-sets.md) describes why fixed
replacement reservations can evict useful state.

The [Tree-sitter reference](https://docs.rs/tree-sitter/0.27.1/tree_sitter/struct.Tree.html#method.edit)
specifies matching byte and point edits before reusing an old tree. Allocation costs
and progress callbacks retain their dependency/grammar limits.
