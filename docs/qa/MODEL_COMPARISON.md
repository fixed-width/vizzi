# Fixed-source CPU model comparison

For contributors testing whether the previous errors persist in a larger CPU candidate.
Preserve the fifteen frozen questions, curated source, gold material facts and prior
results. Compare Qwen3.5-2B and Qwen3.5-4B Q4_K_M from immutable model revisions. This is
a model/capacity bundle comparison, not an isolated parameter-count law or generation
comparison. The quantized artifacts, upstream cards and runtime compatibility are pinned.

Use answer-first output for every question in both runs, selected from the earlier
diagnostic before this comparison's outputs. Keep prompt messages, field schema/meanings,
eight IDs, 300 words, 1,024 output tokens, 2,000 answer characters, temperature 0, seed 42,
no thinking, 8,192 context tokens, two CPU/prefill threads and 360-second request timeout.
One model server at a time, serial requests, Unix nice 10, no GPU or inspected code
execution. Models retain their packaged templates/tokenizers; this is not a claim of
identical tokenization or each model's optimal vendor-recommended sampling.

Rerun the 2B model as a live control. Compare actual source hashes/prompts within every
question pair, preserve generated JSON key order, and record all errors/constraints.
Keep prefix caching enabled and case order unchanged, so timings describe warm-family
sessions rather than tail/cold-cache benchmarks. Keep broad/narrow results separate.
These known-source expert-guided diagnostics are not independent held-out accuracy.

The target is at most 4 GiB observed concurrent process-tree PSS per run, a budget target
rather than an enforced cap. Report any exceedance explicitly; a 3 GB weights file does
not establish a fit. If a model cannot load or exceeds a context/runtime constraint,
report the condition rather than silently changing runtime, quantization or resource
settings. Do not load both models together. Baseline checks finish before inference.
No Rust or dependency changes are needed; retain native provenance/mutation evidence.

Extend the existing pack runner with explicit immutable model configuration, model ID
and output-order selection while preserving prior defaults. Freeze the comparison data,
code and binaries before outputs. Grade factual support, unsupported claims, status,
citations and complete tasks separately, with raw responses and agent-grading limitations.
No training, automatic planner, chat UI or editor/Vizzi viewer startup is part of this work.
