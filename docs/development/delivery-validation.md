# Confirmed delivery validation

Contributor evidence for [confirmed visual-workspace delivery](../configuration/DELIVERY.md),
recorded on 2026-11-14 after `e77dd58`. Native handoff, source-workspace lifetime,
originating-editor return and semantic protocol v1 retain their separate responsibilities.

## Behavioral checks

The initial probes reproduced success with no inner application, FIFO application of
obsolete editor requests, and errors from superseded preparation. The implementation now
prepares source without changing the reading context, verifies the originating editor,
commits synchronously against request/document/ticket identities, and confirms application.

Focused tests exercise delayed/hidden readiness, explicit refusal, manual/newer cancellation,
source changes after preparation, unconfirmed application without retry, stale tickets,
mode continuation, history push versus replacement, and readable caller/destination data.
Channel tests execute the actual outer-shell script with MessagePorts: sibling/wrong-origin
hellos cannot bind, and the outer-shell nonce never reaches native pages. Browser tests
verify that an older gated lookup and its error cannot override either a newer editor link
or a manual function selection.

A real editor check caught lightweight inline call descriptors missing their caller field;
selection now supplies that binding before the shared lookup. A failing controller regression
also exposed discarded preparation blocking a coalesced source publication; current source
updates continue after cancellation. Existing manual navigation and call evidence behavior
remain covered by their original tests.

Review probes first failed for a newer request during a dispatched mode change, manual
navigation before routed preparation resumed, and a comparison POST whose ticket had not
yet reached the browser status stream. The regressions now pass through the shared delivery
and Review preparation interfaces. Additional cases cover a second requested mode, manual
cancellation in either departing or arriving documents, a fresh action after cancellation,
stale status notifications, replacement during an in-flight status request, transport
failure, actual comparison replacement and watcher cleanup on abort.

| Check | Result |
| --- | --- |
| Formatting, all-feature workspace tests and doctests | Passed; 795 passed, 15 explicitly ignored |
| Workspace Clippy, all targets/features, warnings denied | Passed |
| Full editor suite with explicit GC | 168 passed, including native/socket checks and real MessageChannel shell checks |
| Shared delivery/navigation unit suite | 118 passed |
| Complete browser regression suite | Passed, including geometry, source history, modes, live updates and review |
| Real VS Code source-browser menus | Passed: active editor and inactive tab, cursor line, retained document and Back/Forward |
| Real VS Code Git menus and review | Passed: working comparison, revision picks and selected capture; review launch does not start live indexing |
| Installed default embedded workspace | Passed on an aliased workspace, including repeat/restore, sharing, shutdown and editor return |
| Installed rust-analyzer workflow | Passed: context-menu call delivery into readable captured `ok_or_else` source, task restoration and return to its editor line |
| VSIX distribution | `dist/vizzi-vscode-0.9.0-linux-x64.vsix`; source/archive/installed bytes and executable permissions verified |

All final workflows were rechecked against the rebuilt 0.9.0 VSIX. One supplemental
Rust run timed out waiting for rust-analyzer definitions before invoking Vizzi; an
isolated retry passed through the actual call menu, readable standard-library source,
task restoration and editor return. This provider-startup failure remains recorded
separately from the passing delivery checks.

VSIX checks use the official CLI with a temporary profile and virtual display. The
source symlink for the shared module becomes a regular installed file. The native
binary and extension carry the same canonical delivery module. The maintainer's profile
is unchanged; no marketplace publication is claimed. Qualification is local Linux,
not every language extension/editor host/operating system.

The editor's final saved-source verification and remote browser application cannot be
atomic. Verification occurs after data preparation, the commit grant remains bounded,
and browser commit checks are synchronous. Applied effects cannot be rolled back if an
acknowledgment is lost. Tests cover an unconfirmed result, not an assurance that no view
was changed. Native framing retains explicit VS Code schemes without a wildcard.

## Mutation evidence

