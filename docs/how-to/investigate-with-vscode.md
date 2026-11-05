# Explore unfamiliar source beside VS Code

Launch Vizzi on a local source checkout, keeping analysis cache and investigation data
outside it:

```sh
cargo run --release -p vizzi -- \
  --root /path/to/repository --cache /tmp/vizzi-analysis \
  --investigation-file /path/to/notes/repository-investigation.json --workers 1
```

The editor adapter uses `code` from PATH. Use `--editor-code /path/to/code` to choose
another VS Code executable. Loading the page starts read-only source indexing. Opening
an editor location is an explicit button action.

Open the printed URL. The overview fills itself with source areas, languages and file
counts. Choose an area to see its contents; use **Repository** and the breadcrumbs to go
back. Example names on an area are literal captured declarations. They are clues to its
contents, not an inferred description of its responsibilities.

You can also use **Find a name or file** with a word such as `reader` or `compile`;
no path is needed. Search matches declaration names and paths, not full source text.
Choose a result to see a short, syntax-coloured preview and names from that file. Choose
a name to move the preview to its declaration, then **Keep this location** to save it.
No line, title or question needs to be entered. Full reading and editing remain in VS Code.

Open **Saved locations** to revisit what you kept. **Inspect evidence** reveals a note
editor and captured excerpt. **Add or edit a question** is optional. Once two locations
exist, **Explain a connection** reveals the controls to explain why they matter together.
Dashed connections and their list entries represent your assertions; Vizzi does not infer
that they prove a call, dependency or impact relationship.

**Open in VS Code** checks current indexed and actual working-file content against the
saved source identity. Changed, missing, replaced or unverifiable source is refused;
the saved excerpt and notes stay available. Add fresh evidence and reconsider affected
connections rather than assuming an old line still means the same thing. The CLI opens a
live working file: it cannot inspect an existing unsaved editor buffer or guarantee bytes
shown after launch. This is a location handoff, not a bidirectional plugin handshake.

With an investigation file configured, saves are atomic and survive host restart. Without
one, the saved-context view identifies session state. **More → Export investigation**
keeps a portable copy of the question, evidence and reader assertions before stopping the
host. The first slice does not import an arbitrary artifact through the UI.

For a location already open in your editor, **More → Add a known source location** keeps
the advanced path/line/title form available. It is not needed to explore an unfamiliar
repository. The overview covers configured source types, not every document or build
configuration; source-only and deferred coverage are shown explicitly.

Detailed source browsing remains at `/explore`, with **Keep as evidence** returning to
saved context. Conventional comparison remains at `/review`; comparison-side evidence
and historical editor buffers are later extensions.
