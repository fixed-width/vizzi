# Vizzi for VS Code

This extension opens Vizzi in an editor tab using VS Code's definition and reference
providers. Platform-specific VSIX files include the native executable.

Already installed? Right-click a saved call name and choose **Vizzi: Explore Call**,
or click **Vizzi** in the status bar to open an empty workspace.
The [installed-extension guide](INSTALL.md) covers installation, optional project-local
settings and troubleshooting. The workflow below is for extension development and testing.

The plugin has no npm runtime dependencies. Its minimum VS Code version is 1.85;
it uses established workspace/provider commands rather than editor-private LSP access.
Remote and virtual workspaces are not supported in this slice.

## Use the development extension

From the Vizzi checkout, open your project in an Extension Development Host:

```sh
code --extensionDevelopmentPath="$PWD/editors/vscode" /path/to/project
```

To test **Open Workspace** from source, build the native executable and set
**Vizzi: Executable Path** in that window to its absolute path. A packaged extension
uses its included executable by default. The manual sharing workflow remains available:

1. Enable your usual language extension in that window.
2. Run **Vizzi: Share Language Providers (Advanced)** from the Command Palette and select your
   project folder. A trusted local workspace is required.
3. Copy the settings from the **Vizzi** Output channel into `vizzi.toml` in your
   project, then launch `vizzi --config /path/to/project/vizzi.toml`.
4. Save modified buffers before querying definitions/references from Vizzi.

Run **Vizzi: Stop Sharing Language Providers** to stop the connection. This leaves
VS Code's language servers running. Sharing again publishes a new session at the same
connection path; Vizzi reconnects on its next query. If two windows share the same root,
the last sharing command selects the provider window for that connection path.
Stopping leaves the last endpoint file in extension storage so saved launch paths remain
stable. It cannot authenticate to a stopped bridge, and the next sharing command replaces it.

Vizzi never starts a managed fallback server for an unavailable editor connection.
VS Code may activate its normal language extension when a provider is queried. Captured
source must match saved files. Shared Rust definitions can open locally installed
dependency and standard-library functions in independent captures. Other external languages
remain outside results. Use [managed language servers](../../docs/how-to/configure-language-servers.md)
when you need captured older source or external source in other languages.

## Verify changes

With a built native executable, run command, launcher, protocol and provider checks
using Node.js 18 or later:

```sh
npm run test:editor
```

Launcher tests require a built `target/release/vizzi` (override with `VIZZI_TEST_BINARY`).
The real extension-host check uses isolated VS Code user/extension directories and a
temporary Python fixture. Build `vizzi` first, then run:

```sh
cargo build --locked -p vizzi
node editors/vscode/test-extension-host.cjs
```

It requires `code` on PATH, plus `xvfb-run` on Linux. Linux checks use a virtual display
by default; add `--visible` only when you want to inspect the test window. Override the
executable paths with `VIZZI_VSCODE_EXECUTABLE` and `VIZZI_TEST_BINARY` if needed.
On Unix, add `--alias` to check a workspace opened through a symlink. Add `--webview`
to render the real editor tab, follow a call and check saved reading state through
Playwright (`npm ci` supplies it).
Set `VIZZI_TEST_EXTENSION_PATH` to an installed extension directory to test packaged
payload files instead of the source directory.

The check registers deterministic VS Code providers and queries them through the real
native host. It verifies the editor command interface and protocol integration; it does
not qualify every installed language extension's indexing behaviour.

To qualify actual Rust standard-library navigation, point at a locally installed
rust-analyzer extension directory (the test copies it into an isolated profile):

```sh
VIZZI_RUST_ANALYZER_EXTENSION=/path/to/rust-lang.rust-analyzer-version-platform \
  VIZZI_TEST_BINARY="$PWD/target/release/vizzi" \
  node editors/vscode/test-extension-host.cjs --rust --webview
```

This requires local `rust-src`. It uses offline Cargo and disables checks, build scripts
and proc macros in the test profile. It resolves `Option::ok_or_else` with the real
VS Code provider, opens the captured `core` function and restores its saved task.

Add `--editor-call` to the real Rust `--rust --webview` check to activate Explore Call
from the actual editor context menu, then verify repeated navigation and layout retention.
