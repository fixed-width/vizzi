# Ruby analysis and navigation

Contributor contract for extending the existing source, syntax and language-server
interfaces. User setup and language limits belong in the user documentation.

Ruby source is selected by default: `.rb`, `.rake`, `.gemspec`, `.ru`, `Gemfile` and
`Rakefile`. Project metadata participates in invalidation without being executed.
Tree-sitter supplies syntax colouring, module/class/method/constant declarations, exact source
identifiers and written calls. No Ruby runtime or successful project build is needed
for this layer. Ruby stays alongside other languages in a mixed source root.

Recognise ordinary and singleton method definitions, including punctuation names and
endless methods. Preserve singleton/lexical scope in declaration identity. Written calls
include receiver calls, argument calls and bare method calls where lexical locals do
not make the identifier ambiguous. Parameters, assignments, block bindings and pattern
variables must not become fabricated callers. Unknown constructs retain omissions;
raw name matches are candidates, never proven dynamic dispatch.

Use the existing bounded syntax-state, source-window, reflow and structural interfaces.
Ruby reflow must keep strings, regexps and symbols intact and recognise Ruby comments
and operators. Namespace cues use captured declarations rather than guessing a module
from the file path. Reopened declared namespaces can share a cue across files.

An optional `--ruby-lsp EXECUTABLE` profile uses the installed Ruby LSP. The normal
launcher can compose/install a bundle and load project add-ons; the adapter must bypass
those actions. A private, generated server bootstrap disables add-ons and isolates Bundler
from project Ruby configuration. Project Gemfiles and gemspecs are source data, never
evaluation inputs. Only existing local dependencies can be exposed; no installation or
build is invoked while inspecting a project.

The shared stdio transport must support Ruby LSP's indexing progress and full-capture
close/open synchronization followed by ordered
document requests rather than requiring pushed versioned diagnostics. Definition and
reference lookups use captured source in a private mirror, synchronise saved edits,
restart on changed project context and admit external source through the bounded catalog.
Canonical URI deduplication prevents aliases from creating false ambiguity. Reference
results must match a unique definition, with declaration entries excluded. Ruby bodies
take precedence over accompanying RBS signatures; signature-only targets stay unavailable.
Server heuristics and Ruby metaprogramming can leave multiple/unavailable destinations.
ERB, RBS analysis and generated Rails/dynamic methods are outside this initial slice.

Validate through public interfaces: declaration identity, locals versus bare calls,
exact identifier offsets, qualified lookup, colouring/reflow, incremental equivalence,
mixed-language capture, standard protocol peers and real Ruby LSP. Check same-name
methods, references, changed/deleted files, local dependencies and non-execution of
project configuration/add-ons. Use one low-priority worker for mutation testing.
