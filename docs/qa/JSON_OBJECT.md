# Generic JSON-object comparison

For contributors separating broad JSON enforcement from the detailed response schema.
Freeze this protocol before new answers. Reuse the twelve clearer development cases,
oracles, unmodified 0.8B Q4 model and messages from the response-format ablation. Keep
delivery excluded and all eight reserved real-source cases unrun.

Compare the current full schema with response_format={type:json_object}, omitting only
its schema property. The pinned server treats this as a generic object constraint. All
other request fields and prompt instructions stay identical. Do not add schema examples,
repair keys, extract objects or coerce values after generation. Generic JSON does not
promise the required fields, types, answer length or source labels.

Use one server, two CPU/prefill threads, nice 10, serial requests, thinking off, context
8,192, batches 512/128, seed 42, temperature 0, the same 300-word instruction, 1,024 output
tokens and 360-second timeout. Run two passes with alternating first-arm order per case,
reversed on pass two: 48 responses. Finish correctness and single-worker mutation work
before inference. No dependency/Rust changes, training or weight edit are included.

Preflight all exact request bodies through /apply-template before generating answers.
Require equal rendered prompts, preserve them and compare observed prompt-token counts.
Stop before inference if a pair differs. Record the whole cohort's memory rather than
assigning a shared-process peak to either arm. Close all owned processes afterward.

Report JSON parsing/root-object validity, the existing strict schema, key order, native
bindings, raw content and complete tasks separately. Correct content in differently named
fields remains visible as content, without being converted into a valid host response.
Positive raw content needs all requested code facts with no unsupported code claims;
negative content needs an explicit rawAbstention judgment explaining missing evidence.
Keep status/calibration and citation choice separate, retaining the prior bare-No rule.

Rerun full-schema controls and compare them with prior source/messages/raw/native results.
Reuse grades only after exact identity checks. Review every new/changed generic response
in full, including contradictions after a correct opening. Greedy repeats and cached small
synthetic contexts are not independent accuracy samples or real-code qualification.

Use TDD and focused mutation testing for the complete request builder and paired-study
module. Preserve old cohorts, archives and defaults. Reuse the runner; no reserved mode
or new product interface is introduced. A negative result is valid and does not locate
a weight mechanism. Reserved qualification still needs count-matched controls and a
separate candidate freeze.
