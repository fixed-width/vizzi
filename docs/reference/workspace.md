# Views, controls and shortcuts

For readers looking up interface behaviour. The shared header has the same navigation
and Settings menu across views; each view's controls occupy the row below it.

| View | Direct route | Purpose |
| --- | --- | --- |
| Source browser | `/` (also `/explore`) | Repository map, file and declaration search, captured source |
| Call workspace | `/workspace` | Source bubbles, call trails, result stacks and tasks |
| Review changes | `/review` | A scoped Git comparison and paired source |
| Saved locations | `/investigate#saved` | Source anchors, notes and reader connections |

`/investigate` also opens Saved locations. The contributor structural trial is available
at `/investigate?trial=structure`.

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
| Previous change / Next change | Step through changes to this function in local first-parent Git history |
| Return to live | Leave a historical, comparison or captured-call view and resume saved-source updates |
| Compare | Open pinned Before/After views of adjacent function versions at the first changed block |
| Previous changed block / Next changed block | Reveal corresponding changes in visible comparison bubbles |
| Older revision (exit diff) | Step back through function history and exit the comparison in that bubble |
| Find earlier declaration… | Inspect and confirm a rename or file move from a history bubble at the changing commit |
| Open live source | Open the current caller from a result row with its supported call connection |
| Show comparison | Reveal an existing comparison of the caller at its retained reading position |
| Open in captured file | Follow a call using only the historical/comparison file's declarations |
| Look in another file… | Choose a repository file and confirm a name candidate at the caller's Git commit |
| Save | Keep a repository source location separately from the visual layout |
| Editor | Verify and hand off a current source location to VS Code |

Function lookup accepts names and supported qualified forms, including `Type::method`,
`Type.method`, `Type#method`, `Type method` and `Type/method`. It ignores case.
Exact function names rank before partial matches. **Source file path (optional)** limits
the lookup to one captured file, including files and declarations omitted by the bounded
repository search. Use the exact workspace-relative path with forward slashes.

Function history freezes local HEAD when first opened and considers up to 64 file revisions.
**Choose revision…** starts a timeline at an exact local commit, branch or tag.
It skips unchanged declarations, stops at missing or ambiguous matches, and does not infer
renames or moves. Historical bubbles disable current-source actions. See
[reading function history](../how-to/read-function-history.md).

Settings contains Theme everywhere. Code layout and Module outlines are call-workspace
settings. Source reflow preserves source line numbers; copying fully captured rows
preserves their original whitespace and line endings. Long indivisible tokens or bounded
source fragments can still require scrolling/paging.

## Other views

The Source browser's map uses wheel scrolling to **zoom**, unlike background panning in
the call workspace. Its Accessible file list provides keyboard file selection. Back/Forward
returns through the accepted browsing trail, which is separate from saved visual tasks.
Repositories with up to 2,000 captured files use a nested map. Larger repositories show
immediate directory contents in pages of 100 entries, with directory cards reporting
counts for their full subtrees. Repository-wide path search in large repositories is
also paged. The current directory's README and recognized manifests appear as landmarks.

Source/review dividers resize their panes. Left/Right adjusts a focused divider, Shift
makes a larger step, and Home/End selects a width limit. Declaration outlines support
arrows, Home/End and Page Up/Down; Enter reveals the selected declaration.

Review controls select Working tree or Two commits, resolve revisions and scope the
capture. Refresh working updates only a working-tree comparison; Close comparison
retires it. [Review instructions](../how-to/review-local-changes.md) cover reading positions
and expiry. Saved locations has its own Locations menu for adding/exporting anchors.
