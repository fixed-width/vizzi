# Fixed-source CPU model comparison

For contributors assessing a larger local model under the existing budget. Qwen3.5-4B
Q4_K_M completes all twelve guided narrow tasks in this known-source diagnostic, compared
with five for the live 2B control. It is slower, uses **4.76 GiB observed process-tree PSS**
and therefore **misses the declared 4 GiB target**. Neither model completes the three
full workflows under the unchanged material-fact rubric. No product integration follows.

## Source-audit correction

This comparison exposed an evaluator mistake: `Committer::process` is a public wrapper
around the real `Committer::dispatch` handler. Calling `dispatch` invented was wrong.
The native source/scope evidence and model answers were correct on that narrow point.
The [dispatch oracle erratum](DISPATCH_ERRATUM.md) preserves original libraries, raw runs,
grades and assessments and publishes explicit corrected overlays and changed judgments.

The earlier output-order result is corrected from **2/12 versus 4/12** to **3/12 versus
5/12** for status-first versus answer-first. Broad completion remains 0/3. The present
comparison uses the corrected answer-first live-control grades and corrected receiver
oracle. No gold facts entered inference, so this scoring correction changes no prompts
or model outputs. It is a post-output source-audit correction, not a hidden model tune.
Agent-authored source audits and grading still require independent human review.

## Candidate selection and controls

