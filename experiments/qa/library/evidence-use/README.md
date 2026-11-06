# Evidence-use question library

Reference for contributors adding or auditing cases. These are source-aware agent-authored
questions and oracles. They are not independent maintainer evaluation or proof that real
code is absent from model pretraining. The [study contract](../../../../docs/qa/EVIDENCE_USE.md)
defines inference and assessment; [the procedure](../../EVIDENCE_USE.md) defines operation.

`cohort.json` pins source files/identities, two repeats and the unchanged 0.8B control.
`development.json` contains sixteen cases in four synthetic Rust source families:

| Family | Original state | Changed state |
| --- | --- | --- |
| `routing` | `Coordinator::dispatch` receives Apply and calls validate. | validate receives Apply and calls dispatch. |
| `versions` | Reject requested > committed. | Reject requested < committed. |
| `delivery` | Validate before delivering an effect. | Deliver before validation. |
| `pins` | Call release after either validation result. | Call release only after successful validation. |

Every family also has an original-source permutation and a missing-definition case.
`relabel` permutes excerpt order and prompt-local labels together. It does not isolate a
label-number effect from source-position effects. `missing` omits `target` and retains
adjacent helpers, which do not establish the requested behaviour. Expected status is
answered for complete cases and insufficient_evidence for missing cases.

`reserved.json` contains eight cases: full/missing pairs for PyTorch hook removal and
device-index handling, plus Vizzi evidence removal and text-diff guards. Sources use
immutable revisions and a deliberately small file scope. Real-source cases are reserved
for later evaluation and have no model answers in this cohort. Previous Vizzi/PyTorch
exposure does not make these independent repository-level hold-outs.

Each case records a stable ID, family, source, split, condition, question, authorship,
expected status, required facts, forbidden claims, source ranges, minimum supporting
range keys and a scope note. Source descriptors record full-file SHA-256 and a unique
identity needle used only to obtain the native captured content identity. Range keys are
evaluator metadata; actual model labels come from native selection order.

The [first result](../../EVIDENCE_USE_RESULTS.md) flags delivery's physical-effect wording
as ambiguous for its in-memory marker. That family is retained but excluded from the
post hoc clarity-checked diagnosis. All missing cases also have fewer excerpts; future
qualification needs count-matched controls and a fresh freeze. Do not silently rewrite
these recorded inputs or use this starter reservation alone to qualify an edit.

All variants of a family belong to one split. Case IDs, source families and identical
full-file bytes cannot cross development/reserved splits. A changed-source case must
change both source identity and gold facts; a relabel case must preserve the original
range set and gold. Reserved case content is known to the author, so any future editing
process must deliberately exclude it rather than claim a blinded evaluation.
