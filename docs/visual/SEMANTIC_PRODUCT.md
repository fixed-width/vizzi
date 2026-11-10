# Type-aware call navigation

For readers following real call chains in the visual workspace. `self` field chains,
qualified/aliased functions and ordinary variables require project-aware language analysis.
The existing source-local rule is a fast path, not a substitute for that analysis.

- With a language provider configured, selecting an unresolved Rust call asks it for the
  definition at the captured callee's actual identifier position. A unique captured definition
  opens directly beside the caller; unrelated name matches do not appear first.
- Multiple semantic destinations use the chooser. Missing/failed/unsupported analysis keeps
  the explicit candidate fallback and explains its lower precision. A known destination
  outside repository source can enter the [local dependency capture catalog](DEPENDENCIES_PRODUCT.md).
  Unavailable source/declarations are reported rather than replaced with unrelated name matches.
- Definition links identify language-server evidence separately from source-local and
  name-candidate evidence. Definition navigation does not prove runtime dispatch.
- The previous fragment, reading state and layout remain intact during and after lookup.
  Late replies cannot act after a view closes, a new selection supersedes it or source changes.
- The Rust provider uses rust-analyzer through standard LSP behind an editor-neutral native
  definition interface. It is explicitly enabled at host startup. Basic browsing works
  without it. No automatic editor launch, repository builds or procedural-macro execution.
- Lookups use a private source mirror from captured documents and frozen project inputs.
  Changes/configuration are checked before results are returned; target locations must map
  to admitted, version-bound native symbols. A working-tree pathname alone is insufficient.
- Opening a function may prepare its analysis in the background using one recognizable
  captured call. Preparation never opens a destination or supplies a cached navigation
  result. A foreground selection waits and requests a fresh definition.
- One low-priority server/worker handles lookups. Sessions stay warm and synchronize changed
  captures; ordinary source indexing does not wait for semantic navigation.

The first real acceptance case is `GsMeter::process`, line 317, opening
`LufsMeter::process_sample` in `gs-meter/src/lufs.rs` without a name-match chooser. Validate
additional independent field/alias/qualified calls, ambiguous/unavailable providers, external
locations, UTF-16 positions, stale captures, timeouts, incremental reuse and recorded editor
adapters. Known-source replay is functional evidence, not an unseen comprehension trial.
