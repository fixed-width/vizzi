# Admit analysis work without hiding captured source

Implement the next resource stage after [replacement allowances](replacement-allowances.md).
Captured source is durable observation storage, independently of active analysis. The
current in-memory source model does not permit a job scheduler to enforce a whole-process
memory cap. Preserve that distinction rather than labelling an input-byte quota as RAM.

## Scheduling contract

Use one explicitly shareable native admission object across foreground and recovery
attempts, languages, and engines configured with it. An analysis permit charges actual
captured input bytes (one unit for an empty file), before cache loading/parser creation.
It releases on all completion, cancellation, error, and unwind paths. Immutable complete
in-memory document reuse requires no new permit. Active work is bounded by total admitted
input bytes; it is not an estimate or hard bound on parser scratch, facts, source snapshots,
retained trees, allocator retention, or total memory.

Limits include positive total/per-file input bytes and a foreground reserve smaller than
the total. The per-file limit must fit the recovery share (`total - reserve`), ensuring
any admitted size can eventually run during recovery in a quiet host. Foreground work
can use the total; recovery input charges cannot exceed the recovery share. This reserves
headroom for ordinary foreground work without preempting an active parser. New work
never waits for a permit: an oversize or busy request publishes explicit deferred analysis
for its freshly verified source. No partial or old facts are attached to that new source.

Admission is opt-in initially. `for_budget` selects a quarter of input capacity for
foreground headroom and the remaining three quarters as the per-file limit. Expose one
CLI input-budget setting, not a claimed process-memory target. Native callers can share
custom explicit limits. Host refresh roles select foreground/recovery priority through
the same engine implementation. Existing parser-worker and retained-syntax quotas remain
independent and enforced.

## Deferred observations

Add explicit `deferred` coverage to portable analysis. Deferred records contain no facts;
source classification, encoding, context and content identities stay truthful. Capture
and pinned source windows remain available. Overview, source inspector and measurements
identify deferral with its reason. This is distinct from syntax recovery, unsupported
encoding and source-only language selection.

Deferred results are never persisted or accepted from the analysis cache and are never
reused as complete in-memory analysis. A later full verification or hint retries them;
permanent per-file limits stay explicitly deferred until configuration/input changes.
Normal latest-ticket publication and source-hash rules apply. Resource decisions can
make intermediate observations differ; compare completed facts/source independently,
and require final clean agreement once contention has settled or adequate limits apply.

## Measurement and verification

Use owned Linux/PyTorch copies for structural edits, a large paste, a new file and a
three-file batch. Observe recovery before writes, retain separate foreground/recovery
metadata, sample capture/analysis and process memory, and verify fresh facts for completed
analyses plus exact captured source for every file. Compare unconstrained/admitted modes.
Run child processes under a documented address-space limit where available; that does
not qualify physical RAM or the 16 GB laptop. Source/capture storage remains separately
reported and unbounded by this first admission change.

Use TDD for exact limits, foreground reserve, shared concurrent permits, cancellation and
unwind release, oversize/busy deferral, truthful source access, immutable old pins,
cache rejection, retry without edits, all language coverage, browser display, and CLI
validation. Run workspace/doctest/fmt/Clippy/browser checks and complete affected mutation
scopes. Keep performance timing separate from validation jobs. No unsafe code or new
dependencies are needed.

## Delivery evidence

The [reference](../prototype/ADMISSION.md) and [workflow results](../../experiments/admission/RESULTS.md)
record implemented shared input admission, reserve/limit validation, deferred source
publication, cache exclusion, retry and browser presentation. All 16 source/scope checks,
22 completed changed-file fresh parses and four restored complete snapshots agree.
Linux/PyTorch workflows complete under child 8 GiB virtual address-space limits; oversize
pastes remain visibly deferred while eligible batch work later completes without edits.
Complete affected mutation scopes and native/browser baseline checks pass as detailed
in the report. Capture buffers, retained source/history and parser heap are explicitly
outside this first admission change; a real process-memory/capture budget remains separate
architecture work. Defaults remain disabled and no unsafe code/dependencies are added.
