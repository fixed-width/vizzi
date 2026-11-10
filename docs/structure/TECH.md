# Structural exploration implementation

This document describes the existing prototype slice. The active product direction is
[visual working sets](../explanation/visual-working-sets.md); search/list navigation is
supporting infrastructure rather than the primary experience.

Read [the behaviour contract](PRODUCT.md). The implementation follows the source/analysis/
projection/host/editor seams in DESIGN.md. No new external dependency or unsafe code is needed.

## Captured syntax evidence

Extend `Analysis` with an optional bounded call-site record. Missing records mean unsupported
or old analysis, not an empty call set. Rust and Python extract calls during the existing
Tree-sitter traversal. A site records the callee's literal spelling/range, optional terminal
name and nearest emitted enclosing declaration. This is syntactic containment, including
closures/default arguments; it does not identify runtime execution ownership.

At most 4,096 calls and 256 UTF-8 spelling bytes per site are retained per document;
omissions are counted. Dynamic/unnameable callees remain inspectable without candidate
resolution. Validate ranges, literal spelling and enclosing declaration containment when
capturing Documents. Include the collector in analyzer fingerprints so persisted/reused
analysis invalidates correctly. Deferred/unsupported input must not claim call coverage.
Existing source capture and cancellation checks remain in charge; no extra parser pool or
hidden call reparsing is introduced. C-family analysis initially reports no call capability.

## Portable structural projection

A `vizzi-core::structure` module constructs a bounded metadata index from one validated
Snapshot. Its interface returns source areas, paged inventory, written calls and candidate
uses/targets. Opaque symbol identities include native file/version/context/fact identity.
The index owns compact labels/ranges/native bindings, not an Arc to a retired Snapshot or
copies of source text. Queries do not read backing storage.

[Module identity](../visual/MODULES_TECH.md) adds a bounded, cached Rust module-declaration
pass over resident captured text when preparing the index. It reads captured Cargo
configuration and Python package metadata, with no mutable-source IO or build tooling.
Ordinary call queries reuse the prepared projection.

Bound admitted documents/declarations/calls and expose omissions, partial syntax and
unavailable call analysis. Index calls by terminal name, preserving all same-name ambiguity.
Area filters are normalized paths with component boundaries; a similarly prefixed sibling
must not leak into results. Candidate destinations favor the selected area, with finite
outside-area alternatives. Terminal-name matching is a discovery hint, never name resolution.
Every call site is evidence independent of whether a destination is found.

Use independent deterministic fixture names and domains, not Tract/Sketchlayer vocabulary.
Initial limits: 4,096 documents, 65,536 declarations and 65,536 call sites per projection;
8,192 area records; 24 inventory/area rows and eight call/target rows per response. Over-limit metadata produces
explicit counts rather than a false complete inventory. Metadata allowances are not a
process-memory cap. Preserve per-language/context/coverage provenance.

## Native and browser adapters

A native application lazily caches one projection by current SnapshotId. Snapshot acquisition
and publication checks are brief; construction/query run outside the HTTP event loop under
existing bounded blocking work. Check expected current snapshot before publication. Expose
one typed structure operation route; reuse existing preview/current-only pin/editor routes.
Unknown fields and input/body bounds fail clearly. No executable or source bytes arrive
through the operation request.

The browser trial has one area/name search plus explicit inventory/call-site operations.
It renders source labels as text, a bounded inventory and a short reader-selected trail.
Candidate destination buttons are visually distinguished from the captured call spelling.
Reuse existing source drawer and saved investigation; no generic graph renderer or second
IDE outline is introduced. Responses use sequence/abort guards and source updates invalidate
old actions. The ordinary companion and question-search routes retain their defaults.

## Validation and stopping condition

Use TDD for call extraction/validation, projection queries and native/browser behaviour.
Cover comments/strings versus actual calls, Rust/Python, same names, receivers, dynamic
callees, nested scopes, unsupported/partial/omitted calls, exact source bindings, scope
boundaries, cycles, stale queries and replacement. Run workspace tests/doctests, formatting,
Clippy and affected headless browser checks. Run focused and broader relevant mutation
checks at nice 10, one mutant/compiler/jobserver task and serial tests. Distinguish caught,
equivalent, unviable and timed-out variants.

Freeze the implementation and development outcomes before opening Sketchlayer source or
querying its reserved questions. First outputs precede source-audited answers. Compare
navigation operations/source inspections with ordinary editor/text search; label automated
source-aware replay separately from independent maintainer effort. Do not claim a model,
semantic call graph or generalization from successful synthetic fixtures. Finish correctness
work before optimized-build timing; stop owned hosts and use recording-only editor adapters.
