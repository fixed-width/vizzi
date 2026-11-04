# Assess source-grounded task outcomes

Facilitator reference: keep this file, the fixture manifest and the expected answers
separate from participant material. Apply after the answer is recorded. All evidence
refers to lpc-rs pin `1a4f5b65dd2a269298748df2dfd68092a169a8d9` and its supplied
working capture, not the current upstream tree.

## Learning task

Mark each item as supported / partial / unsupported / uncertain, with the operator's
evidence and any help. These criteria describe the source behaviour, not required words.

| Criterion | Expected evidence |
| --- | --- |
| Trace acquisition | Compile entry obtains source through the read operation; injected/default reader roles and raw-byte/error contract are distinguished. |
| Explain retry rules | Missing unsuffixed input permits a second read with the c extension. Already-suffixed missing input and other I/O errors stop after the first read. |
| Explain identity and failure | Input path, resolved path and diagnostic name have distinct roles; the underlying I/O error is retained. |
| Justify the explanation | Source and regression-test evidence support the explanation, including retry and stop cases. |

The source paths/declarations and concrete cases are indexed by the existing
[evaluation reference](../../review-workflow/ORACLE.md#supporting-evidence). A correct
summary without inspectable evidence is incomplete for the source-grounded task.
Record partial success and uncertainty rather than manufacturing a numerical score.

## Review task

Use the private session assignment to choose the expected case. The frozen
[review oracle](../../review-workflow/ORACLE.md#expected-verdicts) and
[machine-readable fixture](../../review-workflow/fixtures/oracle.json) specify the
diagnostic change, behavioural difference/control and working-versus-staged evidence.
The untracked document belongs to the change inventory.

Require a reasoned verdict, before/after source evidence and relevant unchanged
contract/test evidence. Rejecting both cases as having the same defect does not pass
the control. Another concern can be legitimate if its evidence supports it. Distinguish
correct discovery, unsupported suspicion, missed behaviour and justified uncertainty.

## Session classification

Record task completion and evidence quality separately from assistance, prior exposure
and interaction friction. Maintainer/guided attempts remain such even if all criteria
are met. An unfamiliar primary attempt must have no prior task/oracle exposure and
no answer-bearing assistance. A secondary task is labelled as previously exposed.
Do not mark independent acceptance from an empty record, setup verification or replay.

Keep real session notes and recordings outside version control by default. Commit an
explicitly reviewed, de-identified summary only when there are actual observations
and a reason to retain it. The [protocol](../PROTOCOL.md) governs conclusions.
