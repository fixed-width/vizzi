# Investigate retained syntax across mixed working sets

The [recovery follow-up](../../experiments/recovery/RESULTS.md) records a 1.34-second
Perfetto header edit without retained-tree reuse. That report cannot distinguish
absence, eviction, a busy key, or admission pressure. Investigate those effects before
changing resource policy or treating a larger quota as a solution.

## Measurement seam

Expose non-mutating per-key accounting on the existing syntax cache: absent, idle
weight/source/node counts, or leased. Inspection must not lease, promote, reorder,
copy, or expose mutable tree/source state. A point observation can become stale after
returning; it is diagnostic metadata rather than an admission guarantee. Keep aggregate
statistics and admission semantics unchanged.

A qualification-only owned-copy probe should retain one cache across a sequence of
single-file edits, a three-file concurrent batch, a full verification, ten ordinary
edits, and a final large-header edit. Compare disabled, 512 MiB and 1 GiB retention
in separate processes with four shared parser workers. Record target availability and
aggregate charge/eviction/bypass counters before and after every step. Do not sum
concurrent activity into invented per-file attribution.

Use the same pinned Linux/PyTorch scopes and selected files as the recovery report.
Require unique probe contents, prepared compatible analysis caches, fresh per-file
analysis agreement, complete-capture agreement after each step, and restored inputs.
Only disposable copies with a matching ownership marker may be edited. Measure wall
time and RSS separately from quota weights; verification contributes to process peaks.
No analysed project tooling, watcher, HTTP, or browser runs in this controlled probe.

## Decision criteria

Distinguish isolated hot-tree reuse, aggregate working-set pressure, reservation-induced
eviction, concurrent bypass, and invalidated/missing entries. A deterministic fixture
should demonstrate that reserving the full per-entry allowance can discard a large idle
tree even when the eventual small retained state would fit. Real profiles establish
whether that mechanism matters beyond the fixture.

Keep production retention disabled by default. A larger diagnostic quota does not
establish a laptop memory envelope. Choose the next admission change from these
observations; preserve old/new accounting, exclusive ownership, shared budgets,
nonblocking fallback, content/context validation, and cancellation. Per-file parser
scratch/input bounds remain a separate unresolved contract.

## Evidence and next design

[The completed measurements](../../experiments/working-set/RESULTS.md) retain all 120
clean-snapshot checks and 126 independent changed-file analyses. At 512 MiB, serial
PyTorch admission and the three-file batch lose Perfetto; the final edit is cold.
At 1 GiB it remains available. Linux retains its primary header while losing other
state. Both enabled budgets still bypass batch leases. A larger quota is not a policy
change or baseline-laptop qualification.

The next design should replace unconditional maximum reservations with bounded
replacement allowances justified by observed demand. Preserve strict accounting and
ordinary-analysis fallback when an allowance is exceeded. Test source growth,
previously unseen files, cancellation, concurrent replacement, and exact quota limits.
Do not imply that adaptive allowances preserve the whole PyTorch working set at 512 MiB:
observed old/new primary-header overlap alone exceeds that budget with the other states.
Capture/parse input admission and whole-process enforcement remain separate decisions.

The next stage is delivered through [replacement allowances](replacement-allowances.md),
which selects headroom from actual retained state without adding demand history or
changing the disabled default. Unknown state, unexpectedly large growth and true
old/new overlap retain conservative fallback behaviour.
