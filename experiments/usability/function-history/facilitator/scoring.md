# Assess the callback reading task

Facilitator reference. Keep this file and readiness output separate from the participant
handoff. Freeze it before the session; apply it after each answer is recorded. Use the
existing [classification rules](../../PROTOCOL.md).

## Frozen source evidence

Repository: local `lpc-rs`, upstream `https://github.com/xxx/lpc-rs`.
The workspace is pinned to `457446c870ca3144c315ebcae9c78b7bc2b76514`; its first
parent is `2a6a23ad1be7e7892ce527fa77aab8eea8203353`. The real change is
“Add configurable player preservation for call-outs”. Scope the verdict to
`src/interpreter/task/mod.rs::Task::run_entry`, not acceptance of the whole commit.

The immediately older distinct `run_entry` is captured at
`873e71114814a3a00dde7d41b835635aff95a7d1`. Verify that full identity against local
Git and readiness output before using it. Its declaration must equal the review-base
declaration even though the file observations can differ elsewhere. The next older
version is at `39e934d7c5eec524b65c026f2db29cd0eaf80501`. These version identities
are facilitator checks; do not require the participant to recite expected hashes.

## Learning evidence

Mark each criterion supported / partial / unsupported / uncertain, retaining the actual
answer, source locations, revision and assistance. The reader need not enumerate every
unrelated seed-entry branch.

| Criterion | Expected source-grounded account |
| --- | --- |
| Callback selection | `SeedEntry::Callback` checks owner liveness. A simul callback resolves against the simul-efun program; otherwise target liveness and the current image matter. Local, named and retained function cases differ. |
| Stops and errors | Missing simul object/function and retired owner/target are error paths. Missing named callback sets a nil result and returns without executing it. Avoid treating every early return as an error. |
| Entry and execution | Efun entries use argument values and an entry frame; other functions use `TaskSeed::build_call_frame`, push the frame, then resume. Distinguish this visible code from claims about external implementations. |
| Inspectable evidence | At least one related same-file helper supports the explanation. `runtime_error`, `build_call_frame`, `arg_values` and `value_of` are available in this file. Name matching does not establish semantic dispatch. |

## Review evidence

| Criterion | Expected source-grounded account |
| --- | --- |
| Before/after distinction | The callback arm now binds `origin` and `call_out_player`. Only `CallOrigin::CallOut` takes the new restoration path. |
| Changed context | The saved weak player is upgraded and filtered for liveness in the attempt's transaction. `this_player` is set to the resulting option, and `entry_player` is cleared. Missing/dead players produce no restored player. |
| Retry evidence | The same-file test `call_out_retry_restores_player_and_discards_the_rejected_schedule` forces one rejection, observes two attempts/one conflict, a successful player check, and one surviving scheduled call-out. Reading this test is evidence of intended coverage, not a test execution by this trial. |
| Limits of a local verdict | `open_attempt` calls `run_entry` for an attempt. Scheduling/configuration and all effects of context storage require more source. A defensible local verdict can reserve judgement and name missing evidence. |

Supporting cross-file evidence, if the participant requests it: the changed
`src/interpreter/vm/prioritize_call_out.rs` passes the captured player to
`for_call_out(player)`; `lpc-rs-utils/src/config.rs` introduces the default-off
configuration; `src/interpreter/vm/call_out_player_tests.rs` provides broader cases.
Do not supply these locations before the initial answers. Record cross-file needs and
the actual editor fallback, rather than expanding the product during the trial.

There is no planted defect. Do not grade acceptance/rejection alone, or require a single
preferred verdict. Record justified concerns, missed behaviour, unsupported assertions
and explicit uncertainty separately. Do not infer comprehension from successful clicks.

## Interpret observations

Relate each incident to an intention, visible revision/side, expected behaviour, actual
behaviour and recovery. Distinguish participant report, direct observation and facilitator
inference. Preserve evidence of retained/lost reading position, revision-label confusion,
chooser uncertainty, layout friction and duplicated editor work.

The maintainer's learning answer precedes an exposed secondary review. Neither is an
independent newcomer outcome. There is no editor-only baseline or comparative benefit
claim. A successful agent readiness route supplies no participant outcome. Missing
answers, probes or observations stay explicitly incomplete.
