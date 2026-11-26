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
  bounded. Preserve selected source when collapsing
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

## Browsing large repositories

Every captured file has a route from the repository root without typing a path. Keep
the nested map for repositories of up to 2,000 captured files. In larger repositories,
show the current directory's immediate children. Directory cards show full subtree file
counts. Open a card from the map or the directory controls and use breadcrumbs to return
to an ancestor. Keep the selected source open as the reader moves between directories.

Page immediate children in alphabetical order within directories and files, with
directories first. The map and accessible file list display the same page, and the
controls show its range and total. Path search still covers the whole repository and
pages through large result sets. Clearing the filter returns to directory browsing.

Surface the current directory's captured README, recognized project manifests, and files
with recognized execution entry points near the top of the map pane. Ruby gem specifications
use the `.gemspec` suffix. List each file once, even if it contains several entry points.
A landmark opens its captured text like any file. Directory paging leaves these links available.
Capture conventional README names as source-only, subject to the workspace's source exclusions.

Entry points are based on source conventions. Recognize top-level Rust `main` functions,
global C/C++ `main` definitions, Java launcher-style `main` methods, Python `__main__.py`
modules and top-level `__name__` guards, top-level Ruby direct-launch guards, and top-level
CommonJS or ECMAScript main-module guards in JavaScript and TypeScript. A helper named `main`
in an ordinary scope is insufficient. These links describe captured source. The actual
entry point of a particular build or launch depends on that project's configuration and runtime.

Verify with a fixture containing a directory absent from the initial file slice and a
directory with more than one page of files. Open a late file, navigate back to the root,
and open a README, a manifest and an entry-point file. Check the existing small-repository
map interactions as well.
