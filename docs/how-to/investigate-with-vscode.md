# Save source locations and use VS Code

Saved locations hold source anchors, notes and reader-authored connections. They are
separate from visual task layouts and pinned bubbles.

## Keep locations between server runs

Start Vizzi with a state file outside both the inspected project and its cache:

```sh
./target/release/vizzi \
  --root /path/to/project \
  --investigation-file "$HOME/.local/share/vizzi/my-project/locations.json" \
  --workers 1
```

The parent directory is created when needed. Without `--investigation-file`, locations
belong to the running server session. The [storage reference](../reference/storage.md)
describes which data each storage mechanism retains.

## Save and annotate a location

Choose **Save** on a source bubble, or use **Find source** in the Source browser and
choose **Save location** at the file or declaration.
Then choose **Saved locations** in the shared header. Choose **Add notes** on a saved
card to open its inspector and start typing. Choose **Inspect location** to read the
captured excerpt or edit existing notes. Choose **Save note** to keep your changes.

Choose **Open in Source browser** or **Open in Call workspace** on a saved card to
return to the view where you saved it. Vizzi reveals the saved line in the Source
browser or the bubble in the active Call workspace task. If the source
has changed, the saved card still has its captured excerpt and notes.

**Locations → Add a location** provides a path, line and title form for a location
you already know.

Once two locations exist, **Explain a connection** lets you record why they matter
together. These connections are your assertions, not analysed calls or proof of impact.

**Locations → Export locations** downloads the current saved-location record, including
notes and reader connections. There is no general import UI or visual-task export yet.
For normal persistence, keep using the same configured state file.

## Open source in VS Code

Use **Editor** on a current bubble or **Open in VS Code** on a saved location. With
the plugin, this returns to the captured line in the exact VS Code window that launched
Vizzi. It works from an external browser and after closing the Vizzi tab or stopping
provider sharing. If that window is unavailable, reopen Vizzi from it.

Save changes in the target file first. The plugin refuses dirty target buffers and
checks saved source against the capture before selecting the line; other dirty files
do not block it. Vizzi never saves, reloads or edits buffers for you. Changed, missing
or unverifiable source can be refused while saved excerpts and notes remain available.
Refresh the capture or save a current location before retrying changed source.

Standalone launches use `code` from `PATH`; choose another executable with
`--editor-code /path/to/code`. This CLI adapter requests reuse of an available window
and cannot confirm its unsaved buffers. Run it on the desktop where you want the editor
to open. No editor opens merely from browsing a bubble.

Dependency bubbles can use editor handoff when their bytes still match, but saving a
location from dependency source is not supported in this slice. Other editor integrations
are not implemented yet.
