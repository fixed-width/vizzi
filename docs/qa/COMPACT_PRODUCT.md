# Compact-evidence QA iteration

This experiment follows `5cb25a2` and implements the maintainer's approval to review the
eval seed, improve retrieval, and test smaller evidence with the same CPU model. It adds
no chat UI and does not restart the Vizzi viewers or open an editor/browser.

Preserve v1 questions, grades and raw results. Review ambiguous questions and unnecessary
rubric requirements in a separate v2 dataset with explicit lineage/review notes. Previously
observed cases are development/regression cases. Freeze fresh held-out formulations and
source oracles before inspecting their retrieval/model outputs. Agent-authored held-outs
remain distinct from maintainer-authored questions; do not invent human authorship.

Retrieve source roles and lexical scope as well as bytes. Distinguish production code,
test code, current policy/guidance, configuration and historical reports. Roles derived
from paths or syntax are evidence-selection hints, not guarantees of correctness or
resolved call relationships. Query planning must use only question text and corpus
metadata; gold facts, selectors and case categories never enter the retriever.

Measure development evidence coverage without model inference. Compare the unchanged
legacy selector and the role-aware selector under both 8,000- and 2,500-byte budgets.
Compact excerpts remain literal, version-bound and UTF-8 safe. Preserve relevant source
guards and scope; do not relabel a test double as production code. Freeze selection rules
before evaluating fresh held-outs. Report failures instead of editing held-out questions.

Use fixed Qwen 3.5 2B Q4 weights/runtime, two CPU threads, serial requests, lower priority,
no GPU and the existing context/output/decoding limits. A preregistered controlled cohort
contains the reviewed cases, temporal cases and all fresh held-outs. Compare legacy/8 KB
with compact/2.5 KB using the existing quote protocol. Separately compare the same compact
contexts with an evidence-ID-only protocol, leaving answer/status order and decoding
unchanged. This protocol comparison does not isolate changes within retrieval itself.

The host maps valid evidence labels to canonical snapshot/file/line references. Missing,
out-of-context or stale references fail explicitly. ID validity still does not prove
semantic support. Keep raw output and independent facts/status/reference assessments.
Report measured subsets and denominators, source/model/code hashes, latency, tokens,
memory, source-role errors and mutation limitations. No correctness/performance job
overlap and no production-feature or whole-repository qualification claim.
