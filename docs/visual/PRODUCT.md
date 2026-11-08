# Visual call-trail prototype

For a maintainer validating the Code Bubbles-style interaction. The primary surface is
readable, concurrently visible source fragments on a spatial canvas. This is a read-only
editor companion; source search introduces a fragment rather than taking over the page.
Reuse the existing light/dark palette and fonts; Code Bubbles is the supplied interaction
reference, with no separate design mock.

- `/workspace` opens the canvas. **Add fragment** reveals a source-name chooser. A known
  Minitosis example can open its entry function for a guided visual demonstration, clearly
  distinguished from automatic task interpretation or a fresh search evaluation.
- Selecting a written call opens its candidate destination beside the caller. Earlier views
  retain position, dimensions and reading state. Multiple candidates are a deliberate
  choice; missing/dynamic targets remain unresolved. Links identify their source call site
  and candidate status rather than claiming semantic resolution/runtime execution.
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
prototype. No new model, parser, call-resolution mechanism or GPU backend is introduced.
