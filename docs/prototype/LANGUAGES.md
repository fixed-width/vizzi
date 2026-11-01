# Language adapter and worker contracts

This reference describes source selection, analyzer ownership, and validity rules for
the native prototype. The core interfaces live in `vizzi_core::analysis`.

## Registered languages

`LanguageAdapter` supplies exact source extensions, an immutable analysis context, and
a factory for worker-owned `Analyzer` values. `Analyzer::analyze` consumes captured
bytes and returns path-independent analysis. Adapters must not execute project tooling.
They report unsupported encoding and syntax recovery through coverage and diagnostics.

`Languages::new` freezes each registered context. Empty/invalid extension claims,
conflicting extensions, and repeated context identities fail registration. An empty
registry is valid. Extensions are case-sensitive ASCII letters/digits without dots;
one adapter may claim several extensions. There is one configured context per selected
file in this host; the immutable document model also permits multiple analyses.

`Languages::builtin` registers:

| Language | Extension | Extracted declarations | Limits |
| --- | --- | --- | --- |
| Rust | `.rs` | Functions, types, modules, constants, statics, implementations | Raw source; no macro expansion or compiler resolution |
| Python | `.py` | Classes and functions, including methods, nested and async functions | No import/type resolution, assignments, or call relationships; declaration spans omit decorators |

Only UTF-8 source supports declarations. Python encoding cookies do not enable other
decoders. Grammar recovery sets `partial`; undecodable source sets `unavailable` and
remains a captured file. `syntax` describes grammar acceptance, not runtime validity.
Shared traversal generates lexical qualification, byte spans, and duplicate ambiguity.
Unique declaration identities survive position changes in either language.

The default host omits other extensions, including C/C++ sources, ambiguous headers,
CUDA, Python stubs, notebooks, and LPC. Counts describe this selected scope, not all
repository files. C/C++ integration still needs a policy for dialects, compilation
contexts, preprocessor configurations, and headers. There is no extension guessing.

## Native source policy

`Directory::open` uses the built-in registry. `Directory::with_languages` accepts a
shared `Arc<Languages>` for explicit configuration in native callers. There is no CLI
or browser language-configuration interface yet.

Full discovery, targeted capture, event classification, and job dispatch use that same
directory policy. Root exclusions and symlink rules remain as specified by
[incremental capture](INCREMENTAL_CAPTURE.md). A baseline document whose extension is
no longer supported or whose context differs forces full reconciliation before unread
documents can be retained. Explicit extension changes through rename remove the old
document and select the new adapter.

A host's registry is immutable for its lifetime, and startup always reconciles. Native
callers replacing a registry must start with `Engine::refresh` for complete discovery
before supplying path hints. Document-level context checks cannot detect files newly
admitted by a broader policy but absent from the old snapshot. Dynamic configuration
and a persisted capture-policy identity are outside this host's current contract.

## Shared worker budget

`Engine::new` accepts 1–32 workers, defaulting to four in the CLI. Every refresh uses
one largest-file-first queue across all selected languages and at most that many
workers. A worker creates each needed analyzer lazily and owns it exclusively; there
are no independent per-language thread pools. Warm cache hits create no analyzers.
With two adapters, up to twice the worker count of parser instances may exist during
an attempt, with no more than the worker count actively analyzing files.

The host serializes refresh attempts; direct concurrent `Engine::refresh` callers
each receive their own per-attempt limit. This is a concurrency budget, not a hard RAM
limit. Capture still retains all selected bytes, and a single parser may need a large
tree. Per-file size limits, admission by memory estimate, and process-wide scheduling
remain open. Cancellation is checked between jobs; an individual parse is not interrupted.

## Analysis validity and failure

Cache keys combine captured content and context identities. Context fingerprints
include the adapter implementation, shared traversal/registration/model sources, and
the dependency lockfile. This conservatively invalidates old contexts when those
inputs change. File selection/configuration changes must be represented by the registry
and context; timestamps and file extensions alone do not establish analysis validity.

Both cached and newly computed analysis must match the complete registered context.
`Document::capture` validates content identity, ranges, and fact keys before publication;
new output is validated before persistence. Invalid cache records are misses. Factory,
analysis, and validation errors fail the attempt; the host retains its previous
snapshot. Cache write errors remain nonfatal warnings. Valid content-addressed cache
entries produced during cancelled attempts can be reused later.

Source and host response schemas are unchanged. Outlines already carry language,
context identity, diagnostics, and coverage. The browser displays the selected
outline's language alongside its coverage and labels overview counts as source files.
