# Source-ranking prototype results

For contributors evaluating a small model as smarter search. **The source-ranking interface
works, but this first 0.8B ranker does not qualify for integration.** All six proposals are
complete, source-bound orders. Relevance worsens in two presentations, and additional
ranking takes a median 8.73 seconds against a five-second target. Keep the deterministic
baseline as the default. No repository answer, weight edit or viewer feature is added.

The [contract](../../docs/qa/SEARCH.md) precedes inference. The
[source-card trial](results/search/trial.md) lets the maintainer inspect the exact baseline
and both model orders. Its first question is “where are new transactions created?”

## The delivered interface

The Rust `Corpus::reorder` interface accepts a native-held context and a complete label
permutation. It returns the original evidence without changing labels, source bytes,
locations, metadata or snapshot identity. It rejects missing, duplicate, unavailable and
stale labels. The experimental JSON-line host also rejects expired contexts and model
fields that try to supply locations. It reports relevance as unchecked.

An optional model sees only question text and bounded cards: labels, paths/lines, roles,
lexical scopes and literal source. The engine retains content hashes and snapshot binding.
Output is only `{"order":[...]}`. There is no answer, explanation, confidence or generated
location. Every candidate remains available. Invalid/failed proposals fall back to a
freshly validated baseline; a stale baseline fails explicitly. Tests exercise these
fallbacks; none is needed in the six live requests. Valid ordering is distinct from relevance.

Model transport and fallback stay outside the Rust index, so other rankers can reuse
the same interface. The existing viewers and editor handoff remain unchanged. A future
interactive host should expose baseline results while optional ranking runs.

## Source coverage before ranking

All three exact maintainer questions are known development data. The original checksummed
lpc-rs archive at `1a4f5b65dd2a269298748df2dfd68092a169a8d9` is freshly extracted. The
native capture selects the same 931 Rust/Python/text files and 10,754,851 bytes. Index IDs
include implementation identity: adding the module changes the ID even though repository
source is unchanged. Preparation records both index IDs and freezes the new one; runtime
reproduces its snapshot and all three pools exactly.

The unchanged compact selector supplies eight candidates under 4,000 literal bytes per
question. All 24 visible excerpts receive source-aware agent grades and reasons before
any model request: 2 for a direct starting point, 1 supporting context/example, 0 unhelpful.
The [frozen audit](results/search/relevance.json) records omissions and borderline hints.
No gold grades, expected facts or source selectors enter model prompts.

| Question | Useful source present | Important source missing |
| --- | --- | --- |
| Transaction creation | `Transaction::new` at baseline rank 3; a wrapper and test-helper example | The normal `Task::open_attempt` creation caller |
| Add an efun | Native handler examples and an indirect registry note | The `efuns!` registration table, module declaration, tests/documentation extension locations |
| Commit path/checks | A guide and specialized socket `Publication::commit_phase` at rank 7 | Normal Task submission/delivery, `Committer::dispatch` and the validator checks |

Candidate coverage is already a limitation before inference. Reranking cannot restore
these missing locations. Existing role hints also label some test paths as code; they
are never treated as semantic authority. The efun pool has no grade-2 extension mechanism,
and some excerpts duplicate the same handler. No retrieval/classification rules change
after this audit.

## Ordering quality

The model ranks each pool once in its original presentation and once reversed, retaining
native labels and exact card bytes. This is six observations of three questions, not six
independent tasks. All three complete output orders change with presentation.

First direct-result rank refers to grade 2 within the eight candidates. A dash means no
grade-2 source was retrieved, not a successful model abstention.

| Question | Baseline | Model, original presentation | Model, reversed presentation |
| --- | ---: | ---: | ---: |
| Transaction creation | 3 | 1 | 4 |
| Add an efun | — | — | — |
| Commit path/checks | 7 | 5 | 7 |

The transaction constructor moves first in the original presentation, followed by the
test helper and an unrelated class copy-on-write excerpt. Reversing presentation puts
Process fields, architecture prose and the test helper first, demoting the constructor.
That is a useful isolated improvement with an immediate counterexample to stable ranking.

