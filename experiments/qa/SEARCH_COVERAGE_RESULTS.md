# Candidate coverage: raw macros and complementary source pools

For contributors deciding how to improve question-driven search. **The new selector is
faster on the three focused questions and finds four of seven frozen locators, versus
one for compact search, but regresses transaction creation. Keep it experimental.**
The existing selector defaults remain unchanged; no model or natural-language viewer
feature is added. [Inspect both sets of source cards](results/search-coverage/trial.md).

## Delivered behaviour

The raw Rust adapter now captures named macro definitions and their lexical scopes.
It does not expand invocations or report template-generated functions as declarations.
Native tests and the headless browser verify the definition's name/range/parent, visible
macro entry and absence of generated declarations. This also improves ordinary symbol
browsing independently of the search experiment.

`Corpus::retrieve_search` and native `selector: search` provide a separate candidate path.
It separates topic terms from recognised creation, extension and trace/codepath intent,
uses existing postings/scopes and balances two generic operation pools. Scope, overlap,
literal byte and result-count bounds remain explicit. Other question forms delegate to
compact retrieval. The implementation has no repository-specific names or paths.

The source engine owns result locations, source bytes and identities. All ninety pools
pass native source binding. The search model remains optional and unqualified, as in the
[earlier ranking trial](SEARCH_RESULTS.md). There is no inference in this comparison.

## Frozen questions and target coverage

The [contract](../../docs/qa/SEARCH_COVERAGE.md) and
[cohort](library/search-coverage/cohort.json) precede real-source queries. They use three
known maintainer questions, seven preferred locators and twelve existing answerable lpc-rs
development questions with 22 original source-anchor targets. Source-aware authorship and
known-question status are retained. All eight reserved real-source cases remain unrun.

The same pinned source archive is freshly extracted and verified. Both selectors share
the new macro-aware index: 931 selected files, 10,754,851 bytes and 22,821 windows.
The older ranking study had 22,808 windows. Source bytes are unchanged; analysis/selection
version changes index identity. This compares a macro-aware compact control with the new
selector, not an isolated ranking change. The historical ranking-study pools are retained
and still contain only one of the seven preferred locators.

| Question | Compact locators | Search locators | Result |
| --- | ---: | ---: | --- |
| Transaction creation | 1/2 | 0/2 | Loses Transaction::new; Task::open_attempt still missing |
| Add an efun | 0/2 | 1/2 | efuns macro definition first; invocation/table still missing |
| Commit path/checks | 0/3 | 3/3 | Receiver first; validator and FutureVersion branch seventh |
| Total | 1/7 | 4/7 | Partial improvement with a material regression |

All four search hits have visible literal markers; the predeclared scope alternatives
add no extra hits in this run. Scope location and literal presence remain separate in
the assessment. Finding the validator and one rejection does not establish that all
checks or the full commit path fit in the returned previews.

Transaction creation is a useful counterexample to the aggregate gain. The results do
contain other real `Transaction::new` sites: socket publication, command-task attempts
and callbacks. Those may help a developer explore creation paths, but they do not replace
the frozen constructor and ordinary Task-entry targets. The first result is a TxnHandle
wrapper, and the pool also contains Process fields, test-support code and a socket helper.
The 0/2 score is not a claim that every result is useless; losing the known constructor
is nevertheless a regression.

Code inspection suggests a specific weakness: topic admission uses body/path postings,
before parent-scope topic evidence affects priority. `Transaction::new` uses Snapshot/Self
inside its own declaration window, while wrappers explicitly mention Transaction.
Verb-rich attempt implementations also compete ahead of the expected Task entry. This
is a diagnosis from source/result inspection, not a separately isolated causal experiment.
No policy or target changes follow the observed results.

The efun result locates the macro implementation rather than the invocation where a new
row is added. Some other results concern runtime contexts or compiler prototypes rather
than extension steps. These are lexical starting-point suggestions, not a generated
procedure or proof of semantic relevance.

## Regression controls and cost

All twelve broader controls return **byte-identical contexts between selectors** under
the same index. Both find 11/22 original source-anchor targets. Preserving those pools
does not establish reliable answers or complete evidence. No control loses a target.

Three alternating-order repetitions run for each question/selector, ninety observations
total. All thirty distinct question/selector pools repeat identically. These are repeated
known-source lookups, not ninety independent user tasks or fresh held-out accuracy.

| Focused question | Compact median native query | Search median native query |
| --- | ---: | ---: |
| Transaction creation | 517.3 ms | 15.6 ms |
| Add an efun | 649.6 ms | 30.7 ms |
| Commit path/checks | 442.7 ms | 29.4 ms |

Across all 45 observations per selector, including the twelve delegated controls, median
native query time is 629.6 ms for compact and 605.4 ms for search. Both pass the provisional
one-second median target. Posting masks avoid much of compact selection's rescanning on
the recognised questions; the delegated controls still pay compact query cost.

Cold capture/index startup is 13.07 seconds. Whole observed time is 61.89 seconds; peak
process-tree PSS is **0.105 GiB**, including one index, driver and observer, with no model.
The 4-GiB observed target passes. Sampling every 250 ms reports no errors and enforces no
cap; this is not per-query peak memory or physical laptop qualification. Builds, tests,
mutations and browser checks finish before timing. The owned native process closes.

## Verification and decision

Fresh workspace tests/doctests pass: **474 tests**. All **20 serial QA test files**,
formatting, all-target Clippy, optimized native builds and the complete headless browser
suite pass. Tests cover unrelated Rust/Python names, macro non-expansion, conservative
intent fallback, topic/operation isolation, long parent scopes, result/byte limits,
overlap boundaries, Unicode floors and held-context invalidation.

Whole-module Rust mutation work covers the search module and raw Rust adapter. An initial
98-mutant run and baseline-checked retests after stronger tests leave **86 caught, two
reviewed equivalents, nine unviable builds and one timeout**. The timeout is a generated
loop-negation change that fails to terminate on an empty topic pool; it is not counted as
caught by failing tests, and the unmodified empty-topic/full-cohort checks pass.
The equivalents use unique bit positions or remove a zero-mask optimization; the latter's
performance is outside correctness mutation checks.

Supplemental diff-scoped checks of shared intent extraction/native transport give five
caught and two unviable replacements. They supplement the whole-module runs. Initial
survivors exposed that the older named-component fixture did not actually activate its
intended purpose-query branch; explicit type/function/project purpose cases now cover it.

Whole Node-helper mutation testing catches **211**, with nine reviewed equivalents and
zero timeouts/errors/no-coverage. Raw score is 95.91; the tool exits 1 at threshold 100.
The [review](results/search-coverage/mutation-review.json) preserves every exclusion,
unviable compiler error and timeout separately. JSON-only type/rejection diagnostics and
equivalent integer parity explain those survivors. CLI/archival orchestration remains
outside Node mutation scope. Every mutation run uses one low-priority worker/compiler task.

The [archive](results/search-coverage/) retains frozen code/source identities, source audit,
all candidates, raw native responses, separate literal/scope assessments, memory samples,
initial/final mutation evidence and validation logs. All 363 checksummed historical
payloads across fifteen archives remain unchanged. Follow [the procedure](SEARCH_COVERAGE.md)
to reproduce the comparison. No dependency, unsafe code, repository source execution or
model call is added.

The decision is to retain the new selector as an experimental comparison. A next revision
should use captured symbol ownership earlier and protect useful baseline hits while
adding candidate paths. The frozen counterexample supplies a concrete target: retain the
Transaction constructor and find the ordinary Task entry without losing the macro and
validator gains. Fresh questions and an independent usefulness trial remain later gates.
