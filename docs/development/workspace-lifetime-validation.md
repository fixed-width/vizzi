# Source-workspace lifetime validation

Contributor evidence for the [ownership contract](../configuration/WORKSPACE_LIFETIME.md),
recorded on 2026-11-14. Implementation follows `ee7bd67` and preserves captured-source
handoff and originating-editor return while concentrating editor-host resource ownership.

## Regressions and interface checks

Deterministic probes reproduced queued browser opening after Stop, a fresh Open joining
cancelled startup, A → B → A ending at B, and Stop Sharing cancelling native adoption.
Tests now exercise `use`, `stop` and `dispose` directly rather than reconstructing command
imports to establish these ownership rules. Command tests retain selection, configuration,
notification and presentation wiring; real child and loopback checks remain separate.

Further regressions cover cancellation during sharing refresh, selection completing just
before Stop, obsolete return values, native/sharing factory results arriving after
retirement, and tab-disposal errors preventing child cleanup. Test adapters record effects
outside cancelled callbacks so swallowed assertions cannot masquerade as verification.

The startup-coordination review reproduced two additional failures: a second same-root
Open could advance sharing order before the original startup acquired sharing, and a
startup failure during that second command's freshness wait could cause an implicit
second launch. Three focused tests first failed, then passed after each operation captured
and joined its existing startup before refreshing sharing. They verify unresolved initial
sharing blocks both opens, both callers receive the identical failure without a retry,
and a fresh Open after Stop Sharing restores sharing only once native startup is ready.

| Check | Result |
| --- | --- |
| Formatting, all-feature workspace tests and doctests | Passed; 794 tests passed, 15 explicitly ignored |
| Workspace Clippy, all targets and features with warnings denied | Passed |
| `node --expose-gc --test editors/vscode/*.test.cjs` | 165 passed, including 67 direct lifetime cases and two retention checks |
| Ordinary editor suite | Same checks, with the two explicit-GC retention checks skipped |
| VS Code extension host on an aliased workspace | Passed: sharing/reconnection, embedded UI, tab reuse, browser handoff, Stop/restart and verified return |
| Real Git extension and embedded review | Passed: working changes, revision pickers and Source Control menu |
| Source-browser editor and inactive-tab menus | Passed: cursor line, retained document and Back/Forward |
| Installed rust-analyzer and embedded call workspace | Passed: `ok_or_else`, captured standard-library source, repeat/restore and return to its captured editor line |
| VSIX packaging and temporary-profile installation | `dist/vizzi-vscode-0.8.1-linux-x64.vsix`; bundled runtime bytes, executable permissions and SHA-256 sidecar verified |

The installed-payload checks use an isolated profile and virtual display; the maintainer's
profile is unchanged. Explicit external-browser launch is intercepted, while the native
HTTP callback and embedded rendering are exercised. Concurrency and closed-editor-channel
failures are injected through resource adapters; these runs do not claim to reproduce an
actual editor crash or qualify other operating systems/editor hosts. The standalone web
application and Rust implementation were unchanged; native baseline checks still ran.

## Mutation evidence

The whole `source-workspace.cjs` module was mutated using the existing Stryker 10.1.1
installation. Runs used `nice -n 10`, one test-runner process, a passing unmodified
baseline, and the command runner with explicit GC. No Rust code changed, so this change
uses JavaScript mutation testing rather than a new cargo-mutants run.

| Pass | Candidates | Caught by failing tests | Timeouts | Survived | Runner/compile errors |
| --- | --- | --- | --- | --- | --- |
| Initial development | 321 | 176 | 4 | 141 | 0 |
| Pre-review whole-module run | 305 | 260 | 7 | 38 | 0 |
| Startup-coordination correction | 308 | 261 | 7 | 40 | 0 |

The implementation and tests changed between these passes; counts overlap and must not
be added into a unique-mutant total. Intermediate passes strengthened sharing reuse,
independent cancellation, current error ownership, cleanup barriers, signal/listener
retention and microtask ordering. One attempted rerun stopped at a failing baseline
because a new test incorrectly expected an obsolete sharing failure to propagate. The
expectation was corrected to silent supersession and a passing baseline was established
before continuing; that attempt contributes no mutation result.

The seven final timeouts are loop/progress mutations and remain distinct from mutations
caught by failing tests. All 40 current survivors were inspected. They concentrate in private
cancellation representation, once-listener behavior with explicit removal, unique request
ordinal equality, and overlapping ownership/cancellation checks. For example, a retired
native record cannot become current again; both its group and record checks protect that
invariant. A valid connection is rechecked at the sharing-operation admission step before
presentation. Retained defensive checks therefore have single-mutation variants that
leave public outcomes unchanged. No function-wide or mutator-wide exclusions were added.

These reviews are bounded by the current adapter contracts and are not a proof of every
survivor's equivalence. The report retains survivors, and the configuration's threshold
is diagnostic rather than a CI acceptance gate. Complete mutation qualification is not
claimed. The raw local JSON is written to
`experiments/browser/results/local/lifetime-mutation.json`; the checked-in configuration
reproduces the serial run. Coverage of editor-specific presentation beyond the scoped
signal integration remains the separate visual-workspace delivery item.
