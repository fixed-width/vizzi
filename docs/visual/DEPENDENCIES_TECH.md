# Captured dependency sources

For native/browser contributors. Read [the behaviour contract](DEPENDENCIES_PRODUCT.md).
Use the existing language-provider seam. Add a language-neutral capture catalog, separate
from the repository snapshot and its filesystem watcher. Language providers own discovery
of local source reached through their servers; a definition is the admission witness.
Browser requests use opaque native identities, never filesystem paths.

Rust retains its Cargo adapter. Python/Pyright and C-family/clangd share the project-mirror
adapter described in [language-server integration](LANGUAGE_SERVERS_TECH.md). All providers
use one bounded capture catalog, with language-specific configuration identities.

Managed TypeScript exposes existing installation directories through directory
links at the same relative locations in its private mirror. It probes only
`node_modules` entries along captured source and config ancestors. The repository
index stays unchanged. Managed resolution needs permission to create directory
links. Both TypeScript and the editor bridge use the same native
package admission rules. Canonical files within a workspace installation need a
bounded package manifest and a supported source suffix. npm scopes and pnpm's
workspace-local virtual store preserve their physical package identities. For
an LSP result reported at a mirror alias, retain the path and recheck the alias
against the captured physical file before accepting or continuing from it.
Native sessions track content fingerprints of reached package manifests.
A changed manifest rebuilds resolution before the next request. Restored native
captures reopen the saved importing file to load its project. Editor-backed
captures keep the provider's original URI alongside the canonical source path
and recheck the alias before and after follow-up queries.
[Node package lookup](https://nodejs.org/api/modules.html#loading-from-node_modules-folders) ·
[pnpm installation layout](https://pnpm.io/symlinked-node-modules-structure)

Managed Java retains the virtual URI returned by JDT LS in the current server
session. It uses the server's read-only source attachment query and class document
contents to identify a bounded ZIP member. The provider accepts unnamed-package
URIs and requires a unique exact byte match among members with the reported
relative source path, including paths with source-root prefixes. The catalog
verifies the member's bytes against that content before parsing and stores its
archive path, member name, and content identity for restoration. The host reads
members without extracting them. The catalog stores a caller's path and content
identity or its archive capture ID, plus the source and destination positions.
Vizzi can refresh an expired trail when a new lookup reaches the same source.
It keeps a valid direct repository caller when navigating through another archive.
A fresh JDT LS session replays up to eight retained steps and checks each archive,
entry, content, and library origin before using its new virtual URI.
Source archives and explicitly configured library JARs are limited to 256 MiB each.

VS Code's Java provider returns a `jdt:` document and its local binary and source
attachment paths. The native bridge uses the same exact ZIP member admission as
managed JDT LS. It retains the editor's virtual URI and canonical binary path
in the capture. Follow-up calls and references verify the archive and recheck the
provider's attachment and document text. If the editor URI expires, the reader
can follow the call again. The originating VS Code adapter can open the verified
virtual document and rechecks its attachment and displayed text before selecting
the line. The browser receives only capture IDs and capability flags.
[VS Code Java source attachment protocol](https://github.com/redhat-developer/vscode-java/blob/main/src/standardLanguageClient.ts)

Each capture has its own source scope, snapshot, parsed document and structural projection.
A public origin descriptor identifies package/language/version and the capture reference;
absolute paths stay native. Source windows bind the dependency snapshot. Structural and
definition requests remain guarded by the current repository observation, while source
file/version/context and origin identify the actual captured material.

Capture lazily, with bounded file/total text, declaration counts and retained captures.
Persist bounded native references for reload; restore only matching captures, never guessed
locations. Retention expiry remains explicit. Repository live refresh can rebind a retained
dependency view without changing its bytes. Historical cross-scope links remain unverified.

Rust keeps Cargo discovery offline and build execution disabled. Rewrite known dependency
path entries for the private mirror, preserving internal captured paths and external local
paths. Open captured external documents through LSP synchronization and resolve again
before returning their positions; verify bytes after lookup. Changing project configuration
invalidates semantic use of older dependency captures. Unsupported/generated/missing source
or unstable target sets remain outside/unavailable rather than becoming name matches.

References: [Cargo dependency sources](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html),
[rust-analyzer configuration](https://rust-analyzer.github.io/book/configuration.html),
and [LSP document synchronization](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_synchronization).

The initial catalog retains at most 32 captures, 32 MiB of source and 32,768 declarations,
with a 2 MiB per-file limit. Its bounded reference file (1 MiB) stores native paths and
configuration/source identities, not source bytes. Restore re-reads local source and
requires an identical capture; missing, modified or newly analysed captures can expire.
Worker-local parsers reuse queries; unchanged capture inputs are content-verified and reused.
The cache limits are not a process RSS cap. Dependency module membership can be unknown;
package provenance must not be presented as a guessed language module.

The TypeScript 7.0.2 fixtures exercise nested and scoped packages, package-to-package
calls and references, declaration exports, and reopening a dependency after its
source moves. Catalog tests restore distinct mirror aliases. The native suite,
all 17 Chromium tests, the real VS Code fixture, and the existing Pyright dependency
integration passed. The pnpm layout and escape checks use a local symlink fixture.

Follow-up regressions cover in-place export changes, including edits with an
unchanged version, and references from a fresh provider using restored captures.
The VS Code fixture uses a pnpm-style alias with `preserveSymlinks` enabled.
The editor peer test checks exact URI reuse and retargeting refusal. Workspace,
Clippy, 17 Chromium tests, and real VS Code checks passed. Five focused manifest
tracking mutations were caught after a passing baseline.

`Symbol.origin` is optional for legacy/repository symbols. It carries opaque `id` and
`capture` references, plus `label`, nullable `version` and `language`. Source paths remain
relative to their own scope. A dependency source window uses
`GET /api/dependency/window?snapshot=CAPTURE&file=FILE&version=VERSION` plus the normal
bounded WindowQuery fields. The returned snapshot is the dependency capture. Structural
pages keep the current project observation in their snapshot field and native source
identities in each symbol/call. Live refresh accepts optional `views[].capture`; it keeps
captured bytes and marks cross-scope relationship evidence unverified rather than guessing.

`POST /api/dependency/editor` uses the existing OpenSource shape with the dependency capture
as snapshot, re-verifies file/version/line and bytes, then uses the configured editor and
shared handoff admission. No route accepts a native dependency path. Read-only source and
structural requests retry bounded transient admission failures; editor/writing requests do
not retry. Dependency window work has separate, cancellation-safe bounded admission.

Rust sets an explicit local sysroot source path (or an unavailable private path) so
rust-analyzer cannot take its automatic rust-src installation branch. Rustup automatic
toolchain installation is disabled for its child processes. See the
[upstream sysroot implementation](https://rust-lang.github.io/rust-analyzer/src/project_model/sysroot.rs.html)
and [rustup environment reference](https://rust-lang.github.io/rustup/environment-variables.html).
