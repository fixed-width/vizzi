# Published behavioural weight edit comparison

For contributors testing whether the maintainer's weight-space programming example helps
CPU repository QA. This contract precedes inspection of new model answers. Compare the
published Qwen3.5-0.8B loop repair with its exact base; do not discover or tune a custom
edit in this cohort. Keep QA outside the product interface.

## Artifact identity

Start from Qwen/Qwen3.5-0.8B revision
`2fc06364715b967f1860aea9cf38778875588b17`. Reconstruct the published `p50c` patch from
RikiyaT/lm_understand-logs revision `4d09ba6d6a7e891dd8063974dfe81b3d78e3494f` and verify
the authors' base and edited tensor hashes. Audit all source tensors: only the two
declared MLP output weights may differ, with identical names, shapes and types elsewhere.

Convert both locally with the same llama.cpp source revision as the pinned b12110
runtime. Quantize both to Q4_K_M with the same cached quantizer, settings and two threads,
starting from the higher-precision source rather than an existing Q4 file. Record source,
patch, scripts, environment, conversion commands and all artifact hashes. Check GGUF
architecture, tokenizer, template, tensor shapes/types and payloads: unrelated tensor
changes invalidate an isolated-edit comparison. Verify that a nonzero edit survives
conversion and quantization before running inference. Retain any failed condition.

The historical third-party 0.8B GGUF is not the paired control. The published checkpoint
and card are provenance cross-checks; local reconstruction avoids two downloads and gives
a complete tensor audit. No remote model code, optimizer, training or GPU is needed.

## QA conditions

Use the fifteen unchanged complete-source decomposition questions, answer-first JSON,
300 words, 1,024 output tokens, eight source IDs and the 2,000-character answer schema.
Freeze source, question, runner, model and native binary identities before inference.
Both variants receive identical contexts/messages in identical case order. Preserve the
dispatch oracle correction and every historical result.

Run each variant once in its own process-tree memory observation, serially, with two
CPU/prefill threads, nice 10, no GPU, thinking off, context 8,192, logical/physical batches
512/128, temperature 0, seed 42 and the existing 360-second request timeout. Finish
downloads, conversion, tests and builds before model measurements. Close each server
before starting the other. Record errors, truncation, actual arguments, source checks,
latency, tokens and observed PSS against the existing 4 GiB target.

This evaluates the published patch in Vizzi's deployment condition. It does not reproduce
the paper's long-thinking math benchmarks. Do not enable thinking, change sampling,
increase allowances or tune prompts after seeing answers to rescue a negative result.

## Assessment

Grade both variants using the same corrected material-fact rubric. Review every changed
answer; reuse a grade only with identical case, source, messages, raw output and native
validation. Report full workflows and guided narrow tasks separately, including fact
coverage, unsupported claims, refusals and source support. Resource fit alone is not
task success, and valid references alone do not establish truth.

These are previously observed maintainer questions and agent-authored subquestions,
not fresh held-outs or independent grading. One run per model and prefix-cached case
families do not establish broad accuracy or a latency distribution. A negative result
would concern this patch/model/quantization/non-thinking condition, not weight-space
programming in general. A positive result would need fresh cross-repository questions
before custom editing or product integration. Archive raw answers, artifacts and checks.
