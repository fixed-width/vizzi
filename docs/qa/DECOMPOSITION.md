# Focused questions and output-order diagnostic

For contributors isolating failures observed in the complete-source experiment at
`21504f9`. Preserve the original three maintainer questions as broad controls. Add
source-aware agent formulations for individual handler/registration operations, creation
and message links, rejection gates and commit lifecycle branches. These are guided,
known-source diagnostics, not independent held-out accuracy or an automatic planner.
The frozen cohort has three broad controls and twelve narrow questions: thirty responses
across the two orders, without synthesising answers between requests.

Each question uses its parent's identical whole curated source pack. Do not narrow the
evidence with the question. Freeze questions, required material facts and the mapping
to packs before new model outputs. Give the model only repository, source and question;
never expected facts, author notes or broader-question answers. Accept equivalent wording
and distinguish missing detail from false claims. Keep per-fact reasoning and raw outputs
for later human review.

Run every broad and narrow question with status-first and answer-first JSON. Change only
the schema's property/required-field order: `status, answer, citations` versus
`answer, status, citations`. Prompt messages, evidence, field meanings/types, eight-ID
allowance, 300 words/1,024 output tokens, 2,000 answer characters, temperature, seed and
resource limits stay fixed. Record actual generated key order; a backend that ignores the
requested order has not tested the intervention. Validate citations with the existing
native-held context and preserve all incorrect statuses rather than repairing them.

Pair conditions on each question and alternate their order. Keep prefix caching enabled
as before and record cached-token usage. Source is shared across questions within a
family, so latency is descriptive warm-session evidence, not a fair broad-versus-narrow
speed benchmark or cold-cache qualification. Run the fixed Qwen3.5-2B Q4 and llama.cpp
runtime with two CPU threads, serial requests, `nice -n 10`, no GPU and no inspected
source execution. Complete correctness checks before inference. Do not change Rust or
run redundant native mutation cohorts for this Node/data-only experiment; retain the
native selection/provenance verification evidence and its limits.

Report broad and narrow success/facts separately, plus explicit unsupported claims,
wrong-method/message-direction errors, false refusals, order compliance, format/source
checks, token/context constraints and observed memory. Do not equate correct narrow
answers with a complete original workflow. No synthesis of subanswers into a final answer
is included in this diagnostic. Keep QA outside the viewer and editor.
