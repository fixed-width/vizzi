# Native language-server integration

Contributor reference for [the behaviour contract](LANGUAGE_SERVERS_PRODUCT.md).

`definitions::Provider` remains the definition/reference seam. `language_servers::Servers`
routes by native captured language and owns the shared dependency catalog. Rust retains
its Cargo-specific adapter. Python, Ruby, C-family, Java and TypeScript profiles use a shared captured-project
mirror and lookup implementation. Executables are chosen by native launch configuration,
never browser input.

Each managed profile declares its source languages and owns project inputs, startup
settings, external source admission, and result rules. The shared project module owns
the private mirror, document synchronization, and captured-source verification.

An optional native connection file selects `editor_bridge::Bridge` behind the same
Provider interface for all supported languages, including Java, JavaScript, TypeScript
and TSX. Managed options are then rejected before
startup. The [configuration contract](../configuration/TECH.md) defines transport,
ownership, source checks and the VS Code plugin. The remainder of this document covers
Vizzi-managed servers and their captured mirrors.

The stdio client separates initialization options, configuration-section values, process
arguments and readiness. Rust uses its quiescence notification; standard servers do not
wait for a Rust-specific event. Framing, response correlation, UTF-16 conversion, deadlines,
result bounds and refusal of edit/execute requests stay shared.

Project mirrors are bounded and reuse unchanged documents. didOpen carries the actual
language ID; changed/deleted captures use change/close and filesystem notifications.
Python/C-family versioned diagnostic acknowledgements gate changed-document lookups.
Ruby LSP uses its indexing progress, full-capture reopen and an ordered document-symbol
request; this avoids sending unsupported un-ranged changes to its incremental-only API.
Ruby reference locations are checked against unique definitions and declarations are
excluded. Ruby bodies take precedence over accompanying RBS signatures. Project-input
digests include the shared input implementation, the provider profile, and frozen metadata.
An input digest change can require reopening a dependency from its caller for semantic lookup.
Python JSONC/TOML settings and inherited configuration preserve project/source and
environment paths through translation. Clangd
receives a translated database/flags file; compilation commands are data, not shell execution.
The TypeScript profile supplies language IDs for JSX and TSX, consumes captured
project settings and reopens the project when relevant source or settings
change. [Managed TypeScript profile](../prototype/ECMASCRIPT_TECH.md#managed-typescript-server)

JDT LS waits for `language/status` with `ServiceReady`. Its Eclipse configuration
and workspace state are temporary siblings of the source mirror. The profile
passes only source roots and disables project import and automatic builds. The
same roots bound mirror admission and its input identity. Java source edits
inside those roots restart the server and refresh results for unopened files.
Virtual Java destinations can enter the dependency catalog through a verified
local source archive. LSP positions recognize LF, CRLF, and CR line endings in
captured bytes.

Semantic locations map to exact captured identifiers. Returned external local files enter
the bounded catalog, are synchronized and verified, and lookup repeats before trusting a
new capture. Captures evicted by the catalog become outside results rather than unrelated
local choices. Retired external documents close in the server; semantic context includes
known external versions. No analysis claims runtime dispatch.

Native Python project metadata, Cargo lockfiles and enabled C-family build metadata are
captured for change detection. Python stubs participate in source capture. C/C++ call
facts and declaration identifiers use the shared source-position model; raw declarations
do not become semantic bindings by name. Compilation coverage remains separate from raw
syntax classification.

The deterministic client peer checks configuration namespaces and a server without Rust
status messages. Real Pyright/clangd fixtures cover conflicting receiver names, imports,
function-value references, captured/live byte divergence, local dependencies, header
declarations and C/C++ definitions. The browser integration checks the same navigation
and incremental reference behaviour through the public interface.
Set `VIZZI_JDTLS` to a JDT LS launcher to include real Java checks in the native
language-server tests and the Chromium suite.