The [official 4B card](https://huggingface.co/Qwen/Qwen3.5-4B) documents the model family,
architecture and supported non-thinking mode. The selected
[Q4_K_M distribution](https://huggingface.co/bartowski/Qwen_Qwen3.5-4B-GGUF/tree/4168f45a16a1290d65a4ec0fa312ae917a4c15d6)
uses the `qwen35` architecture already supported by the pinned runtime. This is a
controlled model/capacity/quantization/template bundle comparison, not a claim that only
parameter count changed, that a newer generation is free, or that these are each model's
optimal vendor-recommended settings.

The [model manifest](library/model-comparison/models.json) retains the original 2B artifact
and pins the candidate:

- `Qwen_Qwen3.5-4B-Q4_K_M.gguf`, 3,013,027,808 bytes.
- Quantized repository revision `4168f45a16a1290d65a4ec0fa312ae917a4c15d6`.
- SHA-256 `13c16f426047e2de38cd075bdade4a7bcbc8c774384876f677740cda65f8a983`.

The download used one lower-priority worker, resumed a dropped connection, and completed
with the expected size/hash. No weights are committed. llama.cpp b12110 and the same
runtime executable hash load both models successfully; no runtime upgrade, GPU/offload
or vision projector is used.

All fifteen [frozen questions](library/decomposition/cases.json), curated source, source
revision and response settings remain fixed. All use answer-first JSON, eight IDs,
300 words/1,024 output tokens, 2,000 answer characters, temperature 0, seed 42, no thinking,
8,192 context tokens and a 360-second timeout. Prompts and evidence hashes are checked
within every model pair. Models retain their packaged tokenizers/templates; observed
prompt-token counts also match for these inputs.

There are two separate 15-response runs. Only one model server is resident at a time,
with serial requests, two CPU/prefill threads and `nice -n 10`. Downloads, builds and
correctness work finish before timing. The live 2B control reproduces all prior
answer-first prompts, evidence and **15/15 raw responses byte-for-byte**. Grades are
reused only after that check, with the explicit dispatch correction applied.

## Corrected paired results: thirty responses

These are previously observed broad controls and source-aware agent subquestions, not
new held-out accuracy or an automatic decomposition planner. Grading is preliminary
manual work by the source-aware question/pack author. Complete tasks require material
facts/status, no unsupported claims, supporting citations and native binding.

| Measurement | Live 2B Q4 | Candidate 4B Q4 |
| --- | ---: | ---: |
| Complete original workflows | 0/3 | 0/3 |
| Complete guided narrow tasks | 5/12 | 12/12 |
| Supported broad facts | 5/13 | 8/13 |
| Supported narrow facts | 14/18 | 18/18 |
| Answers with unsupported claims, all questions | 7/15 | 0/15 |
| False refusals | 0/15 | 0/15 |
| Native reference checks | 15/15 | 15/15 |
| Answer-first order compliance | 15/15 | 15/15 |
| Median full response, warm-family sequence | 14.4 s | 38.8 s |
| Median first non-reasoning JSON output | 2.6 s | 6.8 s |
| Median output tokens | 116 | 138 |
| Peak observed process-tree PSS | 2.21 GiB | 4.76 GiB |
| Within 4 GiB observed PSS target | Yes | **No** |

The candidate improves **seven** paired tasks, worsens **zero**, shares five successes
and shares the three incomplete broad workflows. All outputs parse and finish normally;
no runtime failures, context overflows, timeouts or token truncations occur. Maximum
candidate output is 347 tokens / 1,205 answer characters, below the configured caps.

The larger model correctly handles conditions that the 2B control misreads: a newer base
returns `FutureVersion`; historical reads skip the base itself; merge folding uses current
committed values; read-only publication returns before version creation; rejected attempts
retry without publishing physical effects; and live-snapshot emptiness gates eviction.
Its dispatch answer is correct—this is where the faulty oracle was discovered.

## Remaining broad-task gaps

The 4B model gives a useful native efun procedure: implement the Rust handler using
`EfunContext`, register the `efuns!` signature row and add Markdown. It omits adding tests,
so the combined tests/documentation workflow fact remains incomplete. This is partial
under the frozen rubric, not a claim that the answer contains no useful information.

For transaction creation, it locates `Transaction::new` and explains the Task/command,
Start and current-world-snapshot sequence. It does not name the required normal
`Task::open_attempt` entry method in the broad answer. The focused answer supplies that
method and its non-joining condition correctly. Accepting creation/cloning terminology
at the snapshot-data level does not require a literal call to `LiveSnapshot::clone`.

For commit tracing, it describes the submission/receiver/validator chain and all four
real rejection checks with relevant references. It still omits the read-only no-version
branch, physical-effect/retry outcome and pin-release/eviction ordering from the full
answer, despite answering those focused questions correctly. Correct narrow facts are
not automatically assembled into a complete original workflow.

The [corrected review](results/model-comparison/review.corrected.md) records all material
facts, ambiguities, citation choices and reasons. A broad user question can be helped by
a partial answer while still failing a fuller source-bound rubric. Independent maintainer
review is needed to decide usefulness and whether that rubric matches the intended task.

## Resource cost and decision

The host is the same i7-13700H with 62.5 GiB physical RAM. Process-tree PSS samples every
250 ms include one native repository index, runner and one inference server. Both observers
finish successfully without errors; neither run enforces a cap or qualifies a 16 GB device.
The 3.0 GB candidate weight file did not establish RAM fit: its measured peak is 4.76 GiB.
The budget miss is retained; no quantization/runtime/resource setting is silently changed.

Prefix caching stays enabled. First cold family requests for the candidate take roughly
101–220 seconds; warm requests still pay generation cost. Initial runtime logs show
about 24–26 prefill tokens/s and roughly 4 generated tokens/s on two threads. The medians
above describe this grouped warm-source sequence, not cold-cache, tail latency or
per-question interactive responsiveness. The first output includes JSON framing.

The candidate demonstrates that small CPU models can extract these guided code facts
more reliably than the original 2B setup when supplied complete source. It does not
establish reliable whole-workflow answering, automatic retrieval/decomposition or a fit
under the current 4 GiB target. Keep the experiment outside the viewer and avoid training
or a chat feature on this evidence alone.

A bounded next resource experiment can vary prefill batch/microbatch size with the same
4B weights, source and prompts, measuring memory, latency and any output changes before
considering a context/quantization change. Do not assume that will reach 4 GiB. If the
budget cannot be met, the trade-off must be revisited explicitly. Before product acceptance,
collect fresh maintainer questions and independently review complete workflow usefulness.

## Verification and artifacts

Formatting, 453 workspace tests including doctests, workspace all-target Clippy and
twelve serial Node test files pass. No Rust/dependency/unsafe changes occur. Existing
native selection/provenance mutation evidence remains 52 caught / 3 unviable within that
prior relevant scope, not a JavaScript or untouched-ranking coverage claim. New tests cover
immutable model selection, fixed response conditions, paired source/message equality and
the dispatch source-oracle regression. Both model servers are closed; no editor or Vizzi
viewer was opened.

[Reproduction procedure](MODEL_COMPARISON.md). Artifacts include
[corrected comparison](results/model-comparison/assessment.corrected.json),
[control reproduction](results/model-comparison/control-reproduction.json),
[source/code freeze](results/model-comparison/prepared.json.gz),
[2B raw responses](results/model-comparison/2b/answers.jsonl.gz),
[4B raw responses](results/model-comparison/4b/answers.jsonl.gz), separate memory/runtime
traces, original/copied and corrected grades, constraints and checksums. The
[source-audit erratum](DISPATCH_ERRATUM.md) links all superseded historical judgments.
