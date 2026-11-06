# Run the CPU repository-question pilot

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
