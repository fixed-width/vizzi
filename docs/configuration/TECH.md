# Configuration and editor bridge implementation

Contributor implementation and validation reference for [the product contract](PRODUCT.md).

## Context

Inspected baseline: `3cc0677d1f06ee043354e087268d2acb9bacf3b2`.
`crates/vizzi/src/main.rs` parses native launch flags; `language_servers::Servers` routes
private captured-source adapters behind `definitions::Provider`. `lsp::Client` owns a
child's stdio and lifecycle. Attaching another client to that stdio would interfere with
initialization, request IDs, document synchronization and shutdown.

VS Code already exposes definition/reference provider commands. A plugin can use those
without assuming a particular language-server transport. The editor-neutral Provider
interface remains unchanged. Editor-owned provider invocation remains separate from captured-source handoff and
presentation, so another editor adapter can reuse native behaviour in its own runtime.
The [versioned connection reference](BRIDGE.md) is the contract for other editor adapters.

## Editor-owned Rust dependency captures

The shared bridge and language router use the same bounded dependency catalog. External
`.rs` definitions returned by VS Code are checked against disk, assigned native Cargo
package provenance and captured independently of repository source. Excluded files within
the selected root remain excluded. Subsequent dependency calls and references use distinct
protocol methods and canonical paths obtained only from retained native captures; older
adapters reject these methods instead of interpreting them as repository-relative paths.

VS Code can return a function's document-symbol range and identifier with an external
definition. If the native grammar misses the identifier, the catalog validates that
range, content digest and identifier witness and retains a primary provider-range analysis.
Raw syntax analysis and colours remain captured separately; missing raw call analysis is
explicit. This admits current Rust `[const]` methods without rewriting their source or
adding a compiler/server. Persisted references retain the optional checked range, and
restoration still requires identical contents and analysis identities.

## Editor call handoff

Editor entry now uses the native [captured-source handoff module](HANDOFF.md).
`handoff.rs` owns window-scoped ordering, compact saved-source identities, fixed deadlines,
capture catch-up and bounded projection work. Its finite HTTP and stdio adapter has no
readiness wait under investigation work admission. `editor-source.cjs` owns VS Code
observations; `handoff.cjs` supplies one generic finite-result loop for both actions.
Native source matching and retry classification are no longer duplicated in the plugin.
The existing `/api/editor/call` binding/retrieval route remains for browser call following
and earlier native clients, sharing the exact call-position implementation.

The editor captures its resource and saved text before focus changes. The native window
scope orders handoffs independently of provider connection ownership. A successful result
contains only captured identities and a reading position. The webview shell forwards those
through its nonce-bound existing call/file fragments, retaining call-trail geometry and
source-browser Back/Forward behaviour. Presentation checks the editor's current intent;
obsolete startup or native replies do not open another view or show an error.
Startup and provider-sharing failures propagate to the command that owns reporting;
source commands apply their current-intent check before displaying errors. A new eligible
open can proceed after an earlier failure; operations sharing an in-flight launch
observe the same original failure, and a subsequent explicit open can retry. A same-root
operation captures and joins that launch before refreshing sharing for the ready session.
The webview delivery step revalidates source after readiness, then rechecks cancellation
and panel identity before posting.

## Source-workspace lifetime

`source-workspace.cjs` owns native and semantic-sharing lifetimes behind `use`, `stop`
and `dispose`. Separate ownership records and cancellation scopes hide startup reuse,
root replacement, stale exits and cleanup ordering from commands. Operations register
before selection, then prepare and present only while current. The
[workspace lifetime contract](WORKSPACE_LIFETIME.md) specifies Stop/latest-project rules.
`workspace-runtime.cjs` constructs the VS Code resources; `launcher.cjs` and
`workspace-view.cjs` keep their concrete process and presentation responsibilities.
Tests exercise the lifetime interface directly with deferred factories, retaining
command-wiring checks and real child/socket/installed-editor validation.

## Originating-editor return

