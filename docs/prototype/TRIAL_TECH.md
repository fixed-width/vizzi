# Trial findings: implementation contract

This reference implements [the behaviour contract](TRIAL_PRODUCT.md) after `a59890e`.
Keep source/language analysis, immutable models, native capture and browser presentation
separate. No production renderer/backend migration is required for these findings.

## Captured colouring

Bundled highlight queries over the existing Tree-sitter parse collect sparse syntax-style boundaries. A compact
immutable index carries vocabulary IDs and byte offsets, independently of CSS/theme.
It is content/context-bound analysis metadata and participates in document identity.
Persist it compactly; base64 0.23.1 was verified as the latest stable release, with
default unsafe SIMD features disabled and the safe standard-library path selected.

Each document index has a 1-MiB packed-byte allowance. Reaching it discards colour data,
without discarding declarations/source. Projection uses binary search and clips to the
actual returned UTF-8 fragments. A window admits at most 1,024 style segments as well
as the existing 96-row/32-KiB text limits. Invalid/missing/limited style data falls back
to plain source with explicit availability metadata. No parser/tree work occurs in
window queries. Custom adapters may omit colouring.

The browser validates style classes/ranges and uses text nodes/spans, never source HTML.
Reuse row DOM when its fragment object is unchanged. Existing version-bound expanded
fragment caching carries its styles with the text. Bump implementation fingerprints
for all built-in adapters so old cache records do not masquerade as coloured analysis.

## Comparison capture

Add an optional target revision to create input and pinned target labels to metadata.
The native spec retains the resolved target alongside the base. Capture the right
observation through the existing revision provider; the portable comparison/ownership
model continues to pair supplied immutable observations. The wire `working` side is
the right-hand observation in both modes; presentation calls it Target for revisions.

Only working-tree comparisons accept refresh. After capture, an explicitly non-root
path with zero inventory on both sides is refused before publication. Do not infer
scope validity from analysed-document counts: metadata-only/excluded paths still count.
Field-specific context in failure messages distinguishes base, target and path errors.

The review controller owns lifecycle errors and pending state. Rendering has a dedicated
form alert independent of current-view notices; a missing view cannot clear it. New
optional wire fields remain additive and legacy working-tree inputs retain defaults.

## Presentation and geometry

Native layout deterministically packs approximately square directory cards into shelves
whose width scales with bounded file count. Publish group bounds for the overview
navigator. Browser camera helpers own fit, anchored zoom and overview navigation;
portable projections retain geometry/selection identity. Explicit overview and minimap
navigation complement readable zoom instead of turning wheel zoom into plain scrolling.

Shared splitter and presentation modules serve exploration/review, with pointer/
keyboard adapters and small deterministic interfaces. CSS variables carry split ratios;
local storage contains only bounded numeric preferences. Keep source/outline windows
virtualised and remeasure after layout/colour changes. Expose concise user-facing copy;
developer timing samples remain available to validation without a footer.

The [second orientation iteration](ORIENTATION_TECH.md) supersedes the shelf layout as
the browser default, adds workspace-scoped personal layout, shared dismissible messages
and CSS system/override themes. Native shelf rectangles remain a compatible diagnostic
projection.
