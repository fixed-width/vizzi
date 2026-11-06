# Source-complete diagnostic results

For contributors deciding whether retrieval alone explains the failed maintainer tasks.
Complete source improves relevant factual content, but **none of the three full tasks is
completed in any condition**. A larger answer allowance does not fix the remaining
synthesis and caller-flow errors in this fixed 2B setup.

## Controlled conditions

The exact three maintainer questions, source revision, rubrics, model weights and runtime
are unchanged from the [previous experiment](COMPACT_RESULTS.md). These are known-question
diagnostics, not new held-out accuracy measurements. The source-aware agent selected and
graded the packs; independent human acceptance remains open.

New native selection accepts explicit ranges in captured files with expected snapshot
and full-file identities. It rejects stale identities, invalid UTF-8/bounds, overlaps and
allowance overflow. It returns exact source bytes and host-resolved canonical references.
It neither expands anchor windows nor truncates complete definitions to fit a budget.

The [pack manifest](library/source-packs/packs.json) records the manual coverage audit,
inclusive source lines and full-file SHA-256/BLAKE3 hashes. Preparation creates a fresh
archive, verifies it against the original pinned source, checks all native-selected bytes
and freezes the source, code, model configuration and binaries before inference.
Evaluator facts, scope notes and manual audit prose never enter prompts.

| Pack | Literal source bytes | Ranges | Content |
| --- | ---: | ---: | --- |
| Add an efun | 9,026 | 7 | Handler and tests, module declaration, complete `efuns!` macro, invocation/example rows, Markdown and documentation generator |
| Create transactions | 5,024 | 5 | Constructor, complete `Task::open_attempt`, snapshot transport/start, committer start arm and command-attempt creation |
| Trace commits | 12,993 | 5 | Complete runner, task commit/deliver, channel helper, committer Commit arm, validation and publication |

Representative registration rows are complete; unrelated efun-table entries are omitted.
The commit arm excludes other protocol arms. The full runner includes admission,
evaluation deadlines, error paths, conflict backoff and successful physical delivery.

Each question runs under three conditions, with arm order rotated by question:

1. Compact retrieved evidence, original 2,500-byte allowance, 100 words/384 output tokens.
2. Complete curated evidence, 100 words/384 output tokens.
3. Identical curated evidence, 300 words/1,024 output tokens.

All conditions retain status/answer order, temperature 0, seed 42, ID citations, no
thinking, two CPU/prefill threads, serial requests, `nice -n 10`, no GPU/offload and an
8,192-token model context. The eight-ID schema is uniform in this diagnostic, matching
the native limit; it raises the previous four-citation cap so the five-block trace can
be cited. The request timeout is uniformly 360 seconds. The 2,000-character answer-schema
cap is unchanged. Earlier runner defaults and archived results are retained.

Curated selection changes relevance, completeness and source size together. This is not
an equal-size ranker comparison. The longer response changes the word/token allowances
together; it does not isolate their individual effects. Prompts differ between those two
conditions only in the word allowance, while request output-token limits differ separately.

## Nine responses

All nine outputs parse as JSON, pass native reference binding and finish normally. There
are no context overflows, runtime failures, timeouts or token-cap truncations. No output
exceeds **201 tokens** or **780 answer characters**, below even the original 384-token and
2,000-character limits. The model does not exhaust the extra allowance.

| Measurement | Retrieved IDs | Curated IDs | Same curated source, longer response |
| --- | ---: | ---: | ---: |
| Complete tasks under unchanged rubric | 0/3 | 0/3 | 0/3 |
| Supported required facts | 0/13 | 6/13 | 4/13 |
| Native reference checks | 3/3 | 3/3 | 3/3 |
| Median prompt tokens | 845 | 3,148 | 3,148 |
| Median full response | 24.6 s | 67.6 s | 68.2 s |
| Median first non-reasoning JSON output | 15.0 s | 51.8 s | 55.9 s |

The first output includes JSON framing, not necessarily a useful answer word. Three
observations per condition and enabled prefix caching do not qualify tail or cold-cache
latency. Manual source selection/audit time is excluded, so the curated condition is
not an end-user performance claim.

