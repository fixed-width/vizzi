# Source browser search

Find a captured file or declaration and open it in the Source browser. The repository
map and the Matches list show the same search at file and declaration granularity.

## Behaviour

1. **Find source** matches captured paths and declaration names as the reader types.
   Default matching is case-insensitive and supports partial names and lexical
   qualification. **Match case** and **Regex** are independent options. A regular
   expression can match a repository-relative path, basename, simple declaration
   name or displayed qualified name. A pattern error appears beside the search
   field and keeps the current source selection. All captured declaration kinds
   are eligible, including module declarations.
2. Search covers the repository by default. **In this directory** uses the directory
   where the reader began searching when that directory is available. Map file-type
   filters also apply to matches. Clearing the search restores the previous directory,
   page and map position while leaving the reader at the source they opened.
   **Clear filters** is available when there is search text or a file-type selection.
   It clears both and keeps the search options.
3. The map shows one tile per matching file, with paging where required. The compact,
   collapsible **Matches** list appears below it during search. Scrolling near the
   end of the list appends more matches, one row per matching file path or declaration.
   Literal searches rank exact names ahead of partial matches. Each declaration row
   displays its kind, qualified name, path and line.
4. Selecting a tile opens its file; selecting a row reveals its exact captured file
   or declaration with the file selected on the map. Both actions enter the reading
   trail for a new location and keep the results available. If the selected tile is
   below the map's visible depth, the map focuses its containing directory. A result
   superseded by a source publication cannot open a different declaration.
5. Saved locations opens directly at `/investigate` and `/investigate#saved`. Its empty
   state links to the Source browser. Existing locations, notes, connections and editor
   actions remain accessible. The structural trial remains available at
   `/investigate?trial=structure`.
