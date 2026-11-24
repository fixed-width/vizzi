# Language support and analysis limits

For users deciding what Vizzi can show for a project. Source browsing and call resolution
are separate capabilities; recognising a declaration does not imply resolving its calls.

| Capability | Python | Rust | Ruby | C/C++ |
| --- | --- | --- | --- | --- |
| Default source selection | `.py`, `.pyi` | `.rs` | `.rb`, `.rake`, `.gemspec`, `.ru`, Gemfile, Rakefile | Explicit C-family scopes required |
| Syntax-coloured source and declaration outlines | Supported within parser/view bounds | Supported within parser/view bounds | Modules, classes, explicit methods and constants | Raw syntax for assigned C/C++; ambiguous headers can be source-only |
| Written-call navigation | Semantic with Pyright; otherwise name candidates | Semantic with rust-analyzer; source-local bindings and candidate fallback | Semantic with Ruby LSP; otherwise name candidates | Semantic with clangd and compilation context; candidate fallback |
| Project-aware definitions and semantic references | Pyright provider | rust-analyzer provider | Ruby LSP; reference destinations cross-checked | clangd provider |
| Caller/reference search without a semantic provider | Written-call name candidates | Written-call name candidates | Written-call name candidates | Written-call name candidates |
| Source outside the configured repository | Locally available Python/stub captures through Pyright | Local captures through rust-analyzer | Local Ruby source reached through Ruby LSP | Local C/C++ captures through clangd |
| Language module/package cues | Recognised Python modules/packages | Captured Cargo target/module structure | Declared Ruby modules, including reopened namespaces | Not available |

## JavaScript and TypeScript

Default source capture includes `.js`, `.jsx`, `.mjs`, `.cjs`, `.ts`, `.mts`, `.cts` and
`.tsx`. Vizzi uses separate TypeScript and TSX parsers and supports JSX in JavaScript.
Browsing works without Node.js, a package install or a project build.

Outlines include named functions, named classes, variable-bound objects, and their
identifier-named or unescaped quoted methods. TypeScript also shows interfaces, enums,
type aliases and named signatures. Computed methods and anonymous argument callbacks
appear in the outline. Named function bodies and methods are available in **Add bubble**.