The native `Editor` seam now carries a SHA-256 saved-byte witness alongside a verified
absolute path and captured line. Source admission remains in the native source host.
Plugin launches select `OriginEditor` through a private, command-line-only descriptor;
standalone launches retain their configured CLI adapter. The native adapter reads and
pins the launch connection once, independently of the semantic bridge's root-wide alias.
The launcher owns the callback through tab closure, external-browser use and Stop Sharing.
The VS Code navigator checks only target buffers, then revalidates after asynchronous
presentation before synchronously selecting the captured line. Other editor plugins can
implement the same [originating-editor contract](ORIGIN_EDITOR.md).

The Neovim Lua plugin uses the same launch argument and protocol. It pins the split ID
at launch, supervises the child, and owns its loopback listener and private descriptor.
The adapter compares raw saved bytes and loaded file buffers before and after presenting
the buffer. The final cursor move follows the last check without yielding to Neovim's
event loop. The workspace opens in the default browser.

After building the native executable, run `node --test editors/neovim/companion.test.mjs`
from the checkout. The test launches two headless Neovim instances and real native
workspaces. Set `VIZZI_NVIM_EXECUTABLE` or `VIZZI_TEST_BINARY` to override the
executables. With the repository's Playwright dependencies and Chromium installed,
set `VIZZI_NVIM_BROWSER=1` to exercise the browser's **Editor** button.

## Launch configuration and editor connection

- Move launch parsing into a binary-private `configuration` module. Deserialize a strict
  optional TOML layer, resolve its paths against its directory, then apply existing CLI
  flags. Keep diagnostic flags out of persisted settings. Bound config input to 1 MiB.
  Validate ownership selection before creating caches. Reuse existing TOML/serde dependencies.
- Add `vscode_connection` to native language-server configuration. A selected connection
  routes all definition/reference requests through an `editor_bridge` Provider, with no
  managed adapters constructed. Reject managed-server/interpreter/build options in that
  mode. Configuration files are explicit to avoid discovering executable launch settings
  merely by inspecting a repository.
- The VS Code extension lives in `editors/vscode`, with no runtime npm dependencies.
  `package.py` stages its runtime files and project licenses for VSCE 4.1.0, producing
  a locally installable VSIX for use in the existing editor window.
  A Node TCP listener binds `127.0.0.1` on a random port. Its per-session manifest contains
  protocol version, canonical root, port and a random 256-bit token. The manifest is
  written with private permissions under extension global storage, outside the source.
  Requests use one bounded newline-delimited JSON frame per TCP connection. Native reads,
  writes and connection setup have deadlines. Neither side exposes arbitrary commands.
- Protocol v1 requests carry the token, version, root, method (`definition` or
  `references`), relative source path, captured query text and UTF-16 position.
  Dependency queries use an absolute path rechecked against an admitted capture. Replies
  carry a stable session ID, editor change epoch, locations with absolute paths,
  UTF-16 positions and target text, an outside count, or an explicit error. Response
  bytes, source size, result count and concurrent requests are bounded.
- Before and after a query, native verification compares every captured document to
  its current regular file, with bounded reads and no source-root escape. The extension
  rejects dirty or disk-divergent open documents in the workspace, validates query text,
  and rejects any editor/document/configuration/filesystem epoch change during lookup.
  Native admission checks returned target text and positions against captured source.
  Browse-only JSON is verified by streaming saved bytes under its own file-count and
  aggregate byte limits. The 4 MiB query and target limit applies to transmitted source.
  Context identities include editor session, epoch and snapshot; old captured versions
  cannot acquire current-editor answers. This is conservative live-editor analysis;
  it cannot freeze an editor's full transitive dependency/configuration universe.
- Read the manifest for every lookup rather than caching its endpoint/token. Never send
  LSP initialization, synchronization, execution or lifecycle messages to editor providers.
  Native admission accepts reached Rust dependencies and supported JavaScript and
  TypeScript source in canonical workspace installations and verified Java source
  attachments reached through `jdt:` documents. Other external and non-file locations
  are counted as outside. An editor package location can supply its original
  provider URI. The bridge retains it in the capture and verifies its canonical
  destination on follow-up requests. [Dependency source contract](../visual/DEPENDENCIES_TECH.md)

## Testing and validation

