# Native language-server integration

Contributor reference for [the behaviour contract](LANGUAGE_SERVERS_PRODUCT.md).

`definitions::Provider` remains the definition/reference seam. `language_servers::Servers`
routes by native captured language and owns the shared dependency catalog. Rust retains
its Cargo-specific adapter. Python and C-family profiles use a shared captured-project
mirror and lookup implementation. Executables are chosen by native launch configuration,
never browser input.

The stdio client separates initialization options, configuration-section values, process
arguments and readiness. Rust uses its quiescence notification; standard servers do not
wait for a Rust-specific event. Framing, response correlation, UTF-16 conversion, deadlines,
result bounds and refusal of edit/execute requests stay shared.

Project mirrors are bounded and reuse unchanged documents. didOpen carries the actual
language ID; changed/deleted captures use change/close and filesystem notifications.
Versioned diagnostic acknowledgements gate lookups after document changes, so a cached
position cannot be paired with a newer declaration tree. Project-input digests
include the provider profile and frozen metadata. Python JSONC/TOML settings and inherited
configuration preserve project/source and environment paths through translation. Clangd
receives a translated database/flags file; compilation commands are data, not shell execution.

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
