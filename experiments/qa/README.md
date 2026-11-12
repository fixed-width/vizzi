# Archived repository-question experiments

For contributors studying the former repository-question and source-ranking experiments.
These experiments are retired from the product direction and active build. Their records,
fixtures and scripts remain here; `vizzi-qa` has been removed from the Cargo workspace
and current source tree. The app still uses the shared `vizzi-search` library.

The native experiment source is preserved in Git at
[`5f92480`](https://github.com/fixed-width/vizzi/tree/5f92480cf211babaeb7dff46bd0cff430a10f69a/crates/vizzi-qa).
For historical reproduction, use a separate checkout of that revision before following
the procedures below:

```sh
git worktree add --detach ../vizzi-qa-archive 5f92480cf211babaeb7dff46bd0cff430a10f69a
cd ../vizzi-qa-archive
```

The historical procedures retain their original source paths, commands and provenance
hashes. They describe Linux experiments and are outside current platform qualification.
They are historical evidence, not a current development plan.

## Historical pilot

The current development direction is [question-driven source ranking](SEARCH.md): an
optional small model helps order inspectable search results. The earlier answer studies
below remain historical evidence; they do not define the model's product role. Read
[the search explanation](../../docs/explanation/question-driven-search.md) for that distinction.
The [first ranking results](SEARCH_RESULTS.md) show stable source binding but inconsistent
relevance and an 8.73-second median cost. The [source-card trial](results/search/trial.md)
keeps the baseline and both proposed orders inspectable. Candidate coverage is the next
engineering priority; this model is not selected for integration.
The [candidate-coverage follow-up](SEARCH_COVERAGE_RESULTS.md) adds raw macro definitions
and a separate deterministic search selector. It finds four of seven preferred locators
at low query cost but loses the transaction constructor; it stays experimental. The
[source-card comparison](results/search-coverage/trial.md) keeps that counterexample visible.
The [ownership revision](SEARCH_OWNERSHIP_RESULTS.md) restores the constructor's exact
baseline identity at rank 1 and moves the validator first, but loses the receiver and
still misses the Task entry. Coverage remains 4/7, so the selector stays experimental.
The [role-specific revision](SEARCH_ROLES_RESULTS.md) separates direct operation names
from ownership and weak ancestor context. It recovers 6/7 preferred locators, including
the Task entry and receiver, without losing broader controls. The table invocation still
misses; [source cards](results/search-roles/trial.md) are ready for a usefulness walkthrough.

This how-to is for contributors reproducing the experiment in
[PRODUCT](../../docs/qa/PRODUCT.md) and [TECH](../../docs/qa/TECH.md). The experiment is
isolated from the viewer and opens no editor/browser. Candidates answer locally;
the experiment runner does not upload source or execute inspected project code.

The [question templates](library/templates.json) are reusable probes. The
[concrete cases](library/cases.json) bind questions to pinned source, factual checklists,
forbidden claims and exact source needles. A general question needs a separate oracle
for each repository. Development and held-out cases are marked before inference.

For the fixed-model, versioned retrieval and citation comparison, follow the
[compact-evidence procedure](COMPACT.md). The original library and pilot results remain
unchanged. The [follow-up results](COMPACT_RESULTS.md) include exact maintainer workflow
questions and distinguish faster, valid references from correct answers.
The subsequent [complete-source diagnostic](SOURCE_PACK_RESULTS.md) separates selection
misses from remaining model/prompt limits; follow its [procedure](SOURCE_PACKS.md) to
repeat the known-question control.
The [focused-question/output-order follow-up](DECOMPOSITION_RESULTS.md) keeps source and
prompts fixed within schema-order pairs. Its [procedure](DECOMPOSITION.md) reports broad
and narrow results separately and does not implement an automatic planner.
The [fixed-source model comparison](MODEL_COMPARISON_RESULTS.md) measures a pinned 4B
candidate against a reproduced live 2B control. Read the [dispatch source-audit erratum](DISPATCH_ERRATUM.md)
before interpreting older receiver-method grades; original frozen artifacts are retained.
The [physical batch resource screen](PREFILL_RESULTS.md) keeps 4B weights fixed and
misses the 4 GiB target at all three sizes; follow its [procedure](PREFILL.md) to repeat it.
The [weight-space explanation](../../docs/explanation/model-weight-optimisation.md)
distinguishes targeted behavioural edits from compression and runtime allocation changes.
The [published weight-edit comparison](WEIGHT_EDIT_RESULTS.md) reconstructs and audits a
matched 0.8B pair. It finds no complete-task improvement in Q4 non-thinking QA; the
[procedure](WEIGHT_EDIT.md) records artifact construction and evaluation separately.
The [evidence-use development diagnostic](EVIDENCE_USE_RESULTS.md) now tests source changes,
excerpt permutation and withheld definitions with the unchanged 0.8B control. It retains
eight ungenerated real-source reservations and documents an ambiguous synthetic family.
The [response-format ablation](DECODING_RESULTS.md) keeps effective prompts fixed and
finds mixed content changes without aggregate improvement when schema enforcement is
removed. All unconstrained outputs are prose; the existing default remains enabled.
The [generic-object comparison](JSON_OBJECT_RESULTS.md) restores JSON syntax but reduces
raw content correctness from 14/24 to 8/24 and yields no host-conforming responses.
This closes simple enforcement relaxation; the full-schema default remains unchanged.
The [extra-row mutation erratum](MUTATION_ERRATUM.md) corrects two earlier equivalence
classifications without changing experiment answers or scores.
The [conditional token probe](TOKEN_PROBE_RESULTS.md) then tests the persistent pin error:
the source contrast moves Yes/No margins in the expected direction but does not flip the
unconditional original, and source permutation has a larger effect. No edit is selected.
The [matched BF16/Q4 control](BF16_PROBE_RESULTS.md) then changes precision with exact
preflight matching. BF16 changes preferences but resolves none of the four original/guarded
pairs: its diagnostic/prose readout flips both to Yes. It costs more time and memory;
the production defaults and eight reserved cases remain unchanged. Follow
[the procedure](BF16_PROBE.md) to repeat the control.

Build with one compiler task and run the normal correctness checks:

```sh
CARGO_BUILD_JOBS=1 cargo build --release --offline -p vizzi-qa -p vizzi-perf \
  --bin vizzi-qa --bin vizzi-envelope
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo test --offline -p vizzi-qa
node experiments/qa/library.test.mjs
node experiments/qa/model.test.mjs
```

The model-adapter test uses a temporary loopback HTTP stub to check streaming latency.
It does not load a model. Finish builds, tests and serial mutation jobs before timing.

Prepare new source archives and audit every gold anchor:

```sh
node experiments/qa/prepare.mjs --lpc-root /path/to/lpc-rs \
  --output /tmp/vizzi-qa-prepared
```

Vizzi defaults to the current repository; override it with `--vizzi-root`. Preparation
uses read-only Git archive on the library's pinned commits. It does not use uncommitted
source or run inspected builds/scripts. `prepared.json` records source, question, model
configuration and native-binary hashes. Repeat preparation if those inputs change.

Acquire the CPU runtime and weights listed in [models.json](models.json), recording and
checking their immutable revisions, sizes and SHA-256 values. The stable llama.cpp
release v0.9.0 points to build b12110. Download only its CPU Ubuntu x64 archive for the
recorded Linux run. Current later Qwen generations checked were too large for this
pilot; model selection is by resource budget rather than generation number alone.

Example weight download; use the corresponding 2B entry as well:

```sh
hf download bartowski/Qwen_Qwen3.5-0.8B-GGUF \
  Qwen_Qwen3.5-0.8B-Q4_K_M.gguf \
  --revision f36b1ea49a332ede8fe5f389bbf5b3575ef71f48 \
  --local-dir /tmp/vizzi-qa-models/0.8b --max-workers 1
```

Run under the existing safe process-tree observer, using fresh output paths:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-pilot.memory.json -- \
  nice -n 10 node experiments/qa/run.mjs \
  --prepared /tmp/vizzi-qa-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-models \
  --output /tmp/vizzi-qa-pilot
```

The runner processes models/requests serially, uses two inference/prefill threads, an
8,192-token context and a 384-token output cap. GPU layers and operation offload are
zero/disabled, thinking is disabled, extra RAM prompt caching is disabled and no image
projector is loaded. It starts and closes its own model servers; existing Vizzi viewers
are not required. `--model ID` selects one configured candidate for a diagnostic run.

`answers.jsonl` preserves exact prompts, contexts, model output, finish reasons, errors,
tokens, first-output/full-response timing and source checks. The first non-reasoning
output includes the JSON preamble, not necessarily the first useful answer word.
`oracle_source` uses gold-selected source excerpts only and must remain labelled as a
diagnostic, never a production retriever. Deterministic checks establish format and
source binding, not factual correctness. Grade separately using [the rubric](RUBRIC.md).

Memory observations sum concurrent descendants, rather than separate process peaks.
Read the observer's PSS/RSS/sampling limitations in the
[memory procedure](../memory-envelope/README.md). Observe-only results do not establish
an enforced memory cap or qualify the whole Linux/PyTorch repositories.

Captured scope is Rust/Python plus plain JavaScript/TypeScript/HTML/CSS, Markdown,
TOML, YAML and root license text. Raw syntax boundaries are available for Rust/Python;
other inputs use bounded text windows. Generated result/facilitator directories and the
QA library are excluded. LPC source files are excluded. Omissions are counted.
Acquisition defaults cap inventory at 100,000 paths, selected files at 2 MiB each and
32 MiB total. The index separately caps 4,096 files and 65,536 chunks. Search ignores
individual words longer than 128 UTF-8 bytes and uses up to 64 query terms. These are
pilot limits, not a final storage/search architecture.
