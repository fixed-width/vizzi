# Response-format ablation

For contributors testing whether the output constraint contributes to the evidence-use
failures before choosing a weight edit. Freeze this protocol before new answers. Keep
the model, source states, questions, literal messages, runtime and sampling unchanged.

Use twelve existing development cases: routing, versions and pins, with original,
changed, permuted and missing-definition conditions. Exclude the already-flagged delivery
wording ambiguity before inference. Do not rewrite any question or oracle. The eight
reserved real-source questions receive no generations.

Compare two request conditions: the current response_format JSON schema and omission of
that field. The system instruction continues to request JSON in both. No examples,
schema text, prompt repairs or new order instructions are added. The omitted condition
removes the enforced shape, answer length and key order together; it is not a comparison
with an independently optimised plain-text prompt. All other request fields must match.

Before any generation, call the pinned runtime's /apply-template endpoint with each
exact request body and require equal rendered prompts within every case pair. Preserve
the rendered prompts and request bodies/hashes. If prompts differ, stop before inference
and report the confound rather than call the result an isolated decoding comparison.
Also compare observed prompt-token counts after inference.

Make two passes, alternating request order per case and reversing it on the second pass.
This gives 48 responses in one CPU-only server, two threads, nice 10, serial requests,
thinking off, context 8,192, batches 512/128, seed 42, temperature 0, 300-word instruction,
1,024 output tokens and the existing 360-second timeout. Prefix caching stays enabled.
Record the whole cohort's process-tree memory; do not invent per-arm peaks from it.
Finish tests, mutation work and builds before inference. Close owned processes afterward.

Retain raw output exactly, including prose, fences, invalid JSON, missing fields and
truncation. Do not repair or extract a JSON object. Report parse success, post-checked
schema validity, key order, native reference validity and full-task correctness separately.
Manual factual/unsupported-claim grading also examines raw text when parsing fails, so
format failure cannot masquerade as absence of a code-understanding ability. Raw content
correctness does not establish a usable, source-bound product response.
For content scoring, positive cases need all material facts without unsupported code
claims. Missing-definition cases need an explicit rawAbstention judgment that the answer
explains the unavailable evidence; an empty output or bare No is not credited merely
because the negative case has no positive facts. Parsed status remains a separate metric.

Rerun the constrained condition as a live control. Compare messages/source/settings and
raw outputs against the earlier evidence-use run. Reuse a semantic grade only after exact
raw/context/message/native-validation identity checks; review changed or non-JSON outputs.
Use the existing material facts and explicitly retain the missing-pin bare-No judgment.
Greedy repeats are replays, not independent accuracy samples or latency qualification.

Use TDD for request construction, paired order, output-shape checks and paired assessment.
Keep existing defaults unchanged. Focus mutation testing on the complete new request
builder and decoding-study module; report its limits, survivors and exclusions. Production
Rust and dependencies do not change. A negative result does not prove a weight-localised
cause; no custom edit, reserved evaluation or product integration follows automatically.