`comparison.cjs` uses the built-in `vscode.git` extension's public API v1 for local
repository discovery, bounded history pages and commit resolution. Source Control
arguments select their own repository; ambiguous palette invocations ask the user.
The selected repository must intersect an open local workspace folder. Canonical
repository roots override the ordinary folder launch root, including subfolder opens.
Selections become immutable hashes before launch. POST `/api/comparisons` starts the
existing native capture; `/review` consumes its ordinary status/event stream. The
webview accepts only a nonce-bound fixed review-navigation message, without arbitrary
paths or browser-provided Git operations. No new native comparison API is needed.

Stable API constraint checked against VS Code's
[menu declarations](https://github.com/microsoft/vscode/blob/main/src/vs/workbench/services/actions/common/menusExtensionPoint.ts):
`scm/historyItem/context` still requires `contribSourceControlHistoryItemMenu`.
The [public Git API](https://github.com/microsoft/vscode/blob/main/extensions/git/src/api/git.d.ts)
supports the read operations used here. Picker/command tests cover pinning, direction,
cancellation, repository context, failures and review-tab reuse; an isolated editor
host verifies the actual Git API and embedded review UI.

The extension requires VS Code 1.100 or later: paged history uses `LogOptions.skip`,
which is present in that release's public Git API and absent in the previous 1.85
minimum. No proposed APIs or additional runtime dependencies are used.

Installed launches use a small Node child-process adapter in `editors/vscode/launcher.cjs`.
It starts the packaged executable without a shell, waits for its loopback launch URL,
forwards diagnostics and owns shutdown. The extension passes the optional project-local
config and explicit root/connection overrides. Default launches retain port 7474 so
browser task storage keeps its existing address; a config file can choose another port.
VSCE packages platform-specific VSIX files with `bin/vizzi` alongside the bridge modules.
The local development host can select a native executable through a machine-level
setting. Distribution does not require downloading or compiling a binary at runtime.

The shared browser/host [delivery module](DELIVERY.md) owns action routing, readiness,
source-bound preparation, cancellation and explicit outcomes. The editor rechecks saved
source after preparation and before a document-bound commit. Native pages use fresh
MessagePorts established from the exact iframe and loopback origin; the outer-shell
nonce remains private to the editor/outer shell. Mode changes carry only an opaque
continuation identity, with the active action retained in the parent. No server broker
or persisted request queue is added. Legacy fragments enter the same controller.

`workspace-view.cjs` owns a single webview panel for the local UI. The panel embeds the
loopback server with a restricted frame origin and retains the complex reading UI when
hidden. Revealing an existing panel does not replace its HTML. Closing it keeps the native
process available; Stop Workspace, a project change or native exit disposes the panel.
An explicit browser command uses the same server. Remote workspaces remain unsupported.
Native page CSP permits only `vscode-webview:` and `vscode-file:` frame ancestors, the
desktop editor's frame chain. Ordinary HTTP/HTTPS parents remain excluded. An isolated
VS Code test uses Playwright to render the real frame, follow a call and restore a saved
task after closing and reopening the panel; Linux runs use a virtual display by default.

Command tests cover invocation without a folder argument, missing folders, visible
failure/cancellation and the browser-launch lifecycle. Adapter tests use real child
processes; the real extension-host check also invokes sharing through its no-argument
command path. Packaging checks verify the executable, documentation and license bytes.

- Behaviour 1–5: CLI integration tests cover no-file defaults, explicit/relative files,
  paths, PATH executables, scalar overrides, source-rule append, strict parsing, missing
  arguments, help and fail-before-storage validation.
- Behaviour 6–8, 10–12: deterministic native bridge peers cover definitions/references,
  authentication/root/version validation, text/position mismatch, old captures,
  disconnection, malformed/oversized frames, external locations and endpoint replacement.
- Behaviour 7–12: Node tests exercise the plugin query interface with injected editor
  providers and real local sockets, including dirty buffers, edits/config changes during
  lookup, authentication, UTF-16, result bounds and session invalidation when stopped.
- Run formatting, workspace tests/doctests and Clippy. Run focused cargo-mutants at one
  low-priority worker/compiler task with serial tests; investigate surviving mutations.
  A real VS Code extension-host run is a separate integration check; mocked commands
  cannot establish compatibility with installed language extensions.

Protocol background: [LSP 3.17](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
and [VS Code provider commands](https://code.visualstudio.com/api/references/commands).

Recorded checks and their limits are in the [validation record](../development/configuration-validation.md).
