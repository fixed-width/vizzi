# Visual workspace prototype architecture

Read [the behaviour contract](PRODUCT.md) and [domain terms](../../CONTEXT.md).

The existing Rust engine remains authoritative for snapshots, captured source, call evidence
and native bindings. Reuse `/api/structure` for declarations/written-call candidates and
`/api/window` for bounded, styled source sections. Reuse current-only pin/verified editor
operations when explicitly requested. Language-specific destination evidence stays in the Rust
analyzer and captured call metadata; the browser never chooses a type based on names or paths.
The first [source-local binding](../explanation/call-destinations.md) covers direct Rust `self`
receivers across inherent/trait impl blocks in one lexical scope/file. The projection returns
a sole target and its evidence kind, or the existing bounded candidates.

A browser presentation module owns a versioned visual-workspace document: view identities,
source bindings, rectangles/collapse state, reading sections/scrolls, camera and visual links.
Its small interface opens roots/destinations, moves/resizes/collapses/closes views and
serialises/restores the workspace. Source symbols and view identities are separate. The
first source-view prototype deduplicates an already-open symbol while leaving the document
schema capable of other representation kinds later. Store source metadata/IDs, not copied
source text, in browser persistence. Bounds/validation reject malformed, oversized and
nonfinite layout data. Do not mutate analysed relationships while arranging presentation.

This presentation model starts in the existing browser implementation so the maintainer
can evaluate interaction before a durable portable model/storage migration. A production
Rust model remains an option; avoid parallel Rust/JavaScript layout implementations. Native
interfaces are editor-neutral and use opaque snapshot/file/version/context IDs, not Git IDs.

DOM source panes and a shared SVG link/overview layer provide the first implementation.
Use the existing theme tokens and styled-text renderer. Keep existing view DOM nodes alive
when another opens so scroll/selection state survives. Place only the new view into free
space beside its parent; avoid reflowing older nodes. A camera changes presentation transforms,
not view geometry. Load up to 96 captured lines per section and at most 24 source views;
page long declarations and call lists explicitly. Handle per-view fetch cancellation and
late replies without replacing/duplicating another view's source. A global source change
invalidates navigation while retaining old loaded views; expired backing is explicit.

The model is independent of DOM/SVG and any future wgpu/Canvas renderer. The prototype does
not settle the production renderer. A large source workspace requires only a focused task
working set to be materialised. A bounded-node/line limit is not a frame-time qualification.

Use TDD through the presentation-model interface and headless browser. Exercise stable
caller/branch geometry, reading state, candidate witnesses, revisits/cycles, camera/fit,
move/resize/collapse, close, malformed persistence, stale snapshots and bounded growth.
Perform mutation checks on the presentation model and native binding rules with one
worker/compiler task at nice 10. Persist source-local and candidate link kinds separately. Run existing workspace tests/doctests, formatting,
Clippy and the affected/full browser suite. Capture the real Minitosis interaction for
review after correctness checks. Do not fabricate semantic links or usability results.