Peak concurrent process-tree PSS was **2.19 GiB**, sampled every 250 ms without observer
errors. This run captures one repository index, unlike the earlier multi-index cohort.
It runs on the same i7-13700H / 62.5 GiB host, with no enforced memory cap or 16 GB device
qualification. All compiler/test/mutation jobs completed before inference. The temporary
model server closed; Vizzi viewers and editors were not opened.

## What the source changed

**Efun creation:** retrieval still describes registering a command handler with `add_rule`.
The short curated answer instead finds `efuns!`, its native registration row and signature
fields. It omits implementing/declaring the handler and adding tests/documentation, and
overstates which row fields are mandatory. With the longer allowance it refuses, claiming
prototype/registration guidance is missing despite the supplied macro documentation and
code. The refusal uses only 106 output tokens.

**Transaction creation:** retrieval still points to an unrecognised `transaction_tests.rs`
fixture. The short curated answer correctly locates `Transaction::new` and its snapshot,
Changeset and effect initialization, but omits the normal `Task::open_attempt` caller.
This is a useful constructor lookup, partial under the frozen two-part workflow rubric.
The longer answer adds the command-attempt example but misnames the task method and
reverses the Start message direction. More text does not establish the correct chain.

**Commit trace:** retrieval cannot find the implementation and refuses. The short curated
answer identifies all four real validation gates: future base, unavailable history,
invalidated reads and merge/type mismatch. It still invents `Committer::dispatch`, omits
the submission chain, read-only no-version branch, delivery/retry outcome and pin-release /
eviction ordering. The longer answer gives three gates but omits the merge/type rejection
and confuses the handler/message direction. Neither completes the requested trace.

All three conditions have valid IDs, including wrong procedures and invented caller links.
Reference identity remains distinct from semantic support and completeness. The raw
[case-author review](results/source-packs/review.md) preserves each judgment and its reasons.

## Implication and verification

Retrieval is one demonstrated bottleneck: supplying relevant complete source corrects
the topic and yields useful constructor/validation facts. It is not the only bottleneck:
the same source does not yield complete procedures or reliable caller/message-flow
synthesis in this model/prompt setup. The longer allowance is not sufficient. This does
not prove a fundamental model-size limit; prompting, decomposition and status-first
generation remain confounded possibilities. Do not treat an empty refusal or valid ID
as a correct answer.

Keep the experiment outside the viewer. A useful next diagnostic is to ask narrowly
scoped subquestions of the same frozen source—for example each rejection check or each
caller link—and separately test answer/status ordering. Those would be new controls on
known cases, not held-out accuracy. Consider task-shaped retrieval or model changes
after measuring which missing operations they must support. No fine-tuning is justified
by this cohort alone.

The subsequent [focused-question/output-order diagnostic](DECOMPOSITION_RESULTS.md)
carries out those controls. It preserves these source-pack outputs and does not treat
guided subanswers as completed original workflows.

Formatting, **453 workspace tests including doctests**, workspace all-target Clippy and
six serial Node test files pass. No dependencies or unsafe code were added. The native
range operation and protocol have failing-then-passing tests for exact literal selection,
version/content identity, UTF-8, bounds/budget, overlap/adjacency, ordering and failed
publication. Streaming tests verify unchanged old defaults and the new allowances.

[Mutation evidence](results/source-packs/mutations.json): an initial focused run catches
25 mutations, leaves one reverse-adjacent ordering survivor and has one build failure.
After adding that public-interface case, the broader relevant selection/quote/ID/excerpt
run catches **52**, with **3 build failures and no survivors or timeouts**. Rounds overlap;
these counts do not establish coverage of untouched ranking algorithms or prove correctness.

[Reproduce the diagnostic](SOURCE_PACKS.md). Artifacts include the
[source/code freeze and coverage audit](results/source-packs/prepared.json),
[assessment](results/source-packs/assessment.json), [raw prompts/responses](results/source-packs/answers.jsonl.gz),
[constraint observations](results/source-packs/constraints.json), memory samples, runtime
logs, grades, validation and checksums. Source packs contain existing repository text;
new gold explanations and evaluator rubrics are excluded from inference.
