# Why a retained syntax tree can become unavailable

A codebase can have fast repeated edits in one file while a return to another file
requires a complete parse. The source observation, persisted analysis, and editable
syntax tree have different ownership and validity rules.

Published documents hold immutable captured source and facts. Their previous versions
remain usable through retained handles. The persistent analysis cache can reuse facts
for identical content and context, but a changed comment still changes content identity.
It does not persist an editable tree. Private syntax retention supplies that separate
state only when its workspace, path and complete context match and the cache grants an
exclusive lease. The state's own exact source determines how to edit its tree.

## Reservations create a working-set cost

The initial policy reserves the maximum new-state allowance for every admitted parse.
The old state remains charged while its replacement is built. If the aggregate charge
would exceed the budget, the cache first evicts least-recently-used idle entries.
Active reservations cannot be evicted. If they leave insufficient room, the request
falls back to ordinary analysis immediately.

For example, let the total budget be twice the per-entry allowance. A large idle tree
weighs one allowance, and another idle tree is small. Their actual retained weights
fit comfortably. A new tiny-file request nevertheless reserves one whole allowance.
Keeping the large tree, small tree and new allowance would exceed the budget, so the
oldest idle tree can be discarded. The tiny replacement may eventually use very little
weight; that does not retroactively restore the discarded tree.

This protects the old/new admission accounting chosen for the first implementation.
It also means that idle working-set weight alone does not predict whether a tree will
survive another edit. Concurrent parsing can run out of reservation room even with
fewer entries than the count ceiling. Parser scratch space and ordinary source buffers
remain outside this accounting; falling back to a full parse is not a process-memory
limit.

## What observations can establish

Per-key inspection can show an absent, idle or leased entry without changing its
recency. Serial before/after observations plus an eviction counter can identify a
state lost while another file is admitted. A concurrent batch can show admission
pressure and resulting availability, but its endpoint observations do not identify
the order in which workers leased or returned state. A busy key and a quota bypass
also share an aggregate counter in the current cache.

The [working-set measurements](../../experiments/working-set/RESULTS.md) exercise these
cases on the reference repositories. They complement the
[recovery browser report](../../experiments/recovery/RESULTS.md), whose lack of per-key
observations prevents reconstructing the exact cause of its cold Perfetto edit.
A larger quota is a diagnostic comparison, not evidence for changing the default.

A future admission policy can reduce unnecessary reservation pressure by choosing a
smaller replacement allowance when justified by observed demand. It must still account
for old/new overlap, release on cancellation, reject over-allowance returns, and fall
back correctly when source growth or a different file shape exceeds that allowance.
Choosing and validating those allowances is separate from enforcing limits on parser
inputs, scratch space, or the whole process. The
[retention reference](../prototype/SYNTAX_RETENTION.md) defines the current contract;
the [investigation plan](../plans/working-set-admission.md) records the decision criteria.
