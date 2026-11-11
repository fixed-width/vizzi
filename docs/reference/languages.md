# Language support and analysis limits

For users deciding what Vizzi can show for a project. Source browsing and call resolution
are separate capabilities; recognising a declaration does not imply resolving its calls.

| Capability | Python | Rust | C/C++ |
| --- | --- | --- | --- |
| Default source selection | `.py` | `.rs` | Explicit C-family scopes required |
| Syntax-coloured source and declaration outlines | Supported within parser/view bounds | Supported within parser/view bounds | Raw syntax for assigned C/C++; ambiguous headers can be source-only |
| Written-call navigation | Name-based candidates | Candidates plus a narrow source-local `self` method rule | Not provided by the current call adapter |
| Project-aware definitions and semantic references | Not implemented | Optional rust-analyzer provider | Not implemented |
| Caller/reference search without a semantic provider | Written-call name candidates | Written-call name candidates | No equivalent call coverage |
| Source outside the configured repository | Not resolved as Python dependencies | Supported local dependency captures through rust-analyzer | Not implemented |
| Language module/package cues | Recognised Python modules/packages | Captured Cargo target/module structure | Not available |

## Python

Python does not need a runtime, virtual environment or language server for syntax browsing.
Vizzi does not run the program. Navigation matches captured written call names; it does
not perform Python import resolution, infer receiver types or establish dynamic dispatch.
Aliases, decorators, inheritance, generated code and callbacks can limit or mislead a
name-only match. The UI preserves candidate status when opening one.

Without semantic reference support, **Find references** falls back to written-call
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

## C/C++

Explicit scopes enable source capture and raw declaration/type outlines. Unassigned `.h`
files can remain readable without an outline because their language is ambiguous. CUDA
is an unavailable dialect in this implementation. No compilation database or compiler
configuration is consumed; configuration coverage remains unknown.

Other languages can share `.c`/`.h` suffixes. Exclude their paths instead of treating them
as C-family source. See [source-scope examples](../how-to/browse-source.md#show-cc-source).

## Shared limits

Vizzi uses its own source policy. At the inspected root, `.git`, `target` and
`node_modules` are excluded. Other paths ignored by Git are not automatically excluded;
use `--exclude-source` for virtual environments, generated source or vendor trees you
do not want in the reading scope.

Workspace capture does not follow source-file symlinks. Unsupported encodings, parser
recovery, deferred analysis, projection limits and missing dependency source are distinct
availability states. Source may be readable even when analysis is unavailable.

Only configured, captured source participates in a query. Results can omit generated,
unindexed, external or unsupported code. No callers/references, or an empty outline,
does not establish that code is unused or absent from a running program.

Large-repository interaction and cross-browser support are still being qualified. The
current user documentation is checked on Linux with Chromium, not a performance or
platform guarantee for every repository.
