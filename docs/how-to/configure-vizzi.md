# Save Vizzi launch settings

Use a TOML file to keep launch options for a project without repeating them in your
shell. Select the file explicitly:

```sh
vizzi --config /path/to/vizzi.toml
```

For example, save this as `vizzi.toml` in your project directory:

```toml
root = "."
workers = 1
port = 7474
exclude-source = [".venv", "vendor"]

[language-servers]
pyright = "pyright-langserver"
python-path = "./.venv/bin/python"
```

Paths in the file are relative to its directory. Source scopes such as `.venv` are
relative to `root`; bare executable names use PATH. Omit `cache` to use the usual
platform cache. An explicit cache or investigation file must remain outside the source.
The TOML settings file itself can stay inside the project. From that directory, launch
with `vizzi --config vizzi.toml`.

Override a setting for one run with its normal command-line option:

```sh
vizzi --config /path/to/vizzi.toml --workers 2 --port 8000
```

CLI source-selection rules append to file rules. Changes to the file take effect on
the next launch. Native CLI launches do not automatically discover project or user config files.
See the [configuration reference](../reference/configuration.md) for all accepted keys.

## Reuse VS Code's language providers

For a first connection, follow [Use Vizzi with VS Code](../../editors/vscode/INSTALL.md).
The bundled extension opens an editor tab with **Vizzi: Open Workspace** and
reads an optional `vizzi.toml` from your project root. Default launches need no settings file.
It supplies the current editor connection on every launch, including after VS Code restarts.
You do not need a `vscode-connection` entry; an existing file value is overridden by the
connection selected for that launch.
Rust calls can enter locally installed dependency and standard-library source using the
same editor provider. Files are captured only when reached; no second Rust server starts.

To run the native executable independently, use **Vizzi: Share Language Providers (Advanced)**,
then copy the settings shown in the **Vizzi** Output channel into your TOML file:

```toml
root = "/path/to/my-project"
workers = 1

[language-servers]
vscode-connection = "/path/from/the/vizzi/output/channel.json"
```

Remove managed-server options such as `pyright` or `rust-analyzer` from that file.
Save changed buffers, launch Vizzi with `--config`, and follow calls or find references.
VS Code supplies those results through its existing providers. It may activate a
language extension on the first query; Vizzi does not launch another language server.
For C/C++, keep the explicit source scopes and syntax assignments from the
[language-server guide](configure-language-servers.md#cc-with-clangd); sharing replaces
the semantic provider while source selection remains a Vizzi setting.

The connection file stays in VS Code's extension storage outside the project. Keep its
credential private. Stop sharing with **Vizzi: Stop Sharing Language Providers**.
Sharing again restores the connection on Vizzi's next query. Vizzi reports unavailable
semantic analysis while disconnected and retains its labelled name candidates.

Shared analysis supports repository source and local Rust dependency source. Save modified
buffers and refresh Vizzi when its capture differs from disk. For older captured source
or external source in other languages, use [Vizzi-managed servers](configure-language-servers.md).
Remote VS Code workspaces and other editor plugins are not available yet.