Direct calls, dotted member calls and constructor expressions provide written-call
candidates. A managed TypeScript language server or VS Code provider sharing can
resolve project-aware definitions and references, including aliases and inferred
types. Results are mapped to captured source. Without a provider, **Find references**
uses written-name candidates. [Configure JavaScript and TypeScript navigation](../how-to/configure-language-servers.md#javascript-and-typescript-with-a-managed-server)

Package labels and accents use the closest captured `package.json` within the
workspace. A missing or invalid name leaves the bubble neutral. Historical labels
use the manifest from the selected Git commit.

Managed TypeScript and VS Code sharing can open installed JavaScript or TypeScript
package declarations in independent dependency bubbles. The selected provider
determines whether a definition points to an implementation or a declaration file.

## Java

Vizzi captures `.java` files by default. Syntax coloring, outlines, and written-call
navigation work without a JDK or project build. Outlines include classes, interfaces,
enums, records, annotation types, methods, and constructors. Use **Add bubble** to
open methods and constructors with bodies.

Bubbles show their declared Java package and share a cue with other bubbles in
the same package directory. A captured `module-info.java` at the corresponding
source root qualifies the label as `module/package`. The cues also work in
function history and comparisons. [Module cues](../visual/MODULES_PRODUCT.md)

Written method calls and `new` expressions provide name candidates, including
separate candidates for overloaded declarations. An implicit record constructor
can lead to its record declaration. A [managed JDT LS server](../how-to/configure-language-servers.md#java-with-a-managed-server)
can resolve overloads and references across configured source roots. The
Workspace menu can import a Maven or Gradle classpath from captured build
descriptors and local caches. A
[shared VS Code Java provider](../how-to/configure-language-servers.md#java-with-vs-code)
uses the editor's project configuration. Vizzi checks results against captured
workspace files. Managed JDT LS can open local JDK and dependency source archives
in independent bubbles. [Configure Java library source](../how-to/configure-language-servers.md#open-java-library-source)

## Python

Python does not need a runtime, virtual environment or language server for syntax browsing.
Vizzi does not run the program. The configured Pyright provider resolves imports, aliases
and inferred receiver types. Without it, navigation matches captured written call names.
Neither mode establishes dynamic runtime dispatch.
Aliases, decorators, inheritance, generated code and callbacks can limit or mislead a
name-only match. The UI preserves candidate status when opening one.

When no semantic provider is configured, **Find references** falls back to written-call
candidates; it does not enumerate all assignments, imports or callback uses. See
[call destinations and references](../explanation/call-destinations.md).

## Rust

Raw syntax works without a project build. Some unambiguous same-file `self.method()`
calls resolve directly from syntax. Other name matches remain candidates unless the
[configured rust-analyzer provider](../how-to/configure-rust.md) supplies stronger evidence.

The provider uses offline Cargo metadata and captured source. Build scripts, procedural
macros and check-on-save builds are disabled. Dependency and standard-library source must
already exist on disk. Macro-generated declarations, conditional configurations and
unsupported project/syntax forms can remain unavailable. Static resolution is not a
runtime call trace.

With the VS Code extension, editor-resolved Rust dependency and standard-library source
can also open as captured bubbles. A provider-reported function range can admit methods
using newer syntax such as `[const]` bounds when native parsing misses them. The original
text is retained, while raw call analysis inside that function can remain unavailable.
VS Code also supplies workspace-installed JavaScript and TypeScript package source.

## Ruby

Basic browsing does not need Ruby or a project build. The syntax adapter preserves class,
module, ordinary/singleton method and constant declarations. Bare method names are
distinguished from lexical locals, including parameters, block bindings and assignments.
Receiver calls, arguments, punctuation names and endless methods use exact source offsets.
The local-binding allowance is 4,096 names per file; when exceeded, bare-call evidence is
omitted while explicit calls remain available within the shared call limits.

The optional [Ruby LSP provider](../how-to/configure-language-servers.md#ruby-with-ruby-lsp)
uses captured source, existing RubyGems and private startup configuration. Method reference
results retain only unique verified definition matches. Dynamic dispatch, metaprogramming,
generated methods, custom load paths and unavailable libraries can limit coverage. Operators
expressed implicitly, such as `a + b`, are not enumerated as method calls. ERB/RBS parsing
and reproducing the complete Bundler environment are outside this initial support.

Module accents describe captured declared namespaces, not runtime aliases or file-based
packages. A class without an enclosing module remains neutral. Ruby source can be explored
alongside the other supported languages in the same repository.

## C/C++

Explicit scopes enable source capture and raw declaration/type outlines. Unassigned `.h`
files can remain readable without an outline because their language is ambiguous. CUDA
is an unavailable dialect in this implementation. Raw syntax does not consume build
configuration. The optional clangd provider consumes a frozen compilation database or
flags file for semantic lookup; it does not run a build to obtain one. Ordinary function
prototypes and method declarations are admitted for navigation. Implicit constructor/operator
calls and some generated/unsupported syntax remain outside written-call capture.

Other languages can share `.c`/`.h` suffixes. Exclude their paths instead of treating them
as C-family source. See [source-scope examples](../how-to/browse-source.md#show-cc-source).

[Configure language servers](../how-to/configure-language-servers.md) for setup and
project-context requirements. Returned declarations can be stubs or headers rather than
implementation bodies.

## Shared limits

Vizzi uses its own source policy. Capture excludes root `.git` and `target`, and
`node_modules` at any depth. Other paths ignored by Git are not automatically excluded;
use `--exclude-source` for virtual environments, generated source or vendor trees you
do not want in the reading scope. Cargo lockfiles, Python project settings and enabled
C-family compilation metadata participate in change detection. Ruby lockfiles and runtime
selection metadata are captured as project context without execution.

Workspace capture does not follow source-file symlinks. Unsupported encodings, parser
recovery, deferred analysis, projection limits and missing dependency source are distinct
availability states. Source may be readable even when analysis is unavailable.

Only configured, captured source participates in a query. Results can omit generated,
unindexed, external or unsupported code. No callers/references, or an empty outline,
does not establish that code is unused or absent from a running program.

Large-repository interaction and cross-browser support are still being qualified. The
current user documentation is checked on Linux with Chromium, not a performance or
platform guarantee for every repository.

The shared Python, Ruby, C-family, Java, and TypeScript semantic mirror admits up
to 8 MiB per source file and 128 MiB of source per provider workspace. Semantic
lookup is unavailable above those bounds. Ordinary captured-source browsing has
separate limits. Local dependency captures retain at most 2 MiB per file, 32 MiB
total and 32 retained captures.
