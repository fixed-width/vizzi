# Visual call-trail prototype

For a maintainer validating the Code Bubbles-style interaction. The primary surface is
readable, concurrently visible source fragments on a spatial canvas. This is a read-only
editor companion; source search introduces a fragment rather than taking over the page.
Reuse the existing light/dark palette and fonts; Code Bubbles is the supplied interaction
reference, with no separate design mock.

Workspace controls use **bubble** for a canvas view and **saved location** for a retained
source anchor. Workspace actions (New workspace, Saved locations) have their own menu;
Settings contains Theme and Code layout. The example button is absent from the header.
Call arrows have no inline evidence/status labels. Hover/focus details retain provenance
and stale-call meaning; old or uncertain links remain visually distinct. Minimap items
name the qualified function and source path/line through a tooltip and accessible label.

Opening or returning to a bubble pans only as needed, preserving the reader's zoom.
Fit workspace and zoom controls are explicit scale changes. When caller and destination
cannot both fit at the current scale, reveal the destination without changing scale.

Separate **Expand height** and **Expand width** actions grow a bubble to fit its currently
loaded section, including current reflow for height and original line widths for width.
Expansion never fetches more source, shrinks a bubble, changes camera scale or rearranges
unrelated bubbles. Directly obstructing bubbles, and any bubbles they obstruct in turn,
move down for height growth or right for width growth with a fixed gap. The selected
bubble stays anchored. Compute the complete bounded change before applying it; failure
leaves the layout intact. Width caps at 1,600 and height at 2,400 canvas pixels; code that
still exceeds these bounds remains scrollable. Manual resizing and section paging remain.

- `/workspace` opens the canvas. **Add bubble** reveals a function chooser supporting
  captured qualified names with `::` or `.`. Rank the complete function result set before
  paging: exact names/qualified names, exact qualified suffixes, function-name prefixes/substrings, namespace prefixes/substrings,
  then path matches; break ties alphabetically and by source location. A guided Minitosis
  URL remains available for reproducible demonstrations, outside the workspace header.
- Selecting a written call opens its destination beside the caller. Earlier views
  retain position, dimensions and reading state. A language-supported unique destination opens directly;
  unresolved multiple candidates require a choice. Source-local Rust `self` bindings are
  distinct from name candidates, as described in [call destination selection](../explanation/call-destinations.md).
  The [type-aware follow-up](SEMANTIC_PRODUCT.md) resolves ordinary receivers through a
  configured language provider. Links identify their call site and evidence rather than claiming runtime execution.
- Captured call identifiers are underlined directly in the source, with hover and keyboard
  focus feedback. Click opens a small action menu; **Open definition** follows the call.
  Ctrl-click (Cmd-click on macOS) follows directly using the same destination rules.
  Enter/Space/Down opens the menu on a focused identifier, Ctrl/Cmd+Enter follows directly,
  and Escape dismisses the menu and returns focus. Source remains selectable; dragging
  a selection does not navigate. Comments, strings and declaration names are not call links.
  The menu admits future actions without presenting unimplemented controls. **Calls** remains
  available when an identifier is outside the visible section or cannot be decorated safely.
- Following another step retains the chain. A branch opens into free space without globally
  rearranging existing views. Recursion/revisiting an open fragment reveals that existing
  view and link instead of recursively creating an unbounded graph.
- Fragment headers support moving, resizing, collapsing and closing. Closing removes that
  view's presentation links, not source facts. Code remains syntax-coloured and readable;
  long functions use explicit bounded sections instead of silent short excerpt replacement.
- Smart reflow fits long code lines to their pane using ranked, language-aware break hints
  and bounded hanging indentation. It is enabled by default; **Settings → Code layout** can
  restore original lines. Resizing recomputes display breaks locally, without source reads
  or moving neighbouring fragments. Each physical source line keeps its original number;
  continuation marks distinguish display rows from source lines. Copying returns the
  selected original text from fully captured rows, with original whitespace and line endings.
  Clipped selections retain browser copying and their visible omission marks.
  Syntax colours, call actions and the visible source location survive reflow. Strings,
  identifiers and other indivisible tokens may still require horizontal scrolling; clipped
  source remains explicitly clipped. Missing language/style support falls back to original
  lines. Language rules must not be hard-coded into the browser layout.
- Source fragments remove the function's file-level leading indentation from display in
  both layout modes. The first captured declaration line establishes a fixed baseline;
  later sections and reloads use that same baseline. Relative body/continuation indentation,
  including tabs and mixed whitespace, is retained. Lines indented less than the baseline
  stop at the gutter. Copying retains original source indentation and original byte/line
  identities continue to drive navigation and editor handoff.
- Canvas/background dragging and ordinary wheel panning move the workspace. Zoom is explicit
  through controls or modified wheel. Code-pane scrolling stays local. Opening a destination
  reveals it with its parent where viewport size allows; overview/fit controls recover earlier
  offscreen context. Avoid non-user-triggered animation and global force layouts.
- A local visual-workspace document preserves geometry, camera, reading state and source
  bindings separately from source facts and the existing investigation. Refresh restores
  those views; storage failure is visible. This prototype's persistence is browser-local,
  not a portable/multi-device source archive.
- Working-tree views follow saved changes through native declaration tracking, preserving
  their visual arrangement. **Pin version** retains one view; **Follow live** resumes it.
  Missing/ambiguous functions and unverified links retain explicit earlier-version context.
  The [live behaviour contract](LIVE_PRODUCT.md) defines refresh, selection deferral and
  capture-expiry limits. Current-only Keep/editor safeguards remain in effect per view.
- Controls are keyboard accessible, source/names render as text, themes follow system/override
  settings, and small viewports can pan/zoom without overflowing the page. No desktop editor
  opens automatically. Automated verification uses recording-only adapters.
- **Editor** opens a current fragment at its original declaration line without requiring
  **Save**. The host verifies its snapshot/file/version and current working bytes first.
  Opening does not create evidence or change the investigation, including when saving is
  unavailable. **Save** remains independent. Stale fragments cannot request a handoff.

The first acceptance question is whether the maintainer can walk and branch through a real
call trail while retaining previous-step context. Groups, alternative representations and
revision/diff views are supported by the design direction but are not required in this small
prototype. No new model, parser or GPU backend is introduced. Source-local Rust bindings extend the
existing captured call metadata without requiring a repository build.
