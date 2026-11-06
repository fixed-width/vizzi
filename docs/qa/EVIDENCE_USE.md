# Evidence-use development diagnostic

For contributors identifying a repeatable QA failure before attempting a purpose-specific
weight edit. This contract precedes new outputs. Keep the published edit comparison and
all previous questions/results unchanged. Do not change weights or tune prompts here.

## Questions and splits

Use four small, clearly labelled synthetic Rust source families: message receiver versus
validator, comparison direction, physical-effect ordering, and release after failed
validation. Each family has four cases with the same question:

1. Complete original source.
2. A second source state that changes the answer.
3. The original source with excerpt order/labels permuted; semantics stay unchanged.
4. Source with the decisive definition withheld; the supplied context cannot establish
   the answer and should produce insufficient_evidence.

These are sixteen development cases, each run twice. Synthetic states are content-bound
fixtures, not existing Git commits. Keep every variant of a source family in the same
split. Freeze questions, source bytes/ranges, expected facts, support requirements and
case order before generation. Fixture changes are deliberate review scenarios, not
corrections to production code.

Reserve eight new cases from four separate real-source families in Vizzi and PyTorch:
one complete and one missing-definition case per family. Audit their source/gold but
generate no answers in this cohort. They are source-aware agent-authored future evaluation
cases, not independent maintainer questions or proof of absence from model pretraining.
The author knows their content; future editing must use development data only. There is
no reserved inference mode in this diagnostic runner. Evaluate a future candidate only
after its weights, runtime and prompt are frozen under a separate evaluation contract.

The QA acquisition adapter currently captures Rust/Python but excludes C headers. Do not
expand native capture/language scope for this study. Real-source exports use immutable Git
objects and a small explicit file scope; no inspected project build or script runs.

## Preparation and inference

Reuse the native captured-range selection/reference validation and existing model adapter.
Verify full-file SHA-256, native content/snapshot identity, literal range bytes and required
anchor availability. Preparation records development contexts separately from reserved
contexts. The inference artifact contains only the development inputs and their hashes;
gold facts and expected labels never enter the model messages.

Use the unchanged locally built unedited 0.8B Q4 control from the weight-edit comparison,
pinned b12110 runtime, thinking off, answer-first JSON, 300 words/1,024 output tokens,
eight source IDs, existing 2,000-character answer schema, temperature 0, seed 42,
context 8,192 and logical/physical batch 512/128. One server, two CPU/prefill threads,
serial requests, nice 10, existing 360-second request timeout and process-tree observation.
Finish correctness/build jobs before inference; close the server afterward. No viewer,
editor, model conversion, training, custom edit or new dependency is included.

## Assessment and interpretation

Grade factual correctness, unsupported claims, expected status and semantic citation
support independently of native reference validity. Apply the same rules to both source
states and repeats. Use the existing manual material-fact scoring interface; accept
equivalent prose without requiring literal API spellings. Retain all raw outputs.

Report complete-answer pairs, source-change tracking, label-permutation stability and
missing-source abstention separately by family and repeat. A reproducible failure is the
same mistake class in both repeats of at least two development cases. Record counterexamples
and mixed repeats rather than forcing one failure diagnosis. This threshold selects a
behaviour to investigate; it does not locate a neuron or establish a causal mechanism.

Greedy repeats with prefix caching are diagnostic replays, not independent samples or a
latency distribution. Toy-source results do not establish real-code usefulness. Reserved
cases stay unanswered; old results and current development answers cannot become fresh
held-outs later. Valid negative results include failure to reproduce a narrow error.

The contrast design is informed by
[CheckList](https://aclanthology.org/2020.acl-main.442/), which distinguishes capability,
invariance and directional tests. Work on
[knowledge conflicts](https://arxiv.org/abs/2305.13300) motivates checking whether supplied
evidence changes answers. These papers do not validate our code fixtures or prove that
the observed errors arise from parametric memory; they motivate test design only.
