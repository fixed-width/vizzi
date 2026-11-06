# Source-complete QA diagnostic

This contract is for contributors separating evidence-selection failures from model and
response-allowance limits after `9c7789a`. Reuse the three exact maintainer questions and
their source-bound rubrics at the same pinned lpc-rs revision. These are now diagnostic
regressions, not fresh held-out accuracy cases. Preserve all earlier results.

The native corpus selects explicit, half-open UTF-8 byte ranges from captured files.
Each request names the expected snapshot and full-file content identity. Reject stale
identities, missing files, empty/out-of-bounds/non-UTF-8 ranges, overlapping selections,
more than eight selections, and total source above an explicit 1–16,000-byte allowance.
Adjacent ranges and ranges in different files remain distinct. Preserve request order,
literal bytes and canonical file/line identities. Never accept caller-supplied excerpt
text. A failed selection publishes no held context. Existing retrieval/oracle operations
retain their behaviour.

Curated packs specify audited source lines and full-file hashes. Convert lines to byte
ranges without normalising source, then verify them through the native corpus. Include
complete relevant definitions and branches; do not pad or truncate them to an arbitrary
byte target. If a pack exceeds a limit, fail preparation and record the constraint rather
than silently dropping evidence. Source comments already in the repository are allowed;
new explanations, fact checklists, scope notes and gold answers are not model inputs.

Before inference, audit every required fact's source linkage, including callers,
registration machinery, rejection paths and post-commit delivery. Record coverage in an
evaluator-only audit. Line anchors alone do not establish semantic completeness.
Freeze the packs, source capture, model/runner code and conditions before inspecting new
outputs. Keep model weights, runtime, seed, temperature, answer/status order, ID citations,
8,192-token context, two CPU threads, serial requests and lower priority unchanged.
Allow eight citation IDs uniformly in this diagnostic, matching the native limit, so
the multi-block commit chain is not constrained by the previous four-citation schema.
Keep the previous default unchanged for older runners. Use a uniform 360-second request
timeout for these larger source packs; report timeouts rather than interpreting them as
model reasoning failures.

Compare three conditions for each question: automatic compact evidence at its original
2,500-byte allowance and 100-word/384-token response allowance; complete curated evidence
with the same response allowance; and identical curated evidence with 300 words/1,024
output tokens. The latter isolates the combined response allowance, not its individual
word/token components. Context selection changes source relevance, completeness and
possibly size; record actual bytes/tokens rather than treating it as an equal-size ranker
comparison. Reject context overflow as a constraint, not a repaired model response.

Use TDD and serial mutation testing for the native selection boundary. Run native and
Node baseline checks before optimized model measurements, never concurrently with them.
Grade source support, facts, status, citation binding and complete tasks separately.
Preserve raw prompts/responses, memory observations, hashes, incomplete/error states and
case-author grading limitations. No inspected builds, model training, UI/editor actions
or Vizzi viewer startup are part of this experiment.
