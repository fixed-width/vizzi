# Function-history trial preparation

Facilitator status for [issue #6](https://github.com/fixed-width/vizzi/issues/6),
prepared 2026-11-17. The [procedure](README.md) reproduces the session. At handoff,
participant answers, observed task paths and resulting product decisions were pending.
Subsequent reports are in the [current findings](FINDINGS.md). This preparation record
does not complete the human-trial acceptance criteria.

The selected real lpc-rs change is frozen in the private-audience
[source manifest](facilitator/fixture.json), with full source/declaration hashes and
the review base/target. The [task brief](participant/task.md) supplies neutral learning
and review questions. The facilitator [scoring reference](facilitator/scoring.md)
was written before any participant answer. Participant material is copied separately;
the handoff contains neither expected answers nor a guided task route.

The local session is `/tmp/vizzi-history-trial-20261117`. Its `participant/source`
is a detached complete shared clone at the task target. The original checkout's HEAD,
refs, index and status digests matched before/after preparation. The original object
store must remain available. The only added source-workspace file is `vizzi.toml`,
which selects one worker, an ephemeral port and a cache outside the source.

An isolated VS Code 1.146.0 window was launched with the current 0.9.0 extension
source and a current release executable. The machine's installed 0.6.0 preview is
not the selected development extension. Its profile disables automatic Rust checks,
build scripts and procedural macros. The inspected repository was not built or run.
Plugin startup, trust prompts and participant display conditions still need to be
recorded during the actual attempt; launching the window does not establish successful
plugin interaction.

The participant selected VS Code and supplied a repository/tool-familiarity baseline;
the self-report is retained in the private session record. This is a maintainer trial.
The learning answer precedes an exposed secondary review, with no editor-only baseline.
Recognition of this specific change and any task/oracle exposure still need recording.
The participant was given the task and optional controls, and asked to report the
learning answer before the review answer. No screen/audio recording was started.

## Readiness evidence

[Mechanical preparation evidence](results/preparation.json) records the build hash,
frozen material hashes and check outcomes. A separate native process, cache and Chromium
context exercised the real task source. It found the function through Add bubble,
stepped history backwards twice and forwards once, opened Before/After, checked their
first source sections against independent Git bytes, followed a same-file captured
helper through the paged Calls list, revisited the retained root and returned it live.
The earlier Before/helper views remained available and no page errors were reported.
Rectangle checks support layout retention in that route; they do not establish that
a human reader retained context. The human workspace starts with a fresh task.

Readiness is source-informed automation. Initial harness attempts needed corrected
asynchronous waits, a unique bubble selector and Calls pagination; these are preparation
script errors, not participant incidents or product defects. The initial restricted
host/test attempt could not bind local sockets. The successful runs used loopback access.

Verification on Vizzi `55c878651159ef7064eed816eecbc25f8d6664c5`:

- Optimised locked/offline native build passed.
- Formatting and Clippy across workspace targets passed.
- Workspace tests including doctests passed: 774 passed, 15 ignored across 98 suites.
- The task-specific Chromium readiness route passed.
- Local documentation links and whitespace were checked.

This change adds trial materials and preparation evidence, with no production behaviour
change. No new mutation run or performance measurement was performed. Existing feature
test/mutation evidence remains in the [history contracts](../../../docs/visual/HISTORY_TECH.md),
[comparison contracts](../../../docs/visual/FUNCTION_COMPARISON_TECH.md) and
[captured-call contracts](../../../docs/visual/CAPTURED_CALLS_TECH.md).

## Evidence still required

Record the actual task path, pauses, assistance and editor switches; collect both
source-grounded answers. Observe previous/next history, comparison, captured-helper
following, return to an earlier bubble and Return to live, using post-answer probes
for operations not attempted spontaneously. Capture revision/chooser uncertainty,
layout friction, retained/lost context and unsupported cross-file or lineage needs.
Then assess the frozen rubric, publish a reviewed de-identified findings summary and
open separate issues for reproducible actionable defects. Feature-expansion decisions
remain deferred until those observations exist. Missing observations remain incomplete
validation, regardless of readiness success.
