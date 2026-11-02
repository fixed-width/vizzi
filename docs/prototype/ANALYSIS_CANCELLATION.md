# Cooperative analysis cancellation reference

`Analyzer::analyze_cancellable(bytes, cancelled)` returns `Result<Option<Analysis>,
ModelError>`. `Some` contains complete facts for exactly the supplied source/context;
`None` abandons the attempt. Cancellation is not a coverage classification, an empty
successful outline, or an analysis error. No partial analysis is returned or persisted.
`Analyzer::analyze` remains the complete operation for noncancelling callers.

The predicate must be cheap, nonblocking, nonpanicking, and monotonic: once true it
stays true for the attempt. The native host checks its atomic refresh-ticket identity.
The default trait operation checks before and after complete analysis, preserving
existing adapters and their errors. Such adapters remain interruptible only around
their complete operation until they override this method.

## Built-in adapters

All four built-in grammars use Tree-sitter's safe progress callback. An interrupted
parse returns no tree; the adapter resets the parser before accepting another input.
Abandoned partial state is never resumed for a different file. Reset is worker-local;
there is no global cancellation flag or unsafe code in this implementation.

After parsing, traversal checks every 256 checkpoint operations shared across node
visits, scope ascent, declaration emission, cursor ascent, duplicate counting, and
identity finalisation. Checks also bracket analysis and successful completion. The
callback cadence bounds work between our checkpoints, not wall-clock time. Hashing,
UTF-8 validation, a collector call, serialisation, tree disposal, and allocation can
still do work between checks. Parser callbacks depend on the grammar and dependency.

Completed cancellable analysis has the same facts, diagnostics, coverage, content, and
context as complete analysis. Unsupported encoding and recovery remain explicit
results when the attempt completes. Changing this implementation conservatively
invalidates analyzer-context fingerprints and old analysis-cache entries.

## Native attempts and publication

The engine invokes the cancellable operation with the refresh predicate. Any worker's
`None` discards the entire candidate refresh, including when the caller's predicate
has not changed. It cannot publish a snapshot missing the interrupted file. Other
workers still join; the host's changed ticket lets their built-in analyzers stop too.

The interrupted file has no complete analysis to validate or cache. Complete results
from other files may already have warmed the disposable content-addressed cache.
Only a successful complete refresh reaches the existing publication-ticket check;
old pinned observations and pending dirty hints retain their existing rules.

This is cooperative cancellation during parsing and extraction. Discovery, reads,
cache access, document validation, snapshot construction, and destructor work do not
have a hard cancellation deadline. Neither a worker limit nor this operation implements
memory admission or persistent hot-file trees. See the
[parser responsiveness plan](../plans/parser-responsiveness.md) for the ownership and
resource contracts required before retaining trees across refreshes.

The [upstream parser reference](https://docs.rs/tree-sitter/0.27.1/tree_sitter/struct.Parser.html#method.reset)
documents progress callbacks and reset after interruption. An
[upstream recovery report](https://github.com/tree-sitter/tree-sitter/issues/6067)
describes callback gaps on pathological TypeScript input. That is a limit on general
deadline claims, not a measured regression in Vizzi's Rust/Python/C/C++ profiles.
