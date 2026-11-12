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

Vizzi uses its own source policy. At the inspected root, `.git`, `target` and
`node_modules` are excluded. Other paths ignored by Git are not automatically excluded;
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

The Python/Ruby/C-family semantic mirror currently admits up to 8 MiB per source file and
128 MiB of source per provider workspace. Exceeding these bounds makes semantic lookup
unavailable; ordinary captured-source browsing has separate limits. Local dependency
captures retain at most 2 MiB per file, 32 MiB total and 32 files.
