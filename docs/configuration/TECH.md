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
