# Captured dependency sources

For native/browser contributors. Read [the behaviour contract](DEPENDENCIES_PRODUCT.md).
Use the existing language-provider seam. Add a language-neutral capture catalog, separate
from the repository snapshot and its filesystem watcher. Rust owns discovery of source
files reached through rust-analyzer; a language-server definition is the admission witness.
Browser requests use opaque native identities, never filesystem paths.

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
