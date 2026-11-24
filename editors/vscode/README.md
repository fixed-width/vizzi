# Vizzi for VS Code

This extension opens Vizzi in an editor tab using VS Code's definition and reference
providers. Platform-specific VSIX files include the native executable.

Already installed? Right-click a saved call name and choose **Vizzi: Explore Call**,
right-click a file or its tab and choose **Vizzi: Open in Source Browser**, or click
**Vizzi** in the status bar to open an empty workspace.
The [installed-extension guide](INSTALL.md) covers installation, optional project-local
settings and troubleshooting. The workflow below is for extension development and testing.

Captured-source handoff is implemented in the native [handoff module](../../docs/configuration/HANDOFF.md).
Editor observations remain in `editor-source.cjs`; `handoff.cjs` handles finite native
results for both actions. Other editors can implement the same contract in their own
runtime.

Return navigation uses the native `Editor` seam and a separate
[originating-editor connection](../../docs/configuration/ORIGIN_EDITOR.md).
`origin-connection.cjs` owns its authenticated listener; `editor-navigation.cjs` checks
target buffers and presents verified locations through public VS Code APIs. The native
workspace owns this connection independently of semantic sharing and the webview.

Workspace ownership is concentrated in `source-workspace.cjs`, through
[selection, preparation and presentation](../../docs/configuration/WORKSPACE_LIFETIME.md).
`workspace-runtime.cjs` constructs editor-owned resources; commands no longer manage
process generations, sharing replacement or shutdown ordering.

Confirmed view delivery uses the shared native-browser module
`crates/vizzi/web/workspace-delivery.mjs`. The extension's `.mjs` entry is a source
symlink; packaging copies the canonical contents into a regular bundled file. The
[presentation contract](../../docs/configuration/DELIVERY.md) covers prepare/verify/commit,
request/document identities, cancellation and unconfirmed outcomes. The VS Code shell
only establishes trusted MessagePorts and relays generic frames; mode routing belongs
to the native browser module.

The plugin has no npm runtime dependencies. Its minimum VS Code version is 1.100;
it uses public Git and workspace/provider APIs rather than editor-private LSP access.
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
source must match saved files. Shared definitions can open local Rust dependencies,
installed JavaScript and TypeScript packages, and attached Java library and JDK
source in independent captures. VS Code can return to the captured line in an
attached Java source archive. [Configure semantic navigation](../../docs/how-to/configure-language-servers.md)

## Verify changes

With a built native executable, run command, launcher, protocol and provider checks
using Node.js 18 or later:

```sh
npm run test:editor
node --expose-gc --test editors/vscode/*.test.cjs
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

Add `--compare --webview` to check the real Git API, Source Control comparison menu,
base/target Quick Picks and embedded review panes. This uses a temporary repository
with two commits plus staged, unstaged and untracked changes; Git must be on PATH.

Use `--source-browser --webview` to check the editor and inactive-tab context menus,
cursor-line reveal, source-browser document retention, Back/Forward reading positions
and navigation from the Call workspace. It uses an isolated two-file Python fixture.

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

The `--expose-gc` run includes two focused retention checks that the ordinary suite
skips. Exercise lifetime ordering and failure behavior directly through
`source-workspace.test.cjs`; retain the installed-editor checks for command and webview
integration. Run the serial whole-module mutation check with the existing development
Node and Stryker installation:

```sh
nice -n 10 node_modules/node/bin/node node_modules/@stryker-mutator/core/bin/stryker.js run editors/vscode/lifetime-stryker.config.json
```

The mutation report goes to the ignored local browser-results directory. Its threshold
is diagnostic; inspect survivors and timeouts rather than treating the score as a gate.

Run confirmed-delivery and navigation checks through the shared interface:

```sh
node crates/vizzi/web/delivery-tests.mjs
nice -n 10 node_modules/node/bin/node node_modules/@stryker-mutator/core/bin/stryker.js run experiments/browser/delivery-stryker.config.json
```

See [delivery validation](../../docs/development/delivery-validation.md) for mutation
survivors, timeouts and installed-editor qualification limits.
