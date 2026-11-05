# Controllable repository orientation and readable appearance

This behaviour contract responds to the second maintainer observations after `24d48df`.
The audience is implementers/reviewers. Use the existing source-workspace visual direction;
no separate mock is supplied. Research rationale belongs in
[the layout explanation](../explanation/repository-layouts.md).

- Git revision expressions and unique short IDs follow Git's own resolution, on both
  comparison sides. Missing shallow history is distinct from unsupported syntax.
  The disposable maintainer review copy should contain local ancestor history without
  changing its pinned HEAD, index, working files or untracked task document.
- Error, warning and informational message rows have accessible close controls. Closing
  changes presentation only. Repeated rendering of the same message/context does not
  resurrect it; a new operation, source context or message can be shown. Closing a review
  notice preserves its recovery actions until the notice itself is dismissed.
- System is the default theme, with Light and Dark overrides. Preferences survive reload
  when storage is available. Both pages and native controls follow the choice; system
  changes take effect while System is selected. Canvas colours follow CSS tokens too.
- Syntax categories are visibly distinct in hue and, for keywords/types, weight. Test
  foreground contrast on plain, selected and diff backgrounds in both themes. Keep
  literal UTF-8 source, viewport limits and existing line metrics.
- The map displays actual directory containment, ordered siblings and readable group
  labels. Compact single-child directory paths instead of inventing package relations.
  Collapse/expand a directory, focus it, return to ancestors, and reorder siblings by
  dragging or keyboard controls. Reordering changes this view, never repository paths.
- A directory tree complements spatial navigation. Keep tree DOM and canvas/data work
  bounded for the existing 2,000-file projection. Preserve selected source when collapsing
  its directory. A new file selection can reveal its ancestors. Keep collapse/order
  preferences scoped to a stable workspace identity, with reset and storage-refusal fallback.
- Reordering, collapse, focus, hit testing and the overview navigator use the same scene.
  Changes to personal layout invalidate incompatible saved map-camera positions, while
  source/outline history retains its existing semantics.

Tiny histories reproduce normal and shallow revision behaviour. Pure scene tests cover
containment, order, collapse, compact paths, bounded depth and mixed directories; browser
checks cover actual interactions, theme changes, dismissals, literal styled source and
linked selection. Run serial lower-priority mutations and scoped release observations.

## Maintainer correction after `7de79b7`

Dismissal applies only to ephemeral full-width status/error banners outside the main
panes. Persistent headings, capture labels, source evidence, outline/source empty states
and pane help remain ordinary content. Refreshing an available view keeps its readiness
label stable; initial capture and real errors retain explicit status. Busy state can be
communicated without momentary replacement text.

At every zoom, a directory heading is either drawn completely within its header band
or omitted when there is insufficient room. Directory controls remain available at
small scales. Declaration names stay aligned. A compact enclosing-scope line and depth
indicator expose lexical structure without unbounded indentation. Keep 48-pixel virtual
rows and the source/outline payload limits. Language adapters supply parent links; do not
infer semantic ownership from names or arbitrary overlapping source ranges.
