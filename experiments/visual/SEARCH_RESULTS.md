# Qualified fragment search and ranking

For maintainers checking the fragment-chooser follow-up. The reported failures were a
qualified query (`LufsMeter::process_sample`) returning nothing and source-order results
placing partial names before exact names.

Search now uses cached case-folded declaration names, qualified lexical names and paths.
Both `::` and `.` work as namespace separators. Results rank by exact name/full qualified
name, exact qualified suffix, function-name prefix, function-name substring, namespace
prefix, namespace substring, then path. Ties use name, qualified name, path and source order.
The full matching set is ranked before paging. The function chooser filters declaration
kinds before matching/counting/paging, while the shared inventory still supports other
kinds. Empty general inventories retain lexical order; empty function lists are alphabetical.

The pure lexical matcher is separate from captured-source facts, source IO and LSP definition
resolution. No parser/analyzer identity or dependency was changed. This supports lookup by
captured/displayed scopes, not semantic import-alias or complete crate-path interpretation.

Focused tests first reproduced all three matching/ranking failures and unstable same-line
ties. They cover qualified suffix/prefix priority, Unicode/case folding, Python dot notation,
trait implementation labels, exact matches beyond an early page, function-only totals,
source-area limits, path fallback and deterministic ties. A browser regression reproduces
qualified lookup, exact-first ordering across pages, correct function totals and opening the
matching source fragment. Normal baseline checks include workspace tests/doctests,
formatting, Clippy and the complete browser suite.

Mutation checks use one low-priority worker/compiler task and serial tests. The initial
cohort covers the full new matcher plus changed inventory/filtering glue; a final complete
matcher run covers the refined preference for a function name over an enclosing type name.
Compile failures, timeouts and surviving variants are separate from assertion failures.
Mutation results and the known-repository replay are evidence, not independent usefulness
or whole-repository performance qualification.

Final checks passed: formatting, Clippy with warnings denied, 570 workspace tests including
doctests, and the complete browser suite. The optional real-language-server tests remain
outside the normal suite; this change does not alter definition-provider behaviour.
[Native log](results/search/native-tests.log.gz), [browser log](results/search/browser-tests.log.gz),
[Clippy log](results/search/clippy.log.gz).

The initial mutation cohort exercised 34 variants: 17 caught by failing tests and 17
unviable compile failures. The final complete matcher run exercised ten variants: nine
caught and one unviable. No survivors, timeouts or exclusions. Compile failures are not
assertion coverage. The initial run includes changed inventory/filtering glue; the final
run covers the whole matcher after the function-name-versus-scope relevance refinement.
[Initial record](results/search/initial-mutations.json.gz),
[final matcher record](results/search/final-matcher-mutations.json.gz).

The [known-source replay](results/search/result.json) on unchanged Tract Plugin Pack
`951cf85413029423e036c641638b845edafe75cf` finds 176 function matches for `process`.
All 57 exact `process` names precede partial and scope-only matches across every page.
`GsMeter::process`, `LufsMeter::process_sample` and `LufsMeter.process_sample` each return
one intended function; selecting the latter opens the captured LUFS fragment.
[Process results](results/search/process.png), [qualified results](results/search/qualified.png).

Single-session browser-request observations were 372 ms for the initial `process` search,
including first projection construction and UI scheduling, and 47–49 ms for the qualified
queries. These are not latency distributions, isolated native query costs or large-repository
qualification. The replay runs against an optimised host after correctness/mutation checks
have stopped. It does not start a language server or build the analysed project.

Reproduce with `node experiments/visual/replay-search.mjs /path/to/tract-plugin-pack /tmp/new-search-replay`
after building the release host. Source facts, opaque identities and analysis fingerprints
are unaffected, so existing visual workspaces can restore after the viewer update.
