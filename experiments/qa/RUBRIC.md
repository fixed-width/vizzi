# Grade repository-question answers

This reference is for an evaluator reviewing raw candidate responses. Keep evaluation
facts outside the indexed source and model prompts. A question template is reusable;
its expected facts and evidence must be authored and audited separately for each source
revision. Keep old cases intact when adding a new revision and give replacements new IDs.

Each case declares repository, pinned revision (through the source manifest), category,
question, template if applicable, development/held-out split, answerability, required
facts, alternative evidence anchors and forbidden claims. Freeze these before reading
candidate outputs. Held-out cases must not drive prompt/ranker changes. The first library
is an agent-authored seed, not a validated independent human benchmark.

Assess these separately:

- **Retrieval:** report preferred-anchor coverage per required fact and all-facts coverage.
  Missing a preferred literal does not prove that no alternative valid evidence exists.
- **Format/provenance:** structured response, available source labels and exact nonempty
  quotes from the supplied snapshot. These checks do not establish entailment.
- **Facts:** mark each required fact supported, missing or contradicted. Accept equivalent
  wording and independently justified alternative source evidence.
- **Unsupported claims:** identify statements that neither the supplied evidence nor
  ordinary interpretation of the source establishes. Do not accept a valid but irrelevant
  quotation as support.
- **Abstention:** distinguish correct refusal on an unanswerable question from failure to
  answer an answerable repository question. A refusal may be faithful to insufficient
  retrieved context while still missing the user's task.
- **Task result:** correct, partial, incorrect or abstained. Correct requires all material
  facts, no forbidden/unsupported claims, appropriate answerability and supporting valid
  citations. Correct refusal requires an unanswerable case and no invented facts.

Report answerable task success, unanswerable refusals, false confident answers, missing
facts and invalid/misattributed citations separately. Never report format validity or
preferred-anchor recall as answer accuracy. Oracle-source diagnostics distinguish
retrieval problems from failures with useful source already supplied.

Record grader identity, reference familiarity, per-fact decisions and concise reasons.
Agent grading by the author of this library is preliminary and has an obvious shared
perspective; preserve raw responses for maintainer/independent review. The runner does
not call an automated judge API. A later evaluation should add maintainer questions,
paraphrases, unfamiliar-user questions and more repositories/languages.

Latency reports separate index preparation/update, retrieval, first non-reasoning JSON
output and complete response. Record truncation/errors as failures or incomplete runs,
never repaired successes. Memory reports include the index/runtime/runner together and
state whether a cap was enforced. Do not extrapolate this small source scope to kernel
or PyTorch scale.

A preferred-anchor diagnostic is not a proven upper bound: snippets can contain each
named fact but omit the caller relationship, component alias or surrounding guard that
connects them to the question. Inspect that linkage before attributing a refusal solely
to the model. Record seed ambiguities without changing the frozen case after observing
an output. The current seed's language hints, purpose/mudlib granularity, and distinction
between inferred relationships and automatic verification need independent review.

Also inspect answer text independently of its status field. A candidate may label an
answer `insufficient_evidence` while actually stating a supported negative answer, or
claim `answered` without establishing a material fact. Protocol correctness and useful
factual content are separate axes. Literal-copy errors caused only by Markdown wrapping
can be diagnosed separately, but must not silently replace the cohort's frozen strict
source-binding result or be treated as semantic entailment.
