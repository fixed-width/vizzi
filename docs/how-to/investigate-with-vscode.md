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

Choose **Save** on a repository source bubble, or **Save location** in the Source browser.
Then choose **Saved locations** in the shared header. Open **Inspect location** on a
saved card to read its captured excerpt or edit your interpretation and choose **Save note**.

To find another location, choose **Explore repository** within this view. Select a source
area or use **Find a name or file**, inspect a result and choose **Save location**.
This lookup matches names and paths, not full source text. **Locations → Add a location**
provides a path/line/title form for a location you already know.

Once two locations exist, **Explain a connection** lets you record why they matter
together. These connections are your assertions, not analysed calls or proof of impact.

**Locations → Export locations** downloads the current saved-location record, including
notes and reader connections. There is no general import UI or visual-task export yet.
For normal persistence, keep using the same configured state file.

## Open source in VS Code

Vizzi uses `code` from `PATH`. To choose another executable, add:

```sh
--editor-code /path/to/code
```

Use **Editor** on a current bubble or **Open in VS Code** on a saved location. The
launcher requests the existing VS Code window when available. Run Vizzi on the desktop
where you want the editor to open; a browser connection is not a remote-editor bridge.
No editor is launched merely by opening a bubble.

Vizzi verifies the current working-file content against the source location before
handoff. Changed, missing or unverifiable files can be refused while the saved excerpt
and notes remain available. Save the current location again when that is what you intend
to work on. This opens a disk file, not an unsaved or historical editor buffer.

Dependency bubbles can use editor handoff when their bytes still match, but saving a
location from dependency source is not supported in this slice. Other editor integrations
are not implemented yet.
