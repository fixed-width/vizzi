# Companion architecture and first slice

Read [the behaviour contract](PRODUCT.md) first. Source access/analysis, investigations,
visual projection and host integration retain separate interfaces.

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

An editor module verifies a saved location against the current document and a no-follow
working-file read. Editor adapters consume a verified local location. The first adapter
uses the documented VS Code `--reuse-window --goto file:line:character` CLI:
https://code.visualstudio.com/docs/configure/command-line . It uses an argument vector,
never a shell or a command supplied by an HTTP request. Tests use a recording adapter.
The executable is configured at native launch (default `code`). File verification is bounded; race-free
confirmation of editor buffer contents, including existing unsaved edits, is outside this CLI handoff's guarantee.

A dedicated investigation route is the primary page; source exploration moves to
`/explore`, and review stays `/review`. Browser helpers continue to consume bounded native
queries. The question, cards and reader connections dominate the workspace; source lookup
and captured preview are temporary panels. Render all source, names and notes as text.
Do not add a diagramming dependency or another parser for this bounded first slice.

Validation follows the interfaces: portable model tests for bounds, identities, links,
removal and roundtrips; native tests for capture/version mismatch, expiry, persistence
failure and working-file changes/replacement; browser tests for constructing a cross-file
investigation, preview, editing notes, linking and handoff refusal. Use safe Rust, TDD and
serial lower-priority mutations. Benchmark interaction separately. The real pinned lpc-rs
workflow must demonstrate a companion contribution, not just another source-browser replay.
