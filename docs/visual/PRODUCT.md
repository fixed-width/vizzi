# Visual call-trail prototype

For a maintainer validating the Code Bubbles-style interaction. The primary surface is
readable, concurrently visible source fragments on a spatial canvas. This is a read-only
editor companion; source search introduces a fragment rather than taking over the page.
Reuse the existing light/dark palette and fonts; Code Bubbles is the supplied interaction
reference, with no separate design mock.

- `/workspace` opens the canvas. **Add fragment** reveals a function chooser supporting
  captured qualified names with `::` or `.`. Rank the complete function result set before
  paging: exact names/qualified names, exact qualified suffixes, function-name prefixes/substrings, namespace prefixes/substrings,
  then path matches; break ties alphabetically and by source location. A known
  Minitosis example can open its entry function for a guided visual demonstration, clearly
  distinguished from automatic task interpretation or a fresh search evaluation.
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
- Canvas/background dragging and ordinary wheel panning move the workspace. Zoom is explicit
  through controls or modified wheel. Code-pane scrolling stays local. Opening a destination
  reveals it with its parent where viewport size allows; overview/fit controls recover earlier
  offscreen context. Avoid non-user-triggered animation and global force layouts.
- A local visual-workspace document preserves geometry, camera, reading state and source
  bindings separately from source facts and the existing investigation. Refresh restores
  those views; storage failure is visible. This prototype's persistence is browser-local,
  not a portable/multi-device source archive.
- Source updates preserve the existing views/loaded bytes and mark them stale. Never silently
  retarget a fragment. New/current source starts an explicit new workspace. Exact captured
  windows and existing current-only keep/verified editor actions retain their safeguards.
- Controls are keyboard accessible, source/names render as text, themes follow system/override
  settings, and small viewports can pan/zoom without overflowing the page. No desktop editor
  opens automatically. Automated verification uses recording-only adapters.

The first acceptance question is whether the maintainer can walk and branch through a real
call trail while retaining previous-step context. Groups, alternative representations and
revision/diff views are supported by the design direction but are not required in this small
prototype. No new model, parser or GPU backend is introduced. Source-local Rust bindings extend the
existing captured call metadata without requiring a repository build.
