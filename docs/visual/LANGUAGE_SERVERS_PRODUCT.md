# Semantic navigation across supported languages

For contributors extending the visual call workspace. Semantic language-server navigation
is a core capability. Written-name matches remain a fallback, not the intended substitute
for language analysis in Python or C/C++.

One host can configure rust-analyzer, Pyright, Ruby LSP and clangd together. Server selection uses
the captured document's language, including in mixed-language repositories. Existing
`--rust-analyzer` launches remain compatible. Python, Ruby and C-family providers are explicitly
configured with `--pyright`, `--ruby-lsp` and `--clangd`; C-family capture scopes remain explicit.

Definition navigation, callers and references use the same semantic provider seam. Typed
receivers, imported aliases and cross-file definitions resolve before the name chooser.
Unique admitted targets open directly; ambiguous results retain a choice. Unavailable
servers, project context or source retain clear fallback evidence. Semantic definitions
are static analysis, not observed runtime dispatch.

Providers analyse a private mirror of captured source. Initialization, configuration
sections, language IDs, arguments and readiness differ by server; JSON-RPC framing,
position conversion, result limits and read-only requests are shared. A Python environment
can be selected explicitly. Clangd uses captured compilation commands rather than running
a build to discover flags. Source changes reuse warm sessions; project-input changes
restart them. Results remain bound to captured bytes and checked configuration.

Local dependency source remains bounded and language-specific. Only source reached by the
provider can enter the existing dependency catalog. Known external/unavailable targets
must never turn into unrelated local name matches. Editor handoff continues to verify
current on-disk bytes; language servers do not grant edit/refactor/debug capabilities.

Validate through deterministic protocol peers and real installed servers: conflicting
method names, aliases, references used as values, UTF-16 positions, mixed languages,
configuration changes, incremental source sync, missing servers, external source and late
replies. C/C++ call extraction must preserve exact written identifiers and source ownership.
Keep worker limits, mutation testing and performance checks consistent with AGENTS.md.
