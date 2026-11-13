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
interface remains unchanged. The initial plugin supplies only shared analysis, leaving
navigation/tasks and additional editor integrations to extend it later.
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

The editor captures its file, saved text and UTF-16 word position before opening the tab.
`POST /api/editor/call` checks an explicitly selected snapshot and saved-source witness,
then uses the existing exact call-position index and native enclosing declaration. Its
invocation query accepts only the resulting call ID. The route has a separate 16 MiB
message limit; source text is bounded to 4 MiB and paths stay repository-relative.

The webview shell becomes ready independently of the inner UI. A nonce-bound host message
sets an opaque snapshot/call fragment on the local iframe. Fragment-only navigation keeps
an existing Call workspace document alive; another mode navigates back to its root. The
frontend consumes the fragment, waits for coherent source, adds or reveals the caller,
and uses ordinary call following and destination choices. Neither a second definition
resolver nor an editor-specific call graph is maintained. Repeated launches reuse the
current sharing session while its manifest still belongs to that window; another window's
replacement is detected before reuse.

## Proposed changes

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
  `references`), relative source path, captured query text and UTF-16 position. Replies
  carry a stable session ID, editor change epoch, locations with absolute paths,
  UTF-16 positions and target text, an outside count, or an explicit error. Response
  bytes, source size, result count and concurrent requests are bounded.
- Before and after a query, native verification compares every captured document to
  its current regular file, with bounded reads and no source-root escape. The extension
  rejects dirty or disk-divergent open documents in the workspace, validates query text,
  and rejects any editor/document/configuration/filesystem epoch change during lookup.
  Native admission checks returned target text and positions against captured source.
  Context identities include editor session, epoch and snapshot; old captured versions
  cannot acquire current-editor answers. This is conservative live-editor analysis;
  it cannot freeze an editor's full transitive dependency/configuration universe.
- Read the manifest for every lookup rather than caching its endpoint/token. Never send
  LSP initialization, synchronization, execution or lifecycle messages to editor providers.
  Non-file/external locations are counted as outside rather than retargeted by name.

## Testing and validation

Installed launches use a small Node child-process adapter in `editors/vscode/launcher.cjs`.
It starts the packaged executable without a shell, waits for its loopback launch URL,
forwards diagnostics and owns shutdown. The extension passes the optional project-local
config and explicit root/connection overrides. Default launches retain port 7474 so
browser task storage keeps its existing address; a config file can choose another port.
VSCE packages platform-specific VSIX files with `bin/vizzi` alongside the bridge modules.
The local development host can select a native executable through a machine-level
setting. Distribution does not require downloading or compiling a binary at runtime.

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
