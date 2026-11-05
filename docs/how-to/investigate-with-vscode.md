# Keep an investigation beside VS Code

Launch Vizzi on a local source checkout, keeping analysis cache and investigation data
outside it:

```sh
cargo run --release -p vizzi -- \
  --root /path/to/repository --cache /tmp/vizzi-analysis \
  --investigation-file /path/to/notes/repository-investigation.json --workers 1
```

The first editor adapter uses `code` from PATH. Use `--editor-code /path/to/code` to
choose another VS Code executable. Opening a location is an explicit button action;
loading an investigation does not launch an editor or start source indexing.

Open the printed URL and write a question. Read source in VS Code as usual. When a
location helps answer the question, choose **Add source evidence**, copy its relative
path and starting line from the editor, and give it a title describing its contribution.
The first capture activates the source index. Wait for indexing if necessary, then keep
the evidence. The title and your interpretation are reader-authored; the source excerpt
is a literal copy from the identified captured document.

Select **Inspect evidence** to add a note or expand its saved source excerpt. Close the
detail when done. Use **Your connection** to connect two locations and explain why they
matter together. The dashed connection and its list entry represent your assertion;
Vizzi does not infer that the source proves a call, dependency or impact relationship.
Remove a connection or evidence when your explanation changes.

**Open in VS Code** checks current indexed and actual working-file content against the
saved source identity. Changed, missing, replaced or unverifiable source is refused;
the saved excerpt and notes stay available. Add fresh evidence and reconsider affected
connections rather than assuming an old line still means the same thing. The CLI opens a
live working file: it cannot inspect an existing unsaved editor buffer or guarantee bytes
shown after launch. This is a location handoff, not a bidirectional plugin handshake.

With an investigation file configured, saves are atomic and survive host restart. Without
one, the page identifies session state; export the investigation before stopping the host
to retain a copy. Export is also useful for sharing the question, captured evidence and
reader assertions. The first slice does not import an arbitrary artifact through the UI.

Source browsing remains at `/explore`, with **Keep as evidence** for the current file or
declaration. It is a source chooser alongside this workflow. Conventional comparison
remains at `/review`; comparison-side evidence and historical editor buffers are later
extensions.
