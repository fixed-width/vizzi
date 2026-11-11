# Browse files and declarations

Choose **Source browser** in the header after [starting Vizzi](run-workspace.md).
This view is useful when you know an area or file rather than a function name.

## Select a file and read its structure

Select a file in the repository map. Its source and declaration outline open together.
Selecting a declaration reveals its lines; kind badges distinguish types, functions and
other declarations. **Line N** is the declaration's starting line.

Use **Find a file** to filter paths. The expandable **Accessible file list** provides a
keyboard-selectable alternative to map picking. The map is bounded; narrow the filter
when it reports that some matching files are not shown.

Drag the map background to pan. Wheel scrolling zooms this map; **Fit map** resets the
framing. The small overview navigator jumps to a different region. Arrow keys move a
focused map or navigator. These gestures differ from the call workspace's background
wheel panning.

Use directory controls to collapse, focus or reorder groups. Breadcrumbs return to
parent areas. Drag rows/headers or use their up/down controls to reorder siblings.
**Reset map** restores the default order and expansion. This changes the view only.

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
