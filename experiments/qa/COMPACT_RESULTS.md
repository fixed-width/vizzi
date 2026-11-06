# Compact evidence and host-resolved citations

For contributors deciding what to investigate next. This fixed-model experiment makes
CPU answers cheaper and references easier to bind, but does not establish a reliable
repository-answer feature. The subsequent three exact maintainer questions all remain
unanswered in every condition. Keep this work outside the viewer.

## Conditions and source scope

Qwen3.5-2B Q4_K_M and llama.cpp b12110 are unchanged from the
[pilot](RESULTS.md). Requests run serially, with two CPU/prefill threads, `nice -n 10`,
no GPU/offload, an 8,192-token context and 384 output tokens. Thinking is disabled.
Answer/status field order and decoding stay fixed. Only the compact citation comparison
holds evidence fixed; legacy versus compact also changes ranking, excerpt length and
source-role/scope metadata. Prompt caching is enabled and arm order rotates by question.

The host is an i7-13700H with 62.5 GiB physical RAM. Both source commits are unchanged:
Vizzi `5ee084d9e7d6876a61af627403ca99fe208d043f` and lpc-rs
`1a4f5b65dd2a269298748df2dfd68092a169a8d9`. Captured scope is 410 files / 3,110,772 bytes
and 931 files / 10,754,851 bytes, plus a three-file temporal fixture. This is selected
Rust/Python/text scope, not Linux/PyTorch or whole-language qualification.

V1 questions and results remain intact. [V2](library/v2/cases.json) preserves lineage,
revises eight ambiguous formulations/rubrics and adds eight source-aware agent questions.
All 32 previously observed formulations are development cases. The
[23-question cohort](library/v2/cohort.json) includes reviewed cases, three production/test
regressions, four temporal cases and all fresh formulations. Selection was fixed before
fresh retrieval/model outputs. Native code, adapters, inputs and binaries were then
[hashed](results/compact/freeze.json). Fresh gold source was audited before candidate
retrieval, without supplying gold prose to inference.

## Retrieval diagnostics

These counts require every preferred literal anchor, not semantic completeness. Alternative
valid evidence can answer a question without those literals, and an anchor can be present
while caller linkage or a control-flow branch is missing.

| Selector / allowance | Development: complete anchors / 27 answerable | Fresh: complete anchors / 8 |
| --- | ---: | ---: |
| Legacy / 8,000 bytes | 9 | 6 |
| Legacy / 2,500 bytes | 6 | 4 |
| Compact / 8,000 bytes | 11 | 5 |
| Compact / 2,500 bytes | 11 | 4 |

The development gain did not generalise to fresh anchor coverage. A root-README
introduction preference was added using development misses; fresh failures were retained.
Byte caps can cut excerpts inside declarations or branches. In the retry question, the
compact source finds production `read_file` but omits its final error branch; both compact
answers reverse the visible extension guard. Source-role hints alone do not solve this.

## Fixed-model results: 69 responses

All counts below use preliminary manual grading by the source-aware case author. Correct
task completion requires the frozen facts/status, no unsupported claims, supporting
citations and native source binding. This is not independent human acceptance.

| Measurement | Legacy / 8 KB / quotes | Compact / 2.5 KB / quotes | Same compact evidence / IDs |
| --- | ---: | ---: | ---: |
| Strict success, answerable tasks | 2/22 | 6/22 | 10/22 |
| Strict success, real repository questions | 1/19 | 3/19 | 7/19 |
| Strict success, fresh formulations | 0/8 | 0/8 | 2/8 |
| Supported required facts | 23/36 | 23/36 | 21/36 |
| Native source checks, including abstentions | 10/23 | 13/23 | 23/23 |
| Correct refusal after fixture deletion | 1/1 | 1/1 | 1/1 |
| Median full response | 48.2 s | 22.3 s | 19.1 s |
| 95th-percentile full response (23 observations) | 55.4 s | 32.7 s | 24.6 s |
| Median first non-reasoning JSON output | 34.9 s | 12.4 s | 13.1 s |
| Median prompt tokens | 2,195 | 848 | 844 |
| Median literal evidence bytes | 7,877 | 2,267 | 2,267 |
| Median retrieval | 0.83 ms | 277 ms | 285 ms |

All 69 outputs parse as JSON; there are no runtime errors or token-cap truncations.
The first output includes the JSON preamble, not necessarily a useful answer word.
The small sample and enabled prefix caching do not qualify cold-cache or tail latency.

Peak observed concurrent process-tree PSS was **2.26 GiB**, sampled every 250 ms with no
observer errors. This includes indexes, inference server and runner. No memory cap was
enforced; this 64 GB host does not qualify a 16 GB device. Builds/tests/mutations were
finished before inference measurements. Both inference servers closed after their runs;
Vizzi viewers were not restarted.

