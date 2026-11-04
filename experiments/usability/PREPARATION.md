# Maintainer-trial preparation

The session is prepared for a maintainer to try both tasks. This is setup evidence,
not a completed human trial or independent unfamiliar-user acceptance. Answers and
interaction observations remain pending the operator's report.

The learning input is an archive of lpc-rs pin
`1a4f5b65dd2a269298748df2dfd68092a169a8d9`. The review input is one privately assigned
frozen case of that pin. It passes the existing byte/inventory/hunk verifier. The
original checkout's status digest matches before and after preparation. Source copies
and caches are outside the original checkout. No lpc-rs build or project script runs.

Participant material contains the neutral task, controls and notes sheet. The review
brief reuses the frozen task instead of duplicating its wording. Case assignment,
fixture manifest, scorer references and expected answers stay with the facilitator.
This audience separation prevents accidental hints; it is not an access-control
boundary against somebody deliberately reading the evaluator's repository.

The local session lives at `/tmp/vizzi-maintainer-20261104`. Its participant
`START.md` records the live URLs; facilitator records retain the private assignment.
Ephemeral ports and tool-session identifiers are local handoff details. Recreate the
session using [the procedure](README.md) after a restart or cleanup.

[Mechanical validation](results/preparation.json) records source/binary/task hashes
and baseline checks. Health checks do not judge an answer, and no new production
behaviour is implemented by this documentation/preparation slice. Mutation results
for the unchanged viewer remain in the [latest implementation report](../review-workflow/REFRESH_RESULTS.md).

Keep maintainer findings separate from the later unfamiliar-user pilot in
[the protocol](PROTOCOL.md). They can still justify a concrete UI regression test or
help choose a task-motivated next view. They cannot establish a comprehension or
productivity improvement over another tool.
