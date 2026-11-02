# Parser responsiveness delivery plan

The [full-repository qualification](../../experiments/qualification/RESULTS.md) found
request-to-completion cancellation delays of 845 ms for Linux and 2,265 ms for PyTorch.
The [outline/edit follow-up](../../experiments/qualification/RESPONSIVENESS_RESULTS.md)
removes DOM growth and repeated metadata scans; large-file edits still take seconds.
This plan separates interruption from persistent syntax-tree ownership.

## 1. Cooperative analysis cancellation

Implemented: [contract](../prototype/ANALYSIS_CANCELLATION.md) and
[validation/measurements](../../experiments/qualification/CANCELLATION_RESULTS.md).

Extend the portable analyzer interface with an optional-result cancellable operation.
Keep the existing complete-analysis operation for callers that need it. Legacy adapters
receive cancellation checks before/after their complete operation; built-in adapters
also check during Tree-sitter parsing and declaration extraction. Cancellation produces
no analysis or partial coverage record. Reset interrupted parsers before another input.
Never persist partial facts or publish an incomplete snapshot.

Use the dependency's safe progress callback, without new unsafe code or global state.
Poll during traversal, scope ascent, and identity finalisation with a small operation
cadence to limit per-node polling overhead. The callback is cooperative;
grammar operations, hashing, decoding, allocation, validation, and filesystem I/O do
not acquire a hard time bound. The upstream parser also reports callback gaps during
pathological recovery; do not claim a deadline guarantee.

Tests must cancel inside parsing, reuse the same parser for different source, cancel
during fact extraction, compare complete/cancellable facts for all four languages, and
check native cancellation does not populate the cancelled file's analysis cache or
replace pinned observations. Qualification wrappers must forward the cancellation
operation rather than silently restoring whole-file cancellation. Measure actual
request-to-worker-return time separately from the query loop's observation cadence.

## 2. Bounded hot-file trees

The retained-tree portion is implemented as opt-in owned tree/source leases, with
worker-local parsers and a shared admission cache. Whole-process working-memory
admission remains open. See the [retention contract](../prototype/SYNTAX_RETENTION.md),
[implementation plan](syntax-retention.md), and
[measurements](../../experiments/retention/RESULTS.md). The following constraints remain
part of that interface:

- Identify entries by workspace, file, and complete analysis context; verify the exact
  retained source before deriving byte/point edits. An event path is only a hint.
- Lease a session to one worker without holding the admission lock while parsing.
  Concurrent refreshes must never edit the same tree or deadlock on a saturated pool.
  Analyzer trait objects remain worker-local without a `Send` requirement; only owned
  syntax state transfers between refreshes.
- Share one retained-state policy across languages, including active leases, old/new
  source overlap, tree weights, and eviction. Parser scratch space and ordinary capture
  buffers remain outside this cache quota; a unified memory policy is separate work.
  Source bytes/node counts are weights, not exact native heap measurements.
- Reset after interruption; keep or discard the old tree deliberately. Superseded work
  may warm a private cache only if the next use validates source/context again.
- Preserve immutable facts, unique/ambiguous entity rules, and clean-capture equivalence
  across Unicode/multiline edits, deletion, rename, recovery, and cache eviction.

Tree-sitter exposes descendant counts but no exact per-tree allocation measurement in
the current safe interface. The current weights have exploratory calibration and
repeated-edit measurements through production extraction. Expand working-set and
baseline-hardware qualification before choosing a default; faster parsing alone does
not bound fact extraction, source transfer, or total edit latency.

## Sources

The [Tree-sitter parser reference](https://docs.rs/tree-sitter/0.27.1/tree_sitter/struct.Parser.html)
documents safe progress callbacks, edited old-tree inputs, and reset after interruption.
The [upstream callback-gap report](https://github.com/tree-sitter/tree-sitter/issues/6067)
concerns pathological TypeScript recovery, not a measured failure in our four grammars;
it limits any general hard-latency claim.
