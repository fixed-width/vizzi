# Semantic navigation across supported languages

For contributors extending the visual call workspace. Semantic language-server navigation
is a core capability. Written-name matches remain a fallback, not the intended substitute
for language analysis in Python or C/C++.

One host can configure rust-analyzer, Pyright, Ruby LSP, clangd, JDT LS and
the native TypeScript server together. Server selection uses
the captured document's language, including in mixed-language repositories. Existing
`--rust-analyzer` launches remain compatible. Python, Ruby and C-family providers are explicitly
configured with `--pyright`, `--ruby-lsp` and `--clangd`; C-family capture scopes remain explicit.
Standalone JavaScript and TypeScript use `--typescript-server` with an installed
TypeScript 7 or newer `tsc` executable. Their project settings come from the capture.

Definition navigation, callers and references use the same semantic provider seam. Typed
receivers, imported aliases and cross-file definitions resolve before the name chooser.
Unique admitted targets open directly; ambiguous results retain a choice. Unavailable
servers, project context or source retain clear fallback evidence. Semantic definitions
are static analysis, not observed runtime dispatch.

Vizzi-managed providers analyse a private mirror of captured source. Initialization, configuration
sections, language IDs, arguments and readiness differ by server; JSON-RPC framing,
position conversion, result limits and read-only requests are shared. A Python environment
can be selected explicitly. Clangd uses captured compilation commands rather than running
a build to discover flags. Project-input changes restart sessions. Profiles
can also restart the server after a source change to refresh its project index.
Results remain bound to captured bytes and checked configuration.

Local dependency source remains bounded and language-specific. Only source reached by the
provider can enter the existing dependency catalog. Known external/unavailable targets
must never turn into unrelated local name matches. Editor handoff continues to verify
current on-disk bytes; language servers do not grant edit/refactor/debug capabilities.

Validate through deterministic protocol peers and real installed servers: conflicting
method names, aliases, references used as values, UTF-16 positions, mixed languages,
configuration changes, incremental source sync, missing servers, external source and late
replies. C/C++ call extraction must preserve exact written identifiers and source ownership.
Keep worker limits, mutation testing and performance checks consistent with AGENTS.md.

The optional [editor-sharing mode](../configuration/PRODUCT.md) reuses VS Code providers
without starting or owning another language server. It verifies saved/captured source,
rejects dirty buffers and admits repository-local results. Its live editor context and
dependency limitations remain distinct from the managed captured-source providers.

## Java with a managed server

Selecting JDT LS in the native launch settings enables project-aware definitions
and references between captured Java files. The workspace root is the default
Java source root. Users can list workspace-relative source roots for layouts
such as `src/main/java` and `src/test/java`.

The server reads a private copy of captured Java source under those roots and
keeps its state outside that copy. Java files elsewhere remain available for
browsing and written-name lookup. Startup and requests remain bounded. Lookups
after a saved edit inside a source root use the new capture. Maven, Gradle, and
Eclipse project files stay outside this source-root configuration. Repository
browsing requires no Java tooling.
JDK and local library destinations can open an independent source capture when
JDT LS identifies a matching attached source archive.

For a Maven or Gradle workspace, **Import Java project** loads the build
classpath through JDT LS from the captured project files. The action uses
already installed tooling and local dependencies. Ordinary browsing and
definition requests do not start an import. After import, saved Java changes
update the existing session. A build configuration change asks the reader
to import again before using that classpath. Java projects imported this way
need no source-root or per-JAR flags.

Gradle projects with a Wrapper use the distribution identified by the captured
Wrapper properties and the completed local cache. This lookup does not run
the Wrapper or provision a distribution. A project without a Wrapper uses a
local Gradle installation. A configured Gradle JVM may differ from the JDT LS JVM.
