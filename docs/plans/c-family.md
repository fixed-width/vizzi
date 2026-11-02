# C-family implementation and validation plan

This plan sequences the [C-family policy](../prototype/C_FAMILY_POLICY.md). The policy
is defined; stages 1 and 2 are implemented. Rust/Python remain the default capture
scope; explicit C-family scopes analyze assigned C/C++ inputs and retain source-only
ambiguity/dialect coverage. No stage below is
an implicit approval to execute analyzed-project tooling.

## 1. Separate source policy from analyzer registration

Complete: [source-policy reference](../prototype/SOURCE_POLICY.md). The policy/native
tests cover scope expansion/reduction, empty baselines, source-only inputs, normalized
rules, precedence, availability changes, and classification/pin consistency. Browser
checks cover schema 2, source-only diagnostics, encoding, live edits, and pinned metadata.
The [validation report](../../experiments/languages/SOURCE_POLICY_RESULTS.md) records
the complete test/mutation scope, measurements, and remaining qualification limits.

Introduce a portable source-policy module over normalized relative paths. It produces
excluded, source-only, or assigned inputs with structured provenance; native source
adapters perform the resulting I/O. Keep factories and analysis contexts in the core
analysis interface. Apply one policy to discovery, hints, and invalidation. Freeze
the policy at host launch; configuration syntax is a delivery
choice, with programmatic policy construction available to tests and later hosts.

Add policy identity to snapshots and classification metadata to document versions.
Enforce full discovery on policy mismatch, including empty baselines and newly enabled
subtrees. Preserve workspace/file identities and path-independent raw caches. Add the
versioned query fields and browser display needed for source-only candidates.

Start with failing behavioral tests for scope expansion, exclusion precedence,
path-component matching (`src` versus `src2`), same-scope assignment conflicts, empty
snapshots, classification-only changes, and pinned old metadata. Include `.c++`/`.h++`
suffixes: the existing registry's alphanumeric-only extension validation cannot
represent those claims and must not silently drop them. The new source-policy seam
should remove that selection constraint rather than duplicate extension maps.

## 2. Add raw C/C++ adapters through that interface

Complete: [extraction reference](../prototype/C_FAMILY_SYNTAX.md). Fixtures cover
multiple typedefs, definitions versus references, templates, methods/operators,
qualified conversion names, recovery/encoding, header assignments, cache isolation,
cross-language rename, and a shared 1/2/4-worker budget across all four languages.
Scoped reference-repository runs are smoke checks; stage 3 still requires full-scale
application/resource qualification.
The [adapter validation report](../../experiments/languages/C_FAMILY_RESULTS.md) records
the final test/mutation scope, scoped checks, and remaining limits.

Use safe grammar wrappers, current stable compatible dependencies, and one shared
worker budget. Establish declaration extraction on small C/C++ fixtures before
enabling reference-corpus profiles. Keep the lpc-rs profile Rust/Python only; explicitly
verify that its `.c`/`.h` fixtures never enter C/C++ analysis.

The correctness matrix includes a header with no assignment, explicit C/C++ header
assignments, C++ in a `.c` file, CUDA rejection, fragments, macro/conditional recovery,
duplicate declarations, templates/operators, invalid encoding, and identical bytes
assigned to different languages. Test targeted edits and cross-language renames against
clean captures. Confirm warm hits create no parsers and worker limits remain shared.

Run formatting, workspace tests/doctests, Clippy, actual browser tests, and complete
affected-module mutation testing. Investigate survivors and report unviable builds,
timeouts, and equivalent exclusions separately. Do not equate grammar acceptance
with correctness of extracted declarations or build coverage.

## 3. Qualify on pinned Linux and PyTorch sources

Full-host collection complete: [results](../../experiments/qualification/RESULTS.md)
cover cold/warm loads, retained reconciliation/reparse at 4/8 workers, cancellation,
live source/large-header edits, and software browser interaction. Qualification exposed
unbounded outline rendering and latency costs. Address the
[proposed performance gates](../performance/BUDGETS.md) before compiler-context expansion;
16 GB baseline hardware and GPU presentation remain unqualified.
The [responsiveness follow-up](../../experiments/qualification/RESPONSIVENESS_RESULTS.md)
implements bounded outline controls and verified capture plans. Large-file parsing,
transport data bounds, and memory admission remain open. The
[cancellation follow-up](../../experiments/qualification/CANCELLATION_RESULTS.md)
implements cooperative interruption within built-in parsing and extraction; bounded
hot-file syntax retention is now opt-in with
[measured limits](../../experiments/retention/RESULTS.md). [Verified source sharing](../../experiments/shared-source/RESULTS.md) reduces capture
copy overlap. Working-memory admission and bounded projection delivery remain work
before compiler-context expansion.

Use scoped raw-C/C++ profiles, including explicit header assignments where desired,
and retain their classification provenance in reports. Record source-only candidates
separately from analyzed files. Capture cold, persisted-cache, in-memory reconciliation,
one-file/header edits, and cancellation/publication behavior. Compare each changed
snapshot with complete capture, and check independently specified fixture facts.

Measure RSS with a previous snapshot retained, large generated headers, interaction
during indexing, and worker counts justified by earlier parsing results. Collect
compiler-free raw results separately from any future configured result. These runs
must establish the next resource budgets; the current grammar-only and Python results
do not qualify C-family application performance on the 16 GB baseline.

## 4. Import profiles, then design configured analysis

First implement read-only compilation-database import and deterministic profile
identities. Test multiple records per file, argument order, relative paths, unsupported
command-string quoting/wrappers, response-file changes, and unavailable generated
inputs. Preserve original records and diagnostics. Import must launch no processes.

Before enabling compiler-backed work, implement captured translation-unit bundles,
source mappings, dependency/lookup invalidation, and bounded context admission. Fixtures
must include one header under two macro environments and two languages, a newly
appearing include candidate, target changes, and an unavailable configured context
with usable raw source. Choose a backend and user action only after those interfaces
and constraints are concrete. Variability-aware parsing remains a separate decision.
