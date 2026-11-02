# Language adapter and worker contracts

This reference describes source selection, analyzer ownership, and validity rules for
the native prototype. The core interfaces live in `vizzi_core::analysis`.

## Registered languages

`LanguageAdapter` supplies an immutable analysis context and
a factory for worker-owned `Analyzer` values. `Analyzer::analyze` consumes captured
bytes and returns path-independent analysis. Adapters must not execute project tooling.
They report unsupported encoding and syntax recovery through coverage and diagnostics.

`Languages::new` freezes each registered context. Empty/duplicate language names and
repeated context identities fail registration. An empty registry is valid. Registration
does not select source extensions; the [portable source policy](SOURCE_POLICY.md) does.
There is one configured raw context per assigned file in this host; the immutable
document model also permits multiple analyses.

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

The default host omits other extensions. Explicit C-family scopes capture C/C++
candidates, ambiguous headers, fragments, and CUDA source without inventing analysis.
C/C++ analyzers are the next delivery; Python stubs, notebooks, and LPC remain outside
scope. Counts describe selected capture scope, not every repository file.

The [C/C++ policy](C_FAMILY_POLICY.md) distinguishes implemented source selection from
future raw adapters and configured analysis. Its [delivery plan](../plans/c-family.md)
records the remaining stages.

## Native source policy

`Directory::open` uses default source policy and the built-in registry.
`with_languages` accepts a shared `Arc<Languages>` with default capture rules;
`with_policy` also accepts explicit policy configuration. The CLI accepts source
scopes, exclusions, and raw C/C++ assignments. Browser configuration is not implemented.

Full discovery, targeted capture, event classification, and job dispatch use that same
directory policy. Root exclusions and symlink rules remain as specified by
[incremental capture](INCREMENTAL_CAPTURE.md). A baseline document whose extension is
no longer supported or whose context differs forces full reconciliation before unread
documents can be retained. Explicit extension changes through rename remove the old
document and select the new adapter.

A host's policy/registry is immutable for its lifetime, and startup always reconciles.
Snapshots retain capture-policy identity; a mismatch enforces complete discovery even
with an empty baseline. Native callers no longer need to manually request a full pass
after changing policy. Dynamic configuration reload and persisted snapshots remain
outside this host's current contract.

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
inputs change. Selection changes belong to capture policy; analyzer configuration and
implementation changes belong to context. Timestamps and suffixes alone do not establish
analysis validity.

Both cached and newly computed analysis must match the complete registered context.
`Document::capture` validates content identity, ranges, and fact keys before publication;
new output is validated before persistence. Invalid cache records are misses. Factory,
analysis, and validation errors fail the attempt; the host retains its previous
snapshot. Cache write errors remain nonfatal warnings. Valid content-addressed cache
entries produced during cancelled attempts can be reused later.

View and pinned-source responses use schema 2 for classification, policy identity,
encoding, and capture/analysis counts. Status/notifications remain schema 1. The browser
shows source-only reasons separately from encoding and grammar recovery; raw outlines
state that build-configuration coverage is unknown. See [API.md](API.md).
