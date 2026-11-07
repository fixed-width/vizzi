# Source-search candidate coverage experiment

For contributors improving question-driven discovery before another model trial. This
contract precedes new real-source candidate results. Keep the existing compact selector
available and all historical archives unchanged. No inference or viewer integration runs.

## Behaviour and architecture

Capture Rust macro definitions as raw declarations with their names/ranges and enclosing
scopes. Do not expand macro bodies, invent generated declarations or resolve calls.
The same raw adapter serves native/browser hosts; validate both targets.

Add a separate portable search selector with the same question, result-count and literal
byte bounds as compact retrieval. Split recognised English creation, extension and
flow/check questions into topic terms and two generic operation facets. Use captured
identifier/body/path postings and raw declaration scopes to retrieve complementary
starting points. Creation covers constructors and entry/caller locations; extension covers
registration mechanisms and examples; flow covers submission/dispatch and validation.
No repository-specific names, paths, question IDs, gold anchors or model output enter
the selector. These are lexical hypotheses, not resolved relationships.
Preview allocation follows compact selection: budget divided by requested count, clamped
to 128–1,000 bytes and to the total budget. Complete shorter scopes remain intact.

Require a topic match before operation words can admit a result. Balance the two facet
pools, suppress repeated scopes/overlapping excerpts and keep conventional test paths
behind implementation sources. The existing role metadata remains a hint.
Excerpts overlap excessively when they share more than half the shorter range; an exact
half overlap remains eligible, as in compact selection.
Unrecognised question types use compact retrieval unchanged. Flow requires explicit trace/codepath
wording; a file extension or file-path question alone does not activate these facets.
A recognised topic with no matches is an empty context, not proof of repository absence.
Preserve literal UTF-8 ranges, stable
labels and snapshot/file binding. Refresh must invalidate held contexts as usual.

Use a small interface, `Corpus::retrieve_search(question, limit, budget)`, exposed as
`selector: search` by the experimental native host. Keep the original selector defaults.
Version the new selection implementation in index identity. No new dependency, unsafe
code, source execution, source-control assumption or model requirement is introduced.

## Validation and frozen comparison

Use TDD with unrelated names and file layouts for creation, extension and flow, including
Python, explicit named-symbol queries, unknown topics, literal bounds and source refresh.
First establish macro declaration/range/parent behaviour without generated facts.
Use focused whole-module mutations, serial at nice 10 with one compiler/jobserver task.
Report equivalent/unviable mutants, timeouts and incomplete work separately.

The known development comparison reuses the three exact maintainer questions and pinned
lpc-rs archive from the ranking study. Before running real queries, freeze seven locator
targets: Transaction::new and Task::open_attempt; the efuns macro definition and invocation;
Committer::dispatch, its commit validator and the FutureVersion rejection. These are
lookup targets, not a complete-answer rubric. Keep all eight reserved cases unrun.

Also compare the twelve existing answerable lpc-rs v2 development questions as regression
controls. Use their original source anchors without changing them to fit outcomes. This
is known-question evidence; none becomes fresh held-out acceptance. Keep source-aware
authorship and anchor-coverage limitations explicit.

Compare compact and search at eight results/4,000 literal bytes on the same new index.
Macro support changes analysis/index identity and may affect compact candidates too;
record the historical ranking-study baseline separately. Do not call this an isolated
ranking-only comparison. Freeze code/settings before the real-source cohort and retain
misses without another policy change in that cohort.

Measure literal target markers and captured-scope locations separately, with first target
rank and per-question source gaps. A declaration can be located by its frozen path/scope
even when its signature is outside the excerpt; table invocations and rejection branches
require visible literal markers. Freeze those alternatives before queries. Neither
location nor anchor presence establishes useful navigation or complete semantic evidence.
Retain all candidates so the maintainer can inspect false positives and compare orders.

After correctness/build/mutation jobs finish, measure optimized query time and process-tree
memory without a model. Use three alternating-order repetitions per question, one native
index, serial queries and nice 10. Record cold startup separately. Report a provisional
one-second median warm-query target and the existing 4-GiB observed PSS target as budgets,
not caps or hardware qualification. Stop the owned native process.

The experiment is useful if the previously missing caller, registration mechanism and
validator become inspectable starting points without material regression on the broader
controls. Partial improvement and remaining gaps are valid results. Do not add embeddings,
query-generating models or another reranking ablation in this slice.

Separating query vocabulary from source vocabulary is motivated by
[CodeHow](https://www.microsoft.com/en-us/research/wp-content/uploads/2015/11/ASE-2015-CodeHow-Effective-Code-Search-based-on-API-Understanding-and-Extended-Boolean-Model.pdf).
Our bounded facets and selection policy are local hypotheses, not a reproduction of that
system or evidence that its reported retrieval results transfer to this experiment.
