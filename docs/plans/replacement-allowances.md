# Reserve replacement state from observed retained weight

The [working-set investigation](../../experiments/working-set/RESULTS.md) shows excess
reservation pressure beyond the real old/new overlap requirement. Implement a small
policy change inside the existing cache, keeping ownership, admission and verification
at the same interface. Do not add host controls or change the disabled default.

## Policy

A key without retained state still requests the configured entry maximum. For a known
compatible state with weight `W`, choose a replacement allowance of
`min(entry maximum, W + max(W / 4, 4,096))`, using integer division. The quarter-weight
or 4 KiB growth margin is provisional engineering headroom, not a measured upper bound
on syntax growth or allocation. This stage uses only observed state already owned by
the cache; it adds no unbounded demand history or source-size predictor.

Charge the full old weight plus that exact selected allowance before analysis. Keep
active reservations unevictable. Evict idle entries only as needed for selected weight
and the existing count limit; busy keys or unavailable quota still bypass immediately.
Expose the selected allowance through the lease and enforce it when state is returned,
even when a third-party analyser ignores the requested limit. Release the exact old/new
reservation on finish, invalidation, cancellation, error or unwinding.

An unexpectedly larger result remains valid analysis but need not retain a replacement
tree. Built-ins already discard over-limit state before copying retained source. Once
that old lease is consumed, the next targeted attempt has no previous state and can
try the full entry allowance again. No retry parse is added to the current attempt.
Context/content checks, full-scan promotion rules, LRU ordering and parser ownership
remain unchanged. True old/new overlap can still require eviction or bypass.

## Validation and delivery

Use TDD for smaller known-state reservations, new-key maximum allowance, both headroom
branches, maximum clamping, growth within/beyond allowance, a malicious over-allowance
return, next-attempt recovery, exact charge release, invalidation, and simultaneous
replacement leases. Compare complete analysis and immutable snapshots with fresh
capture, including source growth and multiple languages.

Run formatting, all-feature workspace tests and doctests, Clippy, browser checks, and
complete production cache mutation testing. Keep mutation failures/timeouts separate
from assertion catches. No dependency or unsafe-code changes are required.

Repeat the existing 20-step Linux/PyTorch working-set matrix with disabled, 512 MiB and
1 GiB conditions. Record source/binary identity and verify every step independently.
Keep these timings separate from correctness and mutation jobs. Compare reuse,
eviction, bypass and charge against the recorded fixed-reservation conditions; timing
observations are not randomized speedup or tail estimates. Preserve baseline-laptop,
parser scratch/input admission, and whole-process memory limits as unresolved work.

## Delivery evidence

Implemented within the existing cache interface and analyser contract. The
[follow-up report](../../experiments/allowances/RESULTS.md) records passing source-growth
and charge tests, full cache mutation results, all 120 clean-capture checks and 126
independent changed-file analyses. Both 1 GiB batches now reuse all three trees without
bypass. At 512 MiB, excess bypass is reduced but first-use and true overlap pressure
remain; PyTorch's final header edit stays cold. Keep margin calibration and broader
input/process admission as follow-up work, with the disabled default unchanged.
