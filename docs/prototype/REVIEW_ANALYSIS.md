# Review analysis integration reference

Admitted review source can now use the same analyser execution as the live workspace.
`ReviewCandidate::analyze` applies an `Engine`'s frozen registration/configuration plan
to both held sides before publication. `Host::analyze_review` uses that host's exact
engine, cancellation flag, cache and worker allocation. No comparison transport or
browser interface is added by this slice.

## Shared execution and allocation

Live indexing, periodic recovery and review invoke one native `analysis_jobs` runner.
It creates worker-local parsers lazily, validates returned context/content/ranges,
uses the existing content/context-addressed cache, applies shared input admission,
and reports cancellation or worker failure. Raw adapters do not execute project tools.
Every worker is joined before errors propagate, so simultaneous parser panics return
an engine failure without an unhandled scoped-thread panic.

Each engine owns an aggregate analysis-worker gate, configured by the existing 1–32
worker limit. Foreground work takes at most the currently available allocation. Review
requests one background worker; periodic recovery and review share a single background
allocation, so they cannot independently consume background pools. With multiple
workers, foreground indexing can use the remaining capacity; with one worker the work
serialises. Waiting for capacity checks cancellation outside the metadata lock and
uses timed condition-variable waits. No fairness or hard latency deadline is promised.

The gate is held until all workers in the attempt finish and releases on success,
error or unwinding. `Engine::analysis_workers` / `Host::analysis_workers` report
capacity, active allocation, background ownership and peak allocation. These count
analysis workers, not capture IO, all process threads or process CPU consumption.

Review shares the existing `Admission` object at recovery priority. Foreground
headroom/per-file refusal produces explicit deferred analysis without stale facts,
while captured source remains available. Deferred results are not cached. Cache
failures remain warnings/misses through the common runner.

Review does not lease mutable live syntax-retention state. Baseline and working
source may use the same path with different contents; their analysis must not replace
the foreground tree-retention entry or reconcile its path set. The immutable analysis
cache remains shared because facts are content/context bound.

## Selection, source and plan identity

The engine's source policy selects Rust/Python and explicitly configured raw C/C++.
Other captured review paths remain source-only with `OutsideAnalysisScope`, including
documentation, analysis exclusions, unsupported path representations and LPC inputs
outside selected C-family scope. Existing unassigned-header/dialect/analyser reasons
remain distinct. Byte inventory and analysis coverage do not have the same scope.

Analysis consumes the already admitted `CapturedSource` owners. It never rereads a
working file to satisfy a held side, copies a body into another charged input, or
transfers raw snapshot/source owners to query callers. Rebinding checks that every
document/path is retained and each source owner is the exact original owner. Analysed
views are destroyed before the raw input's charge is released.

The review snapshot policy includes the full frozen capture-plan identity, including
analyser descriptors/configuration. It uses a review-specific policy namespace and
does not claim the live capture-plan compatibility marker: review can retain text
outside the live discovery scope. Such a snapshot cannot become a partial live-index
baseline merely because its workspace matches.

Create analyses both sides sequentially under the same plan. Refresh reuses the exact
held baseline observation/facts and analyses only the newly captured working side.
A different plan requires a new comparison. A source-only pair cannot silently gain
a new analysis plan through working refresh; create a new comparison instead.

## Failure and publication

Calling analysis makes successful analysis a publication requirement. A failed or
cancelled analysis attempt cannot fall back to source-only publication implicitly.
Both analysed views are installed in the private candidate together, after validation;
errors, stale tickets or cancellation preserve the previous active pair and its source.
An abandoned candidate releases its ordinary ownership/admission guards.

Deferred coverage is an explicit successful observation and may be published. A held
baseline retains that coverage on refresh; a new comparison retries both sides when
capacity is available. Working refresh retries the working side under the same plan.
This slice does not add automatic review retries or background fact upgrades.

`ReviewLease::analysis_plan`, `classification` and `analyses` expose frozen native
metadata. Existing bounded source/outline windows now contain lexical declarations on
the selected side. A host must page/truncate metadata before serialising a review
response and reject obsolete UI responses; borrowed native metadata is not a wire schema.

The [validation report](../../experiments/review-workflow/ANALYSIS_RESULTS.md) records
mixed-language/context/cache checks, admission/failure tests, concurrency evidence and
the frozen review variants. Compilation-backed semantics and relationships remain
later work. The [comparison API](COMPARISON_API.md) now exposes native queries;
the initial [review UI](REVIEW_INTERFACE.md) has controller/native validation and
pending browser acceptance.
