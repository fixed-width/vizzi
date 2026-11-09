# Companion architecture and first slice

This document describes the existing prototype slice. The active product direction is
[visual working sets](../explanation/visual-working-sets.md); search/list navigation is
supporting infrastructure rather than the primary experience.

The [question-search adapter](SEARCH_TECH.md) reuses this workflow through the shared
`vizzi-search` crate. It is an opt-in trial, separate from default metadata-only discovery.

Read [the behaviour contract](PRODUCT.md) first. Source access/analysis, investigations,
visual projection and host integration retain separate interfaces.

The discovery correction after `d647a6e` adds a portable, metadata-only snapshot
projection. It aggregates immediate directory children and file/language counts, or
searches captured declaration names and paths, with bounded paginated results. It reads
neither mutable files nor captured text. Exact file/version/line bindings feed existing
bounded windows and evidence capture; a native handler runs the projection outside the
HTTP event loop. It preserves per-language capabilities and makes no inferred dependency
claim. No new parser, renderer or runtime dependency is needed.

The browser starts read-only indexing on the discovery landing page. Review stays
deferred. Use one optional search input, a clickable area overview and a transient
preview; do not put question/path/line/title/connection forms on the initial screen.
Saved state has a separate view with optional editors disclosed on request. Preserve
stored investigations, stale-source refusal and recording-only editor verification.

`vizzi-core::investigation` owns bounded question/evidence/connection data, mutation
invariants, versioned serialisation and validation. Source anchors use existing workspace,
file, content, document and snapshot identities. The core has no filesystem, process,
HTTP, Git or VS Code dependency. Connections are reader-authored assertions.

A native investigation application captures evidence through current immutable documents
and finite windows. Source text in each stored excerpt is literal and UTF-8 bounded.
Lookup/pin uses exact snapshot/file/version validation. Source freshness is a projection;
it does not mutate saved anchors. The application optionally stores a validated JSON
file atomically outside the source root, keeping it separate from analysis cache data.
Use revision checks to refuse obsolete board edits rather than losing another edit.

The source host verifies a captured location against the current document and a no-follow
working-file read. Direct fragment and saved-evidence handoffs share that verification;
direct handoff leaves investigation state untouched. Editor adapters consume a verified
local location. The first adapter
uses the documented VS Code `--reuse-window --goto file:line:character` CLI:
https://code.visualstudio.com/docs/configure/command-line . It uses an argument vector,
never a shell or a command supplied by an HTTP request. Tests use a recording adapter.
The executable is configured at native launch (default `code`). File verification is bounded; race-free
confirmation of editor buffer contents, including existing unsaved edits, is outside this CLI handoff's guarantee.

A discovery route is the primary page; detailed source exploration stays at
`/explore`, and review stays `/review`. Browser helpers continue to consume bounded native
queries. An automatically populated overview precedes optional saved interpretation;
source previews are temporary panels. Render all source, names and notes as text.
Do not add a diagramming dependency or another parser for this bounded first slice.

Validation follows the interfaces: portable model tests for bounds, identities, links,
removal and roundtrips; native tests for capture/version mismatch, expiry, persistence
failure and working-file changes/replacement; browser tests for constructing a cross-file
investigation, preview, editing notes, linking and handoff refusal. Use safe Rust, TDD and
serial lower-priority mutations. Benchmark interaction separately. The real pinned lpc-rs
workflow must demonstrate a companion contribution, not just another source-browser replay.
