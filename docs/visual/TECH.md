# Visual workspace prototype architecture

Read [the behaviour contract](PRODUCT.md) and [domain terms](../../CONTEXT.md).

The existing Rust engine remains authoritative for snapshots, captured source, call evidence
and native bindings. Reuse `/api/structure` for declarations/written-call candidates and
`/api/window` for bounded, styled source sections. Reuse current-only pin/verified editor
operations when explicitly requested. Language-specific destination evidence stays in the Rust
analyzer and captured call metadata; the browser never chooses a type based on names or paths.
The first [source-local binding](../explanation/call-destinations.md) covers direct Rust `self`
receivers across inherent/trait impl blocks in one lexical scope/file. The projection returns
a sole target and its evidence kind, or the existing bounded candidates. The optional
[definition provider](SEMANTIC_TECH.md) handles other calls on demand through standard LSP,
without adding type heuristics to the browser or structure index.

Fragment editor handoff posts a captured snapshot/file/version and original line to
`/api/editor/open`. It validates the current observation and exact document version, then
uses the same bounded, no-follow working-file/content verification as saved evidence.
Neither evidence nor an investigation revision is required, and opening does not mutate
the investigation. Both handoff routes share the editor concurrency limit and configured
editor adapter. **Keep** remains a separate action.

The `actions` structure projection returns captured identifier line/UTF-8 byte columns,
opaque call identity and any source-local binding. It reads metadata, without loading source
or expanding name candidates. Queries cover at most 96 lines and page at 256 actions; the
browser permits at most 16 pages/4,096 actions per section. Captured source and action
metadata load together. The renderer validates exact identifier bytes, rejects overlapping
or left-clipped ranges, and preserves syntax spans and source characters. An identifier's
action retrieves the full native call witness through `/api/call-definition`, including
when no language provider is configured. Existing source/current-snapshot checks and
destination rules apply to both inline and Calls navigation.

A separate menu overlay uses viewport coordinates outside the transformed canvas, so zoom
and pane clipping do not scale or cut off its controls. Focus, selection, keyboard shortcuts
and dismissal belong to this presentation module; future actions can extend its action list.
Stale observations disable inline actions and dismiss the menu while retaining loaded source.

Source reflow is presentation-only. The opt-in `reflow=true` source-window projection uses
the captured language and parser-derived styling to emit UTF-8 break offsets, priorities
and delimiter-alignment anchors. Rust, Python and C/C++ share the projection; a language
profile classifies operators while literals remain protected. No formatter, new parse,
source mutation or cache-identity change is required. Bound hints per line and window;
unavailable/limited styling and left-clipped lines have no hints.

A language-neutral browser planner chooses display segments for actual font/pane width.
A source-pane module owns rendering, resize coalescing, selection/copy and reading anchors;
the workspace controller owns captured-source requests and call-navigation intent. Retain
source byte coordinates through styled segments. Decorative indentation/continuations are
excluded from copied text. Reflow only changed layouts, defer resize work while source is
selected, and preserve the visible source line/segment on width changes. Camera transforms
do not change the layout width. Keep the existing palette/fonts and avoid extra toolbar
controls or formatting animation. The Code Bubbles papers are the interaction reference;
there is no separate mock.

The source pane also accepts a fixed function margin in display columns (four-column tab
stops). Its controller derives it from the captured declaration line, caching it for later
sections. A restored later section fetches one bounded, pinned declaration window alongside
its body and validates the same snapshot/file/version. Baselines do not come from the
visible body's minimum indentation. Rendering hides original leading whitespace in the DOM
and supplies the remaining relative width as decoration; source strings and call offsets
are unchanged. Reflow measures the displayed prefix and delimiter anchors. Both layout
modes apply the same margin. Copying from a visible line start restores its hidden prefix.

A browser presentation module owns a versioned visual-workspace document: view identities,
source bindings, rectangles/collapse state, reading sections/scrolls, camera and visual links.
Its small interface opens roots/destinations, moves/resizes/collapses/closes views and
serialises/restores the workspace. Source symbols and view identities are separate. The
first source-view prototype deduplicates an already-open symbol while leaving the document
schema capable of other representation kinds later. Store source metadata/IDs, not copied
source text, in browser persistence. Bounds/validation reject malformed, oversized and
nonfinite layout data. Do not mutate analysed relationships while arranging presentation.
Reading state may include a source line/byte anchor in addition to legacy pixel offsets.
It belongs to the displayed captured source and remains independent of display breaks.

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
