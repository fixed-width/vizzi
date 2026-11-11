# Dependency source in the working set

For readers following a call beyond the selected repository. A definition supplied by a
supported language provider can open local dependency source beside its caller. Capture
only reached files; do not scan or add dependency trees to the repository map/search.
Multiple genuine targets still use the chooser. Missing source stays unavailable.

Show a dependency/package label and version when known. Source, line numbers, styling,
reflow, copying, paging, call actions, layout and minimap navigation work as for repository
bubbles. Keep source identities separate even when relative filenames coincide.

Dependency bubbles retain captured bytes. Opening a definition again may capture a newer
version. Rebinding the repository observation must not silently substitute new dependency
bytes. The initial slice does not watch dependency directories. Explain this through the
captured dependency status; keep the existing editor handoff when its bytes still match.
Saved-location support remains repository-only in this slice; the visual layout retains
dependency references. Reload restores retained captures/references where still available;
expired or changed source remains explicit.

Configured [Pyright/clangd providers](LANGUAGE_SERVERS_PRODUCT.md) also admit locally
reached Python/stub and C/C++ source through the shared catalog. Rust uses the opt-in
rust-analyzer provider: cached registry/Git sources,
local path dependencies and installed standard-library source when the server can resolve
them. Keep local Cargo path dependencies valid in the private mirror. No fetches, downloads,
repository builds, build scripts or procedural macros. Other languages need their own
definition/discovery support; do not infer dependencies from arbitrary directories.

Validate exact source/location binding, dependency-to-dependency call steps, same-named
files, stale replies/configuration, unavailable sources, bounded retention, reload and
verified editor handoff with small fixtures and a real installed provider.

Declaration extraction remains a capability limit. In particular, some recent Rust
`[const]` trait-bound syntax is not recognized by the pinned Tree-sitter grammar even
when rust-analyzer resolves its location. Such a target stays outside admitted declarations;
it must not open an unrelated same-name function. Dependency module labels can also be
unavailable; package provenance is shown independently of module membership.
