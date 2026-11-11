# Run Vizzi on your project

Use this guide after [building Vizzi](install.md). The project can be a plain source
directory; Git is only required for revision review. Rust and Python are selected by
default. Vizzi reads files saved on disk, not unsaved editor buffers.

## Start the local server

From the Vizzi checkout:

```sh
./target/release/vizzi \
  --root /path/to/project \
  --cache "$HOME/.cache/vizzi/my-project" \
  --workers 1
```

Replace the project and cache paths. The cache must be outside the inspected project,
including through symlinks. Give different projects separate cache directories.

For a Python project with a virtual environment inside the root, add
`--exclude-source .venv` (or its actual directory). Repeat that option for other generated
or vendored directories you do not want to explore. Vizzi uses its own source policy;
being ignored by Git does not by itself define the reading scope.

Open the printed address, normally `http://127.0.0.1:7474/`. Wait for **Source ready**,
then use **Add bubble** to find a function. [Follow calls](follow-a-call-trail.md) or
[manage a reading task](manage-tasks.md) from there.

The server binds to loopback. If the port is occupied, add `--port 7475` and use the new
printed address. Use a stable address and port when returning to saved browser tasks.
Stop the server with Ctrl-C. The [CLI reference](../reference/cli.md) lists all options.

## Choose a view

Use the shared header:

- **Call workspace** keeps functions, branches and result stacks together on a canvas.
- **Review changes** compares a working tree or two existing Git commits.
- **Source browser** shows the repository map, files and declaration outlines.
- **Saved locations** holds source anchors, notes and reader-authored connections.

Opening the call workspace or a browsing view starts live indexing. To begin with a
scoped review before live indexing, open the printed address with `/review` appended.
Once indexing has started, it continues until the server stops.

**Settings → Theme** uses the system setting by default. Choose Light or Dark for an
override shared by the views at this server address. Dismiss transient banners with
their × button; dismissing a message does not cancel its operation.

## Add optional capabilities

- [Configure language servers](configure-language-servers.md) for Python, Rust and C/C++
  type-aware calls, references and locally available dependency source.
- [Keep saved locations between server runs](investigate-with-vscode.md) with an external
  `--investigation-file`. Visual task layouts already use browser storage.
- [Enable C/C++ source scopes](browse-source.md#show-cc-source) for raw source browsing.

Vizzi does not build the inspected project. Analysis support varies by language and
configuration; see [language coverage](../reference/languages.md). If startup or source
selection is unexpected, use [troubleshooting](troubleshoot.md).
