# Question-driven source ranking experiment

For contributors testing the model as search assistance. The user asks a question and
receives inspectable source results. The engine owns source identities, excerpts and
locations; an optional model suggests their order. There is no generated repository
answer, inferred call graph or model-supplied location. This contract precedes inference.

## First slice

Reuse the pinned lpc-rs source and the three exact maintainer questions: transaction
creation, efun extension and commit checks. They are known development questions, not
held-out evaluations. The first question is the concrete walkthrough. Keep all eight
reserved evidence-use questions unrun and all previous archives unchanged.
Verify the original source archive checksum and extract it afresh. Index identity also
includes implementation version; record the old index ID and freeze the new one without
treating that difference as changed repository source.

Retrieve with the existing compact selector, limit eight and 4,000 literal source bytes.
Do not change retrieval rules after inspecting ranking outcomes. Record candidate recall
separately: reranking cannot recover source absent from the pool. Results retain native
labels, lexical scopes, source roles, snapshot/file identities and literal ranges. A
role or scope is a syntax/path hint, not proof of runtime authority or a resolved caller.
Model cards contain labels, paths/lines, role/scope hints and literal excerpts. Keep
content hashes and snapshot identities in the engine and run records, outside the prompt.

The baseline order is usable without inference. The model can return only a permutation
of every supplied label, with no answer prose, confidence, path, new result or deletion.
The suggested first three results narrow the initial reading; all candidates remain
available. A malformed, incomplete, duplicated, unavailable or timed-out permutation
keeps the baseline and reports the failure. Native validation rejects stale contexts;
a stale result must not be presented as current. An empty pool makes no model request
and means no matches in this selected search scope, not proof of repository absence.

The portable Rust interface validates a complete order against a held current context
and resolves the original excerpts. Keep model transport, prompts and fallback policy
outside it. Extend the existing experimental native JSON-line host; do not put inference
inside the index or add a production viewer dependency. No repository-specific names,
gold facts or relevance grades enter the query rules or model prompt.

## Measurement and decision

Before inference, audit the eight candidate excerpts per question and freeze relevance
grades: 2 for a direct implementation/extension starting point, 1 for useful supporting
context/examples, 0 for an unhelpful match. Grade the visible source rather than an unseen
rest of its file. Record reasons and whether constructor/caller or other expected anchors
are missing. This is source-aware agent grading, not independent usefulness acceptance.

Use the unchanged cached 0.8B Q4 base and b12110 first. Rank each pool once in baseline
presentation order and once with candidate presentation reversed, preserving labels and
bytes: six requests. This checks sensitivity, not independent samples. Keep CPU-only,
two inference/prefill threads, nice 10, context 8,192, batches 512/128, no thinking,
temperature 0, seed 42, 64 output tokens, schema-constrained JSON, cache_prompt=false
and a 30-second request timeout. Do not retry or tune after observing results.

Report baseline and suggested top-three graded relevance, nDCG@3 within the candidate
pool, first direct-result rank and order sensitivity. Preserve failures as baseline
fallbacks, with raw validity reported separately from effective search quality. An
empty relevant pool has undefined nDCG, not a fabricated successful zero/one score.
Retain exact prompts, raw responses, output order, validated source results and timings.

Measure index/candidate retrieval separately from additional ranking latency; observe
one native index, runner and one model server together. Use the 4-GiB observed PSS target
and a provisional five-second median additional-ranking target. They are experiment
budgets, not caps or product guarantees. Baseline results should not wait for inference
in a later interactive host. No viewer/editor opens in this experiment.

Demonstrate the transaction question with source cards and its unchanged baseline.
Improvement requires useful source moving earlier without a relevance loss on another
question in either presentation order, valid permutations and the resource/latency
targets. A miss is a negative result; do not add query rewriting, a larger model or
another format ablation in this slice. A successful development diagnostic would still
need fresh questions and a maintainer trial before product integration.

Use TDD for complete-order validation, stable provenance, source refresh/expiry,
fallback and ranking metrics. Run focused whole-module mutation checks serially at
low priority and the normal native/Node checks before optimized measurement. Stop all
owned processes. No new dependency, training or weight edit is required.

The design uses permutation ranking as studied by
[Sun et al.](https://aclanthology.org/2023.emnlp-main.923/), without assuming their web-search
results transfer to this model or code. The reversed presentation control is motivated
by our own earlier layout failures and
[Liu et al.](https://arxiv.org/abs/2307.03172); it is not a comprehensive position-bias test.
