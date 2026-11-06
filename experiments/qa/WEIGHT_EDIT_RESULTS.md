# Published weight edit on CPU repository QA

For contributors assessing the maintainer's weight-space programming example. **The
published 0.8B edit does not improve complete QA tasks in this paired Q4 non-thinking
diagnostic.** Both variants complete one guided question out of twelve and none of the
three original workflows. The edited variant produces more false insufficient-evidence
statuses and more answers with unsupported claims. Both fit the observed memory target.

This concerns one published loop repair under Vizzi's current deployment condition.
It does not refute weight-space programming or reproduce the article's long-thinking
math evaluation. No custom edit is discovered in this cohort.

## Thirty matched answers

The [contract](../../docs/qa/WEIGHT_EDIT.md) precedes new answers. The fifteen unchanged
questions receive identical complete source, messages and response settings. All thirty
requests finish normally, with no runtime/format errors or truncation. All fifteen raw
answers change, so every response is reviewed; no grades are reused.

| Measurement | Exact base, local Q4 | Published edit, local Q4 |
| --- | ---: | ---: |
| Complete broad workflows | 0/3 | 0/3 |
| Complete guided narrow tasks | 1/12 | 1/12 |
| Supported narrow material facts | 7/18 | 8/18 |
| Supported broad material facts | 0/13 | 0/13 |
| False insufficient-evidence statuses | 4/15 | 11/15 |
| Answers with unsupported claims | 9/15 | 12/15 |
| Native reference validation | 15/15 | 15/15 |
| Requested output order | 15/15 | 15/15 |
| Median full-response latency | 6.35 s | 8.30 s |
| Longest response, also empirical p95 at n=15 | 30.99 s | 36.00 s |
| Median first-output latency | 1.25 s | 1.15 s |
| Median output tokens | 121 | 190 |
| Observed peak process-tree PSS | 1.194 GiB | 1.198 GiB |
| GGUF size | 541,903,872 bytes | 541,903,872 bytes |

The same `Task::open_attempt` question succeeds in both runs. No complete task flips
success/failure. Isolated facts improve or worsen separately. A zero broad material-fact
score does not mean every sentence is wrong: those composite facts require specified
workflow steps/checks. Neither variant consistently supplies them.

These are known maintainer questions and source-aware agent subquestions, graded manually
by the question/pack author, not independent human acceptance or held-out accuracy.
[Base grades](results/weight-edit/base/grades.json) and
[edited grades](results/weight-edit/edited/grades.json) preserve individual judgments.
The unchanged [corrected library](results/dispatch-erratum/decomposition-library.corrected.json)
and [dispatch erratum](DISPATCH_ERRATUM.md) govern both runs. `Committer::dispatch` is real;
these erroneous answers confuse the validator with the message receiver.

Both broad efun procedures confuse internal macro/module machinery with implementing a
native handler. Both broad commit explanations reverse the future-version comparison
and misdescribe call direction. Reference validity is weaker than semantic support:
both state the correct `FutureVersion` outcome but cite a different supplied range.
The edited variant fixes that answer's status while still lacking its supporting citation.

Accept equivalent constructor wording consistently: both narrow task-entry answers name
the right method and describe snapshot-based creation without every wrapper spelling.
The base effects answer really does describe a deadline-bounded backoff branch; that is
not invented. Its second material fact remains incomplete because it omits evaluation
of a fresh admitted/backed-off attempt. Partial facts, unsupported claims, status and
citation support are assessed independently.

## Artifact isolation