ID binding removes literal-copy and unavailable-label failures. It does **not** improve
factual coverage here. Valid IDs also accompany wrong implementation languages, reversed
conditions, incorrect configuration precedence and false refusals. On several negative
questions, the model repeats the answer and then claims it cannot answer. The driver
configuration case even refuses while quoting the supplied diagnostics/exit branch.

Two frozen rubrics remain too specific: `reader` requires an unasked Tokio I/O API,
and `fresh-relative-source-bases` additionally requires constructor-helper derivations.
They are flagged in [the review](results/compact/review.md), not edited after inference.
A separate [post-hoc sensitivity](results/compact/rubric-sensitivity.json), waiving only
those extra details, gives real-repository successes of **1/19, 4/19 and 9/19**. It repairs
neither quotes nor statuses and does not replace the strict primary result.

## Exact maintainer workflow questions: nine further responses

These questions arrived after the v2 selection freeze. Their wording is preserved in a
[separate cohort](library/maintainer-2026-11-05/questions.json). Source-bound rubrics were
audited and [frozen](results/maintainer/freeze.json) before candidate retrieval. The same
native selector, model, prompt, budgets and decoding were used without tuning on them.
Question authorship is independent of the retriever; gold facts/grading are still agent work.

| Maintainer question | Legacy quotes | Compact quotes | Compact IDs |
| --- | --- | --- | --- |
| how do i add a new efun? | Wrong command/parser-registration procedure | Wrong procedure | Wrong procedure; valid IDs |
| where are new transactions created? | Wrong allocation path | Test helper presented as production | Test helper; unrelated source ID |
| show me the codepath that transaction commits take, with all the checks they go through | Missing relevant source; refusal | Missing relevant source; refusal | Missing relevant source; refusal |

No condition completes any of the three tasks. The efun query retrieves implementations
of existing `add_rule`/`parse_add_rule`/`add_action`, rather than the native `efuns!`
registration table. The transaction query selects `presence/transaction_tests.rs`, whose
parent declaration is `#[cfg(test)]`. The current filename heuristic misses `_tests.rs`
and does not propagate cross-file module configuration, so the helper is incorrectly
labelled `code`. This is a measured classification gap, not a production guarantee.
Commit tracing retrieves class semantics and interactive-explanation JavaScript instead
of the submission chain and `Committer::commit` validation gates.

Median full response is **50.0 s / 23.6 s / 21.4 s**; observed peak process-tree PSS is
**2.21 GiB**. All nine outputs parse, without runtime errors or truncation. The three ID
responses all pass structural binding, including the two wrong procedures. The commit
refusals are faithful to missing context, while still failing the source-specific task.

## Verification and next experiment

Formatting, **449 workspace tests including doctests**, Clippy with warnings denied,
42 QA package tests and five serial Node test files pass. No dependencies or unsafe code
were added. Native QA and Node adapters changed; the browser application did not.

Serial, lower-priority mutation results are recorded in
[mutations.json](results/compact/mutations.json): ID validation has 8 catches / 1 build
failure; the broader selector/metadata/protocol run has 125 catches / 91 survivors /
19 build failures / 2 timeouts. A focused follow-up has 62 catches / 12 survivors /
10 build failures. These rounds overlap and evolve; do not add them as unique coverage.
Ranking/boundary survivors remain unproven and the broader run is not a final full-code
coverage claim. Timeouts/build failures are separate from assertion catches.

Keep the canonical native reference interface and the split between repository memory
and inference. Next, test the three maintainer questions against **source-complete,
source-only diagnostic packs** before investing in another ranker or larger model.
That separates retrieval failure from the model's ability to explain relevant code.
Then improve task-shaped retrieval, source-role provenance and answer/status calibration,
using new maintainer questions for a fresh evaluation. Repository-specific training and
a chat UI are not supported as next steps by these results.

The subsequent [source-complete diagnostic](SOURCE_PACK_RESULTS.md) carries out that
comparison and separately tests a larger response allowance. It retains this cohort's
frozen results and rubrics.

[Reproduction procedure](COMPACT.md). Preserved artifacts include
[main assessment](results/compact/assessment.json), [maintainer assessment](results/maintainer/assessment.json),
[raw main responses](results/compact/answers.jsonl.gz),
[raw maintainer responses](results/maintainer/answers.jsonl.gz),
[maintainer review](results/maintainer/review.md), source audits, freeze hashes, memory
samples and checksums. Raw prompts contain supplied source and questions, never gold
rubrics or grades.
