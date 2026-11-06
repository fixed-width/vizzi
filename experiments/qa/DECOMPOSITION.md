# Run the focused-question and output-order diagnostic

For contributors checking guided extraction/linking and output-order effects after the
[source-complete comparison](SOURCE_PACK_RESULTS.md). The [contract](../../docs/qa/DECOMPOSITION.md)
sets the limits. The [frozen question library](library/decomposition/cases.json) has three
exact maintainer broad controls and twelve source-aware agent subquestions. This does
not implement an automatic question planner or combine subanswers into a final workflow.

Run normal formatting, workspace tests/doctests and Clippy, plus the serial Node QA tests.
The new `order.test.mjs`, `pack-study.test.mjs` and `decomposition-assess.test.mjs` verify
schema ordering, unchanged broad controls/old defaults and paired scoring. No native Rust
logic changes in this iteration, so the existing source-range mutation evidence remains
applicable; no redundant native mutation run is required.

Use fresh paths to audit the same whole packs and freeze this suite:

```sh
node experiments/qa/packs.mjs prepare \
  --suite experiments/qa/library/decomposition/cases.json \
  --repo /path/to/lpc-rs --output /tmp/vizzi-qa-decomp-prepared
```

Preparation checks exact source bytes and required anchors for every question and asserts
one context hash per parent family. Source remains identical for the broad control and
every narrow question in that family. Expected facts and author notes remain evaluator data.

Finish builds/tests before measuring the thirty responses:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-decomp.memory.json -- \
  nice -n 10 node experiments/qa/packs.mjs run \
  --suite experiments/qa/library/decomposition/cases.json \
  --prepared /tmp/vizzi-qa-decomp-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-models \
  --output /tmp/vizzi-qa-decomp-run
```

Each pair changes only the JSON schema's property/required order. Prompt messages and
whole source are identical within the pair. Both orders use 300 words/1,024 output tokens,
eight ID citations and the same model/runtime/resource limits as the source-pack control.
Actual generated key order is recorded; inspect compliance before interpreting the effect.
Pair execution order alternates. Prefix caching is enabled, so timings reflect warm
family context and must not be read as a broad-versus-narrow speed benchmark.

Grade every raw answer for material facts, unsupported claims, status and source support.
Accept equivalent terminology and do not require a literal code spelling when the meaning
is correct. For the handler-interface question, the essential points are EfunContext,
argument access and result placement; the Rust Result<()> spelling is not an additional
independent requirement. Incorrect direct-return or API claims still fail explicitly.
Preserve any further rubric ambiguity as notes; do not edit frozen questions after outputs.

Generate the paired assessment after grading:

```sh
node experiments/qa/decomposition-assess.mjs \
  --input /tmp/vizzi-qa-decomp-run/answers.jsonl \
  --library experiments/qa/library/decomposition/cases.json \
  --grades /path/to/grades.json --memory /tmp/vizzi-qa-decomp.memory.json \
  --output /tmp/vizzi-qa-decomp-assessment.json
```

The assessment keeps broad/narrow counts separate, reports paired improvements and
regressions, verifies equal contexts/prompts and records order compliance. A correct
subanswer does not establish a complete original workflow. Raw responses, grades, source
audits, code hashes, runtime logs, memory samples and constraint observations remain
reviewable artifacts. Agent-authored questions and grading are not independent acceptance.