nDCG@3 compares graded relevance in the first three results against the best order of
the same pool. It uses gain `2^grade - 1` and logarithmic position discounts. It does not
measure whether the pool contains a complete answer or whether the developer completes
the task. A pool with no positive grades has undefined nDCG; that case is covered by tests.

| Question | Baseline nDCG@3 | Model, original | Model, reversed |
| --- | ---: | ---: | ---: |
| Transaction creation | 0.758 | 0.879 | 0.274 |
| Add an efun | 0.765 | 0.765 | 1.000 |
| Commit path/checks | 0.242 | 0.000 | 0.242 |

Two presentations improve, two worsen and two are unchanged. The original commit ranking
puts garbage-collection explainer text, historical build-validation prose and visualization
UI exports first: all three are unhelpful for finding transaction commit checks. Its first
direct source moves earlier overall while disappearing from the first three, showing why
that rank alone is insufficient. The efun score of 1.000 only puts supporting examples/notes
first; the extension mechanism is still absent. Duplicate examples and subjective supporting
grades are further limits of this small preliminary audit.

## Cost and verification

Inference uses the cached unedited 0.8B Q4 base and pinned b12110, CPU-only, two inference/
prefill threads, nice 10, context 8,192 and batches 512/128. Requests are serial, thinking
off, temperature 0, seed 42, schema constrained, cache_prompt=false, 64 output tokens and
a 30-second timeout. No model, prompt, retrieval, grade or limit changes follow inference.

| Observation | Measured |
| --- | ---: |
| Additional ranking, median including native validation | 8.73 s |
| Additional ranking range | 8.32–10.12 s |
| Candidate retrieval, three warm-index queries | 0.51–0.72 s |
| Native capture/index startup | 12.31 s |
| Model startup | 1.05 s |
| Peak process-tree PSS, index + runner + model | 1.153 GiB |
| Whole observed run | 70.39 s |

All responses finish normally with 28 output tokens and 1,206–1,446 prompt tokens.
Reported cache_n and cached_tokens are zero throughout. Most model time is cold prefill;
the short output does not make input processing free. The memory target passes and the
median ranking target fails. These six requests are not a tail-latency distribution or
a physical 16-GB-laptop qualification. The observer samples every 250 ms, enforces no cap
and reports no sampling errors. Both owned processes stop.

Fresh workspace tests/doctests pass: **456 tests**. All **19 serial QA test files**,
formatting, all-target Clippy and the optimized native build pass. TDD logs retain the
initial failures and regression fixes. Whole portable-module mutation testing catches
three mutants; one generated stub cannot build because Evidence has no Default. That
unviable mutant is reported separately, with no missed mutants or timeouts.

Whole Node-helper mutation testing catches **191 mutants with two reviewed equivalents**,
zero timeouts/errors/no-coverage. Raw score is 98.96, tool exit 1 at threshold 100.
The [exclusions](results/search/mutation-exclusions.json) explain a redundant type diagnostic
for JSON question values and an early missing-row diagnostic. Initial survivors led to
stronger boundary, multi-question, fallback and ledger tests; noncanonical numeric grade
keys are now rejected. Native/HTTP orchestration is outside the mutation scopes and has
native protocol tests plus six live validated requests. Jobs use one low-priority worker.

All 321 checksummed historical payloads across 14 archives remain unchanged. The
[new archive](results/search/) retains frozen source/model/code identities, inputs, relevance,
requests/responses, source results, quality, memory and validation evidence with
[checksums](results/search/checksums.json). Follow [the procedure](SEARCH.md) to reproduce it.
All eight reserved real-source cases remain unrun. Independent maintainer usefulness
acceptance and fresh-question qualification remain open.

The next engineering priority is candidate coverage: finding the ordinary creation caller,
registration mechanism and commit validator from the original questions. Keep the model
adapter optional while improving that search path. This run supplies no reason to resume
weight editing, ship this ranker or infer a complete code path from its ordering.
