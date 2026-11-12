# Vizzi for VS Code

This first plugin slice shares VS Code's definition and reference providers with a
local Vizzi process. It provides the connection for further Vizzi editor integration;
navigation, task synchronisation and plugins for other editors remain future work.

For use in your existing editor window, [build and install the preview VSIX](INSTALL.md).
The development-host workflow below is for extension development and isolated testing.

The plugin has no npm runtime dependencies. Its minimum VS Code version is 1.85;
it uses established workspace/provider commands rather than editor-private LSP access.
Remote and virtual workspaces are not supported in this slice.

## Use the development extension

From the Vizzi checkout, open your project in an Extension Development Host:

```sh
code --extensionDevelopmentPath="$PWD/editors/vscode" /path/to/project
```

1. Enable your usual language extension in that window.
2. Run **Vizzi: Share Language Providers** from the Command Palette and select your
   project folder. A trusted local workspace is required.
3. Copy the settings from the **Vizzi** Output channel into a TOML file outside the
   project, then launch `vizzi --config /path/to/vizzi.toml`.
4. Save modified buffers before querying definitions/references from Vizzi.

Run **Vizzi: Stop Sharing Language Providers** to stop the connection. This leaves
VS Code's language servers running. Sharing again publishes a new session at the same
connection path; Vizzi reconnects on its next query. If two windows share the same root,
the last sharing command selects the provider window for that connection path.
Stopping leaves the last endpoint file in extension storage so saved launch paths remain
stable. It cannot authenticate to a stopped bridge, and the next sharing command replaces it.

Vizzi never starts a managed fallback server for an unavailable editor connection.
VS Code may activate its normal language extension when a provider is queried. Captured
source must match saved files, and results outside captured repository source remain
outside results. Use [managed language servers](../../docs/how-to/configure-language-servers.md)
when you need captured older source or dependency-source navigation.

## Verify changes

Run deterministic protocol and provider checks with Node.js 18 or later:

```sh
node --test editors/vscode/bridge.test.cjs editors/vscode/providers.test.cjs
```

The real extension-host check uses isolated VS Code user/extension directories and a
temporary Python fixture. Build `vizzi` first, then run:

```sh
cargo build --locked -p vizzi
node editors/vscode/test-extension-host.cjs
```

It requires `code` on PATH, plus `xvfb-run` on Linux without a display. Override the
executable paths with `VIZZI_VSCODE_EXECUTABLE` and `VIZZI_TEST_BINARY` if needed.
On Unix, add `--alias` to check a workspace opened through a symlink.
Set `VIZZI_TEST_EXTENSION_PATH` to an installed extension directory to test packaged
payload files instead of the source directory.

The check registers deterministic VS Code providers and queries them through the real
native host. It verifies the editor command interface and protocol integration; it does
not qualify every installed language extension's indexing behaviour.
