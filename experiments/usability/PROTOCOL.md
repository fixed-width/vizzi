# Learning and review trial protocol

Audience: facilitators and people interpreting results. Participant-facing material
contains the task and controls; scoring references stay separate. This protocol
evaluates the [product milestones](../../docs/plans/product-milestones.md).

## Purpose and current status

Identify concrete obstacles to learning a subsystem and reviewing a local change with
Vizzi. A maintainer trial is the immediate activity. Independent unfamiliar-user
acceptance remains open until eligible participants actually attempt the tasks.
Preparation checks and scripted walkthroughs are not participant results.

This is a formative task-based pilot. There is no editor/tool comparison condition,
so success, elapsed time or confidence cannot establish a comprehension/productivity
improvement. Think-aloud, assistance and previous exposure also affect elapsed time.
Record individual outcomes and concrete incidents; do not infer population success
rates or timing tails from this pilot.

The [ACM SIGSOFT general standard](https://github.com/acmsigsoft/EmpiricalStandards/blob/master/docs/standards/GeneralStandard.md)
motivates explicit questions, design, evidence and limitations. Its
[human-participant experiment standard](https://github.com/acmsigsoft/EmpiricalStandards/blob/master/docs/standards/Experiments.md)
also highlights tasks/materials, allocation/order and links between validity threats
and conclusions. This pilot borrows that reporting discipline; it is not a controlled
experiment or a claim of compliance with every research standard.

## Session types and exposure

Record role, Rust/source/tool familiarity and exposure to the task, source, oracle or
guided routes before starting. A maintainer or previously exposed operator provides
interaction feedback, not independent newcomer evidence. Agent replays remain guided
functional evidence even when their browser actions are recorded.

For a later unfamiliar-user pilot, give each participant one primary task: learning,
or one review case. Share only that task and the neutral controls/notes. Review case
assignment and expected verdict stay with the facilitator. Cover both frozen cases
with different eligible reviewers; record allocation before the session. Three primary
attempts, one for each learning/review-case cell, are a minimal coverage pilot, not a
sample-size or saturation claim.

A second task is optional and explicitly exploratory. Learning the subsystem before
review, or seeing the other review case, changes prior knowledge. Never pool that
attempt with an unexposed primary attempt. The current maintainer trial may do both
tasks, labelled in their actual order.

## Conduct and record

Use the fixed lpc-rs pin `1a4f5b65dd2a269298748df2dfd68092a169a8d9`, disposable
workspaces, one parser worker and the recorded Vizzi commit/binary. Record the browser,
hardware/display conditions, orientation and any environment failure. Use the same
neutral controls guide. Do not supply target files, declaration names or a navigation
route before the first attempt.

Offer a 20-minute primary attempt and about five minutes for reflection; these are
local scheduling choices, not validated limits or product latency targets. The operator
may stop earlier. Record pauses and extensions. Ask them to describe their intention
and expectation when useful; neutral prompts include “Describe what you expected
here” and “Explain what you would try next”. Avoid hints about the correct answer.

If help is requested, record the last unaided state, the exact cue/tool and its time.
Distinguish interface orientation, navigation hints and answer-bearing help. The
operator can then continue; report assisted completion separately. Do not silently
repair the view or source and count the attempt as unaided.

Collect the answer/verdict and source evidence before optional interaction probes or
debriefing. Then ask about lost context, misleading labels, unanswered questions and
desired external tools. A requested feature becomes a hypothesis: record which task
question and attempted route motivated it. Raw observations and the facilitator's
interpretation occupy separate fields.

## Interpretation and next work

Assess source-grounded correctness using the private rubric, separately from how
pleasant or fast the interaction felt. Count neither favourable comments nor rejecting
every patch as successful review. Record uncertainty honestly. Do not retroactively
change the task or rubric after reading a participant's answer.

A wrong source/side/version label or silent substitution of changed bytes is a
correctness blocker: retain evidence and reproduce it in a regression test. Navigation
obstacles and unsupported questions should name the visible state and user intention.
Prioritise a bounded fix or new view from such evidence, noting whether the incident
was seen once or independently repeated. Wishlist items alone do not select a view.

Maintain separate statuses for preparation, maintainer observations, guided functional
replays and independent primary outcomes. A useful unfamiliar-user demonstration needs
eligible unaided, source-supported outcomes for learning and both review cases; small
coverage still does not establish a comparative usability/comprehension benefit.
Report unsuccessful, assisted and missing attempts alongside successful ones.