Runs use existing Stryker 10.1.1, `nice -n 10`, one command runner and a passing unmodified
baseline. Scope includes the complete shared delivery, browser connection, call preparation,
source controller and history modules. This is broader than changed lines. The native
changes add static assets only; public HTTP tests verify their availability, content types,
no-store/nosniff and unchanged framing policy.

| Pass | Scope | Candidates | Failing-test catches | Timeouts | Survived | Runner/compile errors |
| --- | --- | --- | --- | --- | --- | --- |
| Initial integration | Five complete modules | 1797 | 926 | 225 | 646 | 0 |
| Strengthened interface checks | Three complete modules | 796 | 552 | 25 | 219 | 0 |
| Captured-binding/manual-navigation checks | Three complete modules | 796 | 644 | 26 | 126 | 0 |
| Review regression fixes | Complete shared delivery and Review adapter | 933 | 304 | 240 | 389 | 0 |
| In-flight status checks | Complete Review adapter | 110 | 69 | 2 | 39 | 0 |
| Transfer and status follow-up | Transition branches and complete Review adapter | 246 | 113 | 70 | 63 | 0 |
| Legacy history follow-up | History replacement expression | 11 | 8 | 0 | 3 | 0 |

The initial pass identified gaps in semantic choice classification, invocation/caller
bindings, warmup cancellation and native document channel setup. Tests were strengthened
through the same public interfaces, and the prepared-view publication regression was added.
Follow-up work covers the three affected whole modules. Further tests independently reject
wrong invocation/caller/snapshot bindings, task replacement, duplicate application and
manual mode navigation, while retaining modifier-click and new-tab behavior.

Survivors were investigated by public behavior and source location. The follow-ups
strengthened independent snapshot/invocation/caller checks, replaced-task rejection,
source-window completion, one-time commit, preserved geometry and readable collapsed
panes. Browser-side tests now cover actual parent/MessagePort binding, mode-link
cancellation, modifier/new-tab preservation and legacy call identities. The source
controller and history modules retain their original regression suites as well.

The three-module pass's 126 survivors concentrate in defensive checks and uncommon
cancellation, cleanup and navigation branches. They are retained in the report and are not counted
as caught or all declared equivalent. The initial whole-module transport run also has
survivors beyond this follow-up scope. This remains a qualification limit; real editor
checks exercise normal channels but do not prove every malformed-packet branch.

The later review-fix pass also exposed missing checks around route-history replacement,
immediate cancellation before preparation arrives, stale transition cancellations and
unexpected destination tickets/modes. New interface tests cover those cases, plus
optional Review tickets/signals and status-fetch failures. The final focused pass checks
the affected transition branches and the complete Review adapter. Other whole-module
survivors remain a coverage limit; broader malformed-packet and cleanup branches are
not all qualified by these regressions. Investigation of the focused survivors added
one final legacy-submission history test. Review survivors include redundant abort
guards, current-status guards behind readiness, and listener cleanup; none are excluded
or all declared equivalent. Transfer survivors include packet/phase validation and
branches unreachable through the normal handshake. The reports preserve them for
further qualification. The last three history survivors alter redundant continuation
matching within an authorized transfer; the separate same-ID and transition checks
still select replacement for valid deliveries. No exclusions were added.

Mutation counts are overlapping passes and must not be added into a unique total. Timeouts
remain separate from tests catching a mutation. No function-wide exclusions are used and
no complete mutation qualification is claimed; the configuration's score threshold is
diagnostic, not an acceptance gate. The initial report is written to the ignored local results directory; the recorded
follow-up report is `/tmp/vizzi-delivery-followup-mutations.json`. The checked-in
configuration includes those five modules and the new Review preparation adapter. Review-fix
reports are `/tmp/vizzi-delivery-review-mutations.json`,
`/tmp/vizzi-delivery-review-status-mutations.json` and
`/tmp/vizzi-delivery-review-focused-mutations.json`; the final history-only check is
`/tmp/vizzi-delivery-review-history-mutations.json`. One added test helper initially
assigned a read-only synthetic Event property; it was corrected before the final passing
baseline and is not a mutation catch.
