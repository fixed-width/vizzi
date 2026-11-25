# Browse files and declarations

Open the address printed when you [start Vizzi](run-workspace.md), or choose
**Source browser** in the header.
This view is useful when you know an area or file rather than a function name.

With the VS Code extension, right-click inside a saved project file or on its editor
tab and choose **Vizzi: Open in Source Browser**. The command starts Vizzi if needed,
opens that file and reveals the cursor line when its editor is visible. It is also in
the Command Palette. Unsaved buffers are never saved automatically; save the file first.

## Select a file and read its structure

Select a file in the repository map. Its source and declaration outline open together.
The legend below the map identifies file colors. An outline marks the selected file,
and corner marks show syntax recovery and deferred or unavailable analysis.
Selecting a declaration reveals its lines; kind badges distinguish types, functions and
other declarations. **Line N** is the declaration's starting line.

In a large repository, start with **Landmarks** to open a README or project manifest
when one is available. Select a directory card on the map or a directory name in the
list to see what's inside. Each directory shows its total captured file count. Use the
breadcrumbs to return to a parent directory.

Use **Next page** and **Previous page** to see more entries in a large directory.
The expandable **Accessible file list** shows the files on the same page as the map.
Use **Find a file** to search paths across the repository, including directories you
haven't opened.

Select file types in the map legend to show files of any selected type. Select a type
again to remove it from the filter. Directory counts reflect the matching files.
With **Find a file**, the results must also match the path you entered. Choose **Clear
filters** to clear the type selections and path search.

Drag the map background to pan. Wheel scrolling zooms this map; **Fit map** resets the
framing. The small overview navigator jumps to a different region. Arrow keys move a
focused map or navigator. These gestures differ from the call workspace's background
wheel panning.

On the nested map, use directory controls to collapse, focus or reorder groups. Drag
rows or headers, or use their up/down controls to reorder siblings. **Reset map**
restores the default order and expansion.

## Continue in the Call workspace

Click a highlighted function name in its definition and choose **Open in Call
workspace** to open its bubble in the active task. Click a highlighted call name and
choose **Open definition in Call workspace** to open its caller and follow the call.
If Vizzi finds several possible destinations, it opens the same chooser used in the
[Call workspace](follow-a-call-trail.md).

Ctrl-click or Cmd-click to open directly. With the keyboard, Tab to a highlighted
name and press Enter for its menu or Ctrl/Cmd+Enter to open directly.

## Adjust the reading panes

Drag dividers to resize the map/source or declaration/source sections. Focus a divider
and use Left/Right, Shift for a larger step, or Home/End for its limits. Width preferences
persist locally when browser storage is available.

Use the view's **Back**/**Forward**, or browser history, to return to an accepted file,
declaration and reading position. This is a browsing trail, not the visual task shelf;
reload starts a new trail. Changed or missing source is identified instead of silently
substituting another location.

Use **Save location** to retain a source anchor. [Saved locations](investigate-with-vscode.md)
explains notes, editor handoff and optional persistence.

## Show C/C++ source

Enable capture for a relative source scope when starting the server:

```sh
./target/release/vizzi \
  --root /path/to/project \
  --workers 1 \
  --c-family-root src \
  --syntax-language src/include=cpp \
  --exclude-source src/vendor
```

Use `.` for the whole project. The syntax assignment chooses raw C or C++ for ambiguous
source such as headers. Unassigned `.h` files remain source-only; unsupported dialects
such as CUDA are identified. Exclusions apply to all languages.

These are relative paths under `--root`, not revisions or arbitrary host paths. Repeat
options to add scopes. No compiler, compilation database, build or project script is
invoked. The outlines describe raw syntax rather than a configured compiler view.

Check the [language coverage table](../reference/languages.md) before relying on navigation.
Some unrelated languages also use `.c`/`.h`; exclude those directories rather than
classifying them as C/C++.
