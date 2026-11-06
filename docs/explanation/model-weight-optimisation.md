# What changing model weights can improve

For contributors deciding how to improve the small local QA model. Weight changes can
alter how it uses supplied evidence as well as reduce storage. Vizzi's stable purpose
could make behavioural editing useful across repositories, while the external index
continues to supply changing, revision-specific facts. These are separate hypotheses.

Model performance includes at least answer quality, resident memory, prompt-processing
speed and generation speed. An optimisation can help one while hurting another. Reducing
the byte size of a file does not establish peak process memory or CPU throughput.

## Programming behaviour through weights

The maintainer's example, Takehi et al.'s November 2, 2026
[weight-space programming report](https://rikiyat.github.io/blog/weight-space-programming/),
uses an LLM to inspect and modify another network without gradient descent. Its Qwen3.5-0.8B
experiment reports a targeted repair of thinking loops and MATH-500 increasing from 7.9%
to 48.1%. Its Gemma follow-up did not establish an improvement in ordinary reasoning.
This is promising early evidence for repairing an identifiable obstruction to existing
ability; it does not establish arbitrary new capabilities from a purpose statement.

There is a concrete artifact to investigate. The
[released checkpoint](https://huggingface.co/RikiyaT/Qwen3.5-0.8B-weight-space-programming)
identifies two changed MLP output tensors in layers 15 and 19, supplies tensor hashes,
and links a bit-exact reconstruction. The
[rebuild code](https://github.com/RikiyaT/lm_understand-logs/blob/main/exp8_math/code/apply_edit.py)
checks both base and edited tensor hashes and retains their shapes. Applying a known
patch and discovering a useful new patch are different costs.

The [experiment log](https://github.com/RikiyaT/lm_understand-logs/blob/main/exp8_math/README.md)
records 158 turns, 80 Python runs and 20.3 hours for the 50-pass Qwen run, including
4.7 hours waiting for an unreachable model. A forward-pass request could contain hundreds
of sequences. The editing kernel was CPU-only; that does not establish a cheap,
end-to-end CPU discovery procedure. The checkpoint card's reported reproduction uses
GPU inference, long thinking outputs and a different runtime from ours.

Related literature offers more constrained edits. [ROME](https://arxiv.org/abs/2202.05262)
locates and updates factual associations through feed-forward weight changes;
[MEMIT](https://arxiv.org/abs/2210.07229) scales association updates to thousands on GPT-J
and GPT-NeoX. These results concern specific architectures and association tasks. They
do not demonstrate faithful reasoning over a changing repository or transfer to Qwen3.5.

For Vizzi, a useful hypothesis is a stable improvement in evidence use: following supplied
call paths, covering requested checks, or distinguishing missing evidence from a complete
answer. Our current runs already disable thinking. They omit material facts even with
complete source, so the published anti-loop patch may address a different failure.
Purpose narrows the evaluation; it does not identify a circuit or guarantee an edit.

Shape-preserving edits leave the dense computation and weight allocation broadly intact.
They could improve time to a correct answer by reducing repetition, retries or the model
size needed for acceptable quality. They do not inherently increase tokens per second or
solve the 4B memory miss. Discovering a patch can use a larger model offline without
requiring that larger model in the deployed inference path.

The [paired CPU comparison](../../experiments/qa/WEIGHT_EDIT_RESULTS.md) now tests the
released 0.8B edit against its exact base with matched local conversion/quantization.
The edit survives Q4, but complete tasks do not improve in our non-thinking diagnostic
and false insufficient-evidence statuses increase. Results in the published thinking
configuration would be a distinct condition. Quantization can change or erase a small
edit, so a BF16 result alone would not validate deployment in Q4.

Any custom edit needs an observed development-set failure, a mechanistic hypothesis and a
reversible, hashed patch. Judge it on fresh source-bound questions and regressions for
missing evidence, misleading names, changed revisions and incomplete call paths. Measure
unsupported claims and completeness as well as speed; fewer refusals alone is not
success. The fifteen already-inspected questions remain diagnostics, not fresh held-outs.
The published patch is evaluated; no custom weight edit has been discovered here.

The [evidence-use diagnostic](../../experiments/qa/EVIDENCE_USE_RESULTS.md) now reproduces
errors with the unchanged 0.8B control: it can follow an altered condition yet invent a
call path after source permutation, and assert behaviour when its definition is missing.
That establishes a narrow behaviour to investigate, not a located circuit. Output grammar
and presentation need controlled checks before attributing the failure to particular
weights. A wording ambiguity and presentation-count shortcut in the starter cases are
documented so they cannot become hidden evaluation assumptions.

## Practical weight transformations

The current 4B artifact is already Q4_K_M. The
[prefill diagnostic](../../experiments/qa/PREFILL_RESULTS.md) retains its exact SHA-256
while varying physical batch size, keeping runtime allocation effects separate from
behavioural edits and precision changes.

**Quantization** stores tensors at lower precision, reducing weight storage/bandwidth.
The pinned runtime supports mixed/per-tensor quantization and calibration importance
matrices. A lower-bit format is a plausible next weight experiment, but this file is
already quantized. Converting the existing Q4 into Q3 cannot recover information lost
during the first conversion; the local `llama-quantize --allow-requantize` help warns of
quality loss relative to starting from higher precision. Prefer a separately pinned,
freshly quantized artifact or higher-precision source and evaluate quality again.
CPU speed depends on the actual format/kernel, not just nominal bits.
[llama.cpp quantization documentation](https://github.com/ggml-org/llama.cpp/blob/cd959bae99ff4bed22bef2604f45241c09bbb8c8/tools/quantize/README.md).

Calibration can preserve quality more effectively than treating all weights equally.
For example, [AWQ](https://arxiv.org/abs/2306.00978) uses activation statistics to protect
important channels without backpropagation. That is evidence for a method, not a measured
gain for this Qwen artifact on this CPU. Its reported platform/kernel results do not
transfer automatically to our GGUF/runtime combination.

**Structured compression** removes computation by shrinking dense matrices or removing
supported components. [SliceGPT](https://arxiv.org/abs/2401.15024) replaces matrices with
smaller dense ones; its experiments concern other model families/hardware. Some such
methods are post-training calibration procedures; others need recovery training. Our
pinned quantizer has a `--prune-layers` option, but the option's existence is not proof
that arbitrary removal is sound for Qwen3.5's hybrid attention/recurrent architecture.
Architecture, conversion and kernel compatibility need separate validation.

**Unstructured pruning** sets selected values to zero. A dense CPU matrix multiplication
still visits the same matrix shape, so zeros alone do not establish fewer operations or
lower resident allocation. [SparseGPT](https://arxiv.org/abs/2301.00774) demonstrates
post-training sparsification without retraining and also considers structured patterns.
Real speed requires the appropriate sparse representation and supported kernels, with
quality/resource measurements on our task. It is not a general licence to zero arbitrary
parameters and expect a speedup.

**Storage layout/packing** can improve access to the same quantized values when the
runtime and CPU have matching kernels. This changes representation rather than teaching
the model repository facts. Benefits are architecture/backend-specific. The existing
llama.cpp CPU backend already selects specialised implementations; any offline repacking
needs compatibility checks against the pinned build rather than renamed tensor metadata.

## Training and semantic edits

Fine-tuning or a [LoRA adapter](https://arxiv.org/abs/2106.09685) can change answers, but does not ordinarily shrink the dense
model's dimensions or make its base matrix multiplications cheaper. A merged adapter can
avoid separate adapter overhead; that is different from accelerating the original base.
[Distillation](https://arxiv.org/abs/1503.02531) trains a smaller student and may improve its quality/cost trade-off, but
requires a training/evaluation programme. Neither is a quick manual edit to a handful of
constants, and repository-specific training is not required by the current project design.

## Order of experiments

The physical-batch cohort missed its memory target. Further runtime allocation work,
lower-bit artifacts and behavioural edits would answer different questions and deserve
separate conditions. The released behavioural patch offers a concrete test of the
maintainer's idea; allocation profiling or matched lower-bit comparisons address memory
more directly. The published patch comparison does not improve complete QA tasks; a custom
edit would need a separate failure hypothesis. Preserve the distinction between fewer bytes, fewer operations, lower
observed memory, faster tokens and correct source-specific answers.
