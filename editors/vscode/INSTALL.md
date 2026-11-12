# Vizzi for VS Code

Share definitions and references from your existing VS Code window with Vizzi's visual
workspace. VS Code owns the language servers and their configuration. Vizzi does not
launch another language server when an editor connection is selected.

## Install the preview

From the Vizzi source checkout, build the current native host and extension with Rust,
Python 3, Node.js 22 or later, and npm available:

```sh
cargo build --locked --release -p vizzi
python3 editors/vscode/package.py
code --install-extension dist/vizzi-vscode-0.1.0.vsix
```

The build uses Microsoft's VSCE 4.1.0 packager and includes the project licenses. The
VSIX installs locally; a Marketplace account or publication is not required. In the
VS Code window that already has your project open, run **Vizzi: Share Language Providers**
from the Command Palette and select the project folder.

## Connect Vizzi

Copy the settings from the **Vizzi** Output channel into a TOML file, then launch:

```sh
./target/release/vizzi --config /path/to/vizzi.toml
```

The generated settings have this shape:

```toml
root = "/path/to/project"

[language-servers]
vscode-connection = "/path/from/the/vizzi/output/channel.json"
```

Keep your C/C++ source-selection options if needed. Remove managed-server options such
as `rust-analyzer` or `pyright` when selecting the connection. Save modified buffers
before querying from Vizzi. **Vizzi: Stop Sharing Language Providers** closes sharing
without stopping VS Code's servers. Sharing again reconnects the next Vizzi query.

The preview supports trusted local workspaces and repository-local captured targets.
It reports unavailable analysis for dirty buffers, mismatched saved source and dropped
connections. It does not automatically start a fallback server. VS Code may activate
its usual language extension on the first provider request.

Official background: [installing VSIX extensions](https://code.visualstudio.com/docs/configure/extensions/extension-marketplace#_install-from-a-vsix)
and [packaging extensions](https://code.visualstudio.com/api/working-with-extensions/publishing-extension).
