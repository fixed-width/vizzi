# Views, controls and shortcuts

For readers looking up interface behaviour. The shared header has the same navigation
and Settings menu across views; each view's controls occupy the row below it.

| View | Direct route | Purpose |
| --- | --- | --- |
| Call workspace | `/` (`/workspace` is an alias) | Source bubbles, call trails, result stacks and tasks |
| Review changes | `/review` | A scoped Git comparison and paired source |
| Source browser | `/explore` | Repository map, files and declaration outlines |
| Saved locations | `/investigate#saved` | Source anchors, notes and reader connections |

`/investigate` opens the repository-discovery part of Saved locations. The experimental
structural view remains a contributor prototype, not the main reading workflow.

## Call workspace

| Control or gesture | Result |
| --- | --- |
| Add bubble | Find a function and open its captured declaration |
| Click a highlighted call | Open its action menu |
| Ctrl-click / Cmd-click | Follow a highlighted call directly |
| Enter, Space or Down on a focused call | Open its action menu |
| Ctrl/Cmd+Enter on a focused call | Follow directly |
| Escape in a menu | Close it and return focus to its trigger |
| Calls | Show the bubble's recorded written calls |
| Bubble **⋯** | Find callers, find references or edit task-group membership |
| Drag header / lower-right grip | Move / resize a bubble |
| Arrow keys on a focused header / grip | Move / resize with the keyboard |
| ↕ Fit height / ↔ Fit width | Grow or shrink to fit the loaded section |
| Background drag or wheel | Pan the call workspace |
| + / −, or Ctrl/Cmd+wheel | Zoom |
| Fit workspace | Frame the current working set |
| Overview box | Return to a bubble; hover for its name/location |
| Pin version / Follow live | Hold captured source / resume saved-file updates |
| Save | Keep a repository source location separately from the visual layout |
| Editor | Verify and hand off a current source location to VS Code |

Function lookup accepts names and supported qualified forms, including `Type::method`,
`Type.method`, `Type method` and `Type/method`. It ignores case. Exact function names
rank before partial matches; qualified names and paths narrow the results.

Settings contains Theme everywhere. Code layout and Module outlines are call-workspace
settings. Source reflow preserves source line numbers; copying fully captured rows
preserves their original whitespace and line endings. Long indivisible tokens or bounded
source fragments can still require scrolling/paging.

## Other views

The Source browser's map uses wheel scrolling to **zoom**, unlike background panning in
the call workspace. Its Accessible file list provides keyboard file selection. Back/Forward
returns through the accepted browsing trail, which is separate from saved visual tasks.
The map shows at most 2,000 files per filtered query; filtering narrows a larger repository.

Source/review dividers resize their panes. Left/Right adjusts a focused divider, Shift
makes a larger step, and Home/End selects a width limit. Declaration outlines support
arrows, Home/End and Page Up/Down; Enter reveals the selected declaration.

Review controls select Working tree or Two commits, resolve revisions and scope the
capture. Refresh working updates only a working-tree comparison; Close comparison
retires it. [Review instructions](../how-to/review-local-changes.md) cover reading positions
and expiry. Saved locations has its own Locations menu for adding/exporting anchors.
