# Correction to extra-row mutation exclusions

For contributors interpreting the decoding and generic-object mutation reports. Two
survivors were wrongly classified as equivalent: removing the assertion that response
count equals planned count. This is a behavioural coverage gap, not merely different
diagnostics. The original implementations contain the guard and correctly reject the
invalid input; the missing test failed to demonstrate why that guard matters.

The counterexample supplies four planned rows plus a fifth unique, fully graded row for
an already-known case. All planned IDs remain present, so the remaining plan checks do
not reject the extra row. Both historical modules reject the input normally, but accept
it with only the count-guard mutation applied. This is reproduced using the exact archived
source/mutation locations through the exported comparison interface, without model inference.

| Historical run | Misclassified survivor | Correct classification |
| --- | --- | --- |
| [Response-format ablation](DECODING_RESULTS.md), source in its final mutation report | decoding-study.mjs mutant `123`, count assertion at line 20 | Behaviour-changing survivor; original tests lacked an extra unique graded row. |
| [Generic-object comparison](JSON_OBJECT_RESULTS.md), source in its final mutation report | decoding-study.mjs mutant `155`, count assertion at line 29 | Same behavioural gap. |

Historical raw totals do not change: decoding remains 234 caught / 13 survivors, of which
twelve are equivalent and one is this gap; generic-object remains 260 caught / 15 survivors,
of which fourteen are equivalent and one is this gap. Original mutation reports, exclusion JSON,
logs and checksums are retained as recorded. This explicit correction supersedes those two
equivalence judgments rather than silently rewriting frozen evidence.

All actual experiment ledgers have their exact planned counts, and original guards were
active. Consequently no generated answer, manual grade, content score, task score or
resource result changes. The correction concerns the strength of the tests and the accuracy
of the mutation audit. Similar early-ID guards remain equivalent when later property access
still rejects the malformed input; a total-count guard cannot be treated that way without
checking extra unique rows as well as missing/duplicate rows.

An extra-row regression is now added to both paired scoring and the new token-probe
effects scorer. The current paired-study mutation rerun catches 261 mutations with fourteen
equivalent survivors, zero timeouts/errors/no-coverage; the previously surviving count guard
is caught by a failing test. It is a new run, not a relabelling of a historical survivor.
The raw 100% threshold remains unmet because the reviewed equivalents remain survivors.

[Counterexamples and current regression evidence](results/token-probe/ledger-audit/)
retain exact source hashes, mutation IDs/locations, supplied counts and acceptance results.
The counterexample script can be run from the repository root. The historical sources are
loaded from their immutable mutation archives; their unchanged support-module hashes are
recorded with the evidence. This correction does not consume reserved questions.
