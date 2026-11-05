# CPU repository-question experiment

This experiment implements the maintainer's request after `5ee084d`: answer questions
about unfamiliar code with a small local model and external repository memory. Model
training is optional future research. A chat interface is not part of this experiment.

Start with pinned Vizzi and lpc-rs source archives. Never execute inspected project
tooling. Include source and useful documentation/manifests, with explicit scope and
omission counts. Preserve source paths, content identities and snapshot bindings.

Maintain a versioned question library with reusable, repository-independent templates
and concrete codebase-specific questions. A reusable question still needs a separate
repository-specific oracle. Each runnable case records its source revision, question,
category, split, expected facts, forbidden claims, evidence anchors and answerability.
Include absent information, changes that alter answers, and branch/version separation.
Keep development and held-out cases separate before observing model outputs.

Questions and source excerpts are the only repository material supplied to the model.
Never supply gold answers, evidence selectors or rubric hints in a retrieval prompt.
An explicitly labelled oracle-context diagnostic may use gold-selected source excerpts,
but not the gold prose answer. Preserve raw model output and failures without repairs.

Report retrieval evidence coverage, answer/citation format validity, exact quote/source
binding, abstentions, semantic correctness, unsupported claims and omissions separately.
Valid citations do not prove that a claim follows from them. Semantic scoring uses the
frozen factual checklist and records who graded it; reference-aware agent scoring is
preliminary and must not be presented as independent human evaluation.

Compare quantized 0.8B and 2B Qwen 3.5 candidates on the same evidence/output limits,
CPU-only inference, two CPU threads, serial requests and lower priority. The official
catalogue currently offers later Qwen generations at substantially larger sizes; newer
models remain interchangeable candidates when they meet the resource budget.

Measure retrieval/indexing, first-token and complete-answer latency, model startup,
tokens, context size and concurrent process memory separately. Target an observed
combined peak below 4 GiB for the pilot, without calling observations an enforced RAM
qualification. Stop model processes after their cohort. Run performance observations
after correctness and mutation jobs finish. No GPU or cloud answer generation is used.

Provisional interactive goals are a warm median first non-reasoning output token within five
seconds (including the JSON preamble) and complete response within twenty seconds under these fixed limits. Report
p95 as well; these are experiment decision budgets, not established product guarantees.

The result is a reproducible baseline, a question library and an honest assessment of
retrieval and generation bottlenecks. Linux/PyTorch scale and unfamiliar-user usefulness
remain separate qualification tasks. Failure to meet quality or resource targets is a
valid experimental result, not a reason to change the questions after seeing outputs.
