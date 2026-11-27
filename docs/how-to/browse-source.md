# Browse files and declarations

Open the address printed when you [start Vizzi](run-workspace.md), or choose
**Source browser** in the header.
Use the map to explore an area or **Find source** to jump to a file or declaration.

With the VS Code extension, right-click inside a saved project file or on its editor
tab and choose **Vizzi: Open in Source Browser**. The command starts Vizzi if needed,
opens that file and reveals the cursor line when its editor is visible. It is also in
the Command Palette. Unsaved buffers are never saved automatically; save the file first.

## Select a file and read its structure

Select a file in the repository map to read its source. The **Declarations** pane
appears when Vizzi finds declarations in that file. Otherwise, the inspector shows
**No declarations extracted** and gives the source the full width.
The legend below the map identifies file colors. An outline marks the selected file,
and corner marks show syntax recovery and deferred or unavailable analysis.
Selecting a declaration reveals its lines; kind badges distinguish types, functions and
other declarations. **Line N** is the declaration's starting line.

Use **Landmarks** to open the current directory's README or project manifests. Open the
**Entry points** picker to find files with launch conventions anywhere under the current
directory. Start at the repository root to see every captured entry-point file. Select a
directory card on the map or a directory name in the list to see what's inside. Each
directory shows its total captured file count. Use the breadcrumbs to return to a parent directory.

Use **Next page** and **Previous page** to see more entries in a large directory.
The expandable **Accessible file list** shows the files on the same page as the map.

Select file types in the map legend to show files of any selected type. Select a type
again to remove it from the filter. Directory counts reflect the matching files.

Drag the map background to pan. Wheel scrolling zooms this map; **Fit map** resets the
framing. The small overview navigator jumps to a different region. Arrow keys move a
focused map or navigator. These gestures differ from the call workspace's background
wheel panning.

On the nested map, use directory controls to collapse, focus or reorder groups. Drag
rows or headers, or use their up/down controls to reorder siblings. **Reset map**
restores the default order and expansion.

## Find a file or declaration

Enter a file path or a name such as `read_file` in **Find source**. You can also
enter a qualified name such as `Reader::read_file`. The map narrows to matching
files. Expand **Matches** below the map to choose a file or jump to a declaration.
Scroll down in the list to load more matches. You can also focus the list and use
Page Down or End. If loading fails, select **Retry loading matches**. Select a match
to read its source while keeping the other matches available. A file with several
matching declarations has one tile and a row for each declaration.

Select **Match case** to distinguish names such as `Reader` and `reader`. Select
**Regex** to enter a regular expression such as `^read_.*$` for declaration names
or `^src/.*\.rs$` for file paths. Regex also checks file basenames and qualified
declaration names such as `Reader::read_file`. The options work independently.
If a pattern is invalid, Vizzi shows the error below the search field.

Search starts across the repository. If you started inside a directory, select
**In this directory** to narrow the map and list to that subtree. File types selected
in the legend apply to both. Clear the search box to return to the previous map
position. **Clear filters** clears the search text and file-type selections and keeps
your **Regex** and **Match case** settings.

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
declaration and reading position. Returning through the header or reloading in the same
tab restores your current selection, search, and reading position as the start of a
new browsing trail. If the file changed while you were away, Vizzi opens its current
capture. If the file disappeared, Vizzi reports that the previous selection is unavailable.

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