Start from Qwen/Qwen3.5-0.8B at `2fc06364715b967f1860aea9cf38778875588b17` and reconstruct
`p50c` from RikiyaT/lm_understand-logs at `4d09ba6d6a7e891dd8063974dfe81b3d78e3494f`.
The [published card](https://huggingface.co/RikiyaT/Qwen3.5-0.8B-weight-space-programming)
and [rebuild code](https://github.com/RikiyaT/lm_understand-logs/blob/4d09ba6d6a7e891dd8063974dfe81b3d78e3494f/exp8_math/code/apply_edit.py)
provide upstream tensor identity checks; the edited full checkpoint is not downloaded.

All 488 source tensors are audited. Only the two declared MLP output tensors differ,
matching the authors' exact hashes. The patch changes 130,188 BF16 entries; the article's
131,072 touched coordinates include values rounded back to their original bits.
All other names, shapes, dtypes and payloads remain identical.

Both variants use the same pinned llama.cpp converter revision as runtime b12110,
the same BF16 intermediate recipe, cached quantizer, Q4_K_M settings and two quantizer
threads. No importance matrix, requantization, training or GPU is used. Current stable
compatible conversion packages are recorded, including CPU-only PyTorch 2.15.0+cpu and
Transformers 5.22.2; application dependencies do not change.

Each BF16/Q4 pair has 335 text-model tensors. Metadata, tokenization, template, layouts
and unrelated payloads match; exactly `blk.15.ffn_down.weight` and
`blk.19.ffn_down.weight` change. The edit therefore survives Q4. Block quantization can
change additional coordinates within those tensors. The historical third-party 0.8B
GGUF has a different build and is not this control.

The [model manifest](library/weight-edit/models.json) pins the local files:

- Base SHA-256: `94d3009ee9cfca48d7d852bf0fe6c14c6f4f2506e818ca63f8c873fa46320a0a`.
- Edited SHA-256: `d18f0ffdc661ebcadd2fdb6adfd7cfd6cb98e46af8e0e0a186d9c6c16e43e2cd`.

The [build record](results/weight-edit/build.json), compressed source/BF16/Q4 audits and
package list retain provenance. `gguf_parameter_count` is stored tensor elements, not
unique upstream parameters. No weights are committed. [The procedure](WEIGHT_EDIT.md)
describes rebuilding/measuring the pair.

## Resources, verification and limits

Inference uses thinking off, two CPU/prefill threads, nice 10, one slot, context 8,192,
logical/physical batches 512/128, temperature 0 and seed 42. Both receive the same
300-word/1,024-token/eight-ID limits, 2,000-character schema and 360-second timeout.
Source is lpc-rs `1a4f5b65dd2a269298748df2dfd68092a169a8d9`. Actual runtime arguments
differ only in model path and ephemeral loopback port.

Each model runs once, base first, with prefix caching within families. Medians mix
cold-family and cached requests; edited outputs are longer. No repeated trials or
counterbalancing establish a latency distribution, slower arithmetic per token or an
intrinsic loading-speed change. Startup is 1.08 versus 0.84 seconds; whole observed
cohorts, including indexing/startup, take 160.1 versus 191.5 seconds.

PSS is a 250-ms sampled process-tree sum including observer, driver, native index and
model. Both peaks are below 4 GiB, without sampling errors, but no cap, physical 16-GB
laptop or editor workload is emulated. Sampling can miss transient peaks and excludes
unmapped filesystem cache/kernel charges. The inactive 12-GiB observer default is not
the QA target. The small peak difference is not an established intrinsic memory change.

Before inference, 453 Rust tests/doctests, formatting, all-target Clippy, thirteen serial
Node test files and eight Python audit tests pass. The audit first fails without its
implementation; real small GGUF CLI fixtures exercise acceptance and rejection of lost
edits, extra changes and changed metadata. Rust is unchanged, so no new native mutation
cohort runs; Python fixtures are not mutation coverage. Build/preparation/model hashes
are rechecked and historical experiment payload checksums remain unchanged. Both servers
close; no viewer/editor or inspected-project build runs.

[Artifacts](results/weight-edit/) retain raw answers/source/messages, grades, assessment,
runtime/run logs, memory samples, build audits and verification logs.
[Checksums](results/weight-edit/checksums.json),
[comparison integrity](results/weight-edit/comparison-integrity.json) and
[validation](results/weight-edit/validation.json) record the checks and limitations.

The published patch is not a useful QA improvement here. A custom purpose-specific edit
would need a repeatable evidence-use failure and mechanistic hypothesis, with separate
development questions and fresh cross-repository held-outs. These thirty diagnostic
answers do not establish a production answer feature. The
[weight-space explanation](../../docs/explanation/model-weight-optimisation.md) puts this
result alongside the broader research idea.
