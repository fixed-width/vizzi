# Save Vizzi launch settings

Use a TOML file to keep launch options for a project without repeating them in your
shell. Select the file explicitly:

```sh
vizzi --config /path/to/vizzi.toml
```

For example, save this beside your project directory:

```toml
root = "./my-project"
workers = 1
port = 7474
exclude-source = [".venv", "vendor"]

[language-servers]
pyright = "pyright-langserver"
python-path = "./my-project/.venv/bin/python"
```

Paths in the file are relative to its directory. Source scopes such as `.venv` are
relative to `root`; bare executable names use PATH. Omit `cache` to use the usual
platform cache. An explicit cache or investigation file must remain outside the source.

Override a setting for one run with its normal command-line option:

```sh
vizzi --config /path/to/vizzi.toml --workers 2 --port 8000
```

CLI source-selection rules append to file rules. Changes to the file take effect on
the next launch. Vizzi does not automatically discover project or user config files.
See the [configuration reference](../reference/configuration.md) for all accepted keys.

## Reuse VS Code's language providers

Install the [Vizzi preview extension](../../editors/vscode/INSTALL.md) in your existing
VS Code window. Run **Vizzi: Share Language Providers**, select your folder, and
copy the settings shown in the **Vizzi** Output channel into your TOML file:

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

This first plugin slice supports local repository source. Save modified buffers and
refresh Vizzi when its capture differs from disk. For older captured source or local
dependency-source navigation, use [Vizzi-managed servers](configure-language-servers.md).
Remote VS Code workspaces and other editor plugins are not available yet.
