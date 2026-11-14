# Configuration and editor bridge validation

Contributor evidence for the [configuration contract](../configuration/PRODUCT.md)
and [editor protocol](../configuration/BRIDGE.md), recorded on 2026-11-12. Implementation
started from `3cc0677d1f06ee043354e087268d2acb9bacf3b2`. Real editor/VSIX integration
was qualified locally on Linux; hosted native checks are recorded in the CI follow-up.
These checks do not qualify every installed language extension.

## Correctness and packaging

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo test --locked --workspace --all-features --no-fail-fast` | 757 passed, 0 failed, 15 explicitly ignored; includes doctests |
| `CARGO_BUILD_JOBS=1 cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | Passed |
| `npm run test:editor` | Seven Node bridge/provider tests passed |
| Isolated VS Code extension host | Passed on VS Code 1.146.0, Linux x86-64, including a symlinked workspace |
| Packaged extension | VSCE 4.1.0 produced `dist/vizzi-vscode-0.1.0.vsix`; code and project license bytes verified against source |
| VSIX installation and runtime | Installed successfully into a temporary profile; installed payload passed the isolated extension-host check |

The real editor check exercises native TOML startup, public definition/reference handlers,
dirty-buffer refusal, stopping and reconnecting. It registers deterministic VS Code
providers, rather than relying on separately installed language servers. The fifteen
ignored integration tests were not qualified by this run. Existing browser rendering
code was unchanged; no new rendering or performance qualification is claimed.

The VSCE stable version and Node requirement were checked against the public npm registry
before use: 4.1.0, Node >=22. Packaging stages an explicit runtime-file list and the project
licenses, without test files or npm runtime dependencies. The preview is a local artifact;
it was not published to a marketplace or installed into the maintainer's editor profile.

## Mutation testing

All runs used cargo-mutants 27.1.0, `nice -n 10`, `CARGO_BUILD_JOBS=1`,
`RUST_TEST_THREADS=1`, `-j 1 --jobserver-tasks 1`, and a passing unmodified baseline.
The scope was whole configuration, editor-bridge and language-routing files, plus the
shared C/C++ reference-offset helper. It was not limited to changed lines.

| Pass | Scope and completion | Test failures caught | Survived | Unviable builds | Timeouts |
| --- | --- | --- | --- | --- | --- |
| Initial development | Interrupted after 13 of 153 candidates | 6 | 7 | 0 | 0 |
| Strengthened-test checkpoint | Interrupted after 105 of 157 candidates | 92 | 8 | 5 | 0 |
| Continuation | Completed 100 remaining candidates; iteration reused 97 prior caught/unviable outcomes | 91 | 2 | 7 | 0 |
| Final survivor rerun | Completed three selected candidates | 2 | 0 | 1 | 0 |

These overlapping passes are reported separately; their counts are not a unique-mutant
total or a coverage percentage. The interrupted passes alone do not establish full coverage.
Compiler failures are not counted as mutations caught by tests.

Survivors prompted exact input/output-frame, file, workspace-byte, file-count and result
boundaries; diagnostics that distinguish rejected metadata from a later connection failure;
invalid declaration cases; the first overflowing MiB value; and managed dependency-catalog
admission through the public provider interface. C++ qualified methods and conversion
operators also received a failing regression before sharing their identifier-offset logic
with the managed adapter.

One equivalent candidate retained a JSON frame's terminal LF instead of slicing it off.
The implementation now parses that complete frame directly, removing the unnecessary
operation. No manual mutation exclusions were added. All observed behaviour-changing
survivors were caught on rerun; unviable candidates remain compiler evidence only.

No optimized benchmark or process-memory comparison was performed. Avoiding a second
Vizzi-managed server follows the selected ownership path; this run does not quantify
RSS savings or qualify an editor's full transitive configuration/dependency environment.

## CI follow-up

The first hosted run for PR #3, [43290165792](https://github.com/fixed-width/vizzi/actions/runs/43290165792),
exposed two differences from the initial local Rust 1.99.1 checks:

- The macOS test peer inherited nonblocking IO from its listener. Three large-transfer
  regressions failed immediately with `WouldBlock`. Explicitly setting the accepted
  stream to nonblocking mode reproduced the same failure on Linux; the fixture now
  explicitly restores blocking IO before its timed reads and writes. All fifteen
  editor-bridge tests pass locally after the correction.
- Hosted stable Rust updated to 1.100.0. Its Clippy flags the final cancellation guard
  in `SourceSpool::write_range` as `needless_bool`; `Ok(!cancelled())` preserves the
  single cancellation check and its result without suppressing the lint.

Rust 1.100.0 was installed in temporary toolchain directories without changing the
maintainer's default compiler. The full native presubmit passed with that compiler:
757 tests passed, 15 were explicitly ignored, and formatting and Clippy passed. A
focused mutation deleting the final cancellation negation was caught by existing spool
tests after a passing baseline: one candidate, one caught, zero survivors, unviable
builds or timeouts. This focused check does not establish whole-spool mutation coverage.

Hosted Linux/macOS confirmation is available in the [PR checks](https://github.com/fixed-width/vizzi/pull/3/checks).

## Bundled extension follow-up, 2026-11-13

The 0.2.0 extension adds Open Workspace, an optional project-root `vizzi.toml`, native
process ownership and visible sharing progress. Regression tests first reproduced the
silent empty-window/cancelled-picker paths and the missing no-argument single-folder
workflow. These establish failure paths in the previous implementation; they do not
identify which path caused the maintainer's original missing-Output report.

| Check | Result |
| --- | --- |
| `npm run test:editor` | 20 command, native launcher, bridge and provider tests passed |
| Release helper tests | 13 passed, including complete VSIX asset sets, matching native bytes and platform metadata |
| Source extension host | Shared providers and Open Workspace launch, browser handoff, reuse and shutdown passed |
| Installed Linux x86-64 VSIX | Installed into temporary VS Code directories; included executable and Open Workspace integration passed |
| Packaged payload | Runtime sources, README, native executable, executable permissions and checksum verified |
| Documentation | 49 local/repository links and TOML examples validated; project-local settings also launched from a parent directory |
| Workflow dependencies | Modified CI/release workflows retain their existing locked action-reference sets |

The installed-payload harness copies its test entry point into an isolated copy of the
extension so its VS Code API context matches the runtime's. Browser opening is intercepted
there; the tests verify the handoff URL and real HTTP listener, not the operating system's
browser integration. The maintainer's installed extension/profile was not changed.

Native formatting, workspace tests/doctests and Clippy ran during this session. The
sandbox blocked editor-bridge socket tests; all fifteen passed on an unrestricted rerun.
No Rust implementation changed in this follow-up, and no additional cargo-mutants run
or mutation-coverage claim is made for the JavaScript launcher. macOS/ARM64 installed
runtime checks and hosted release execution remain unqualified locally. No release or
Marketplace publication was performed.

## Editor tabs follow-up, 2026-11-13

The 0.2.1 extension opens the workspace in a retained editor webview by default, with
an explicit external-browser command. The first real-frame check exposed the native
page policy's `frame-ancestors 'none'`. A failing HTTP regression preceded the change
to permit the desktop editor's `vscode-webview:` and `vscode-file:` ancestor schemes;
HTTP/HTTPS page ancestors remain excluded.

- Native formatting, workspace tests/doctests and Clippy passed: 758 tests passed,
  with 15 optional integration tests explicitly ignored.
- All 23 Node command/launcher/provider tests and 13 release-helper tests passed.
- Both source and installed Linux x86-64 extension checks rendered the real iframe,
  followed a semantic call, renamed a reading task and restored its two source bubbles
  after closing and reopening the tab. API checks covered tab reuse, source-tab
  switching, the explicit browser handoff, project changes and shutdown.
- The installed 0.2.1 payload, native executable and checksum matched the final source.
  All 49 checked documentation links and the TOML examples remained valid.
- A focused cargo-mutants run caught one replacement of the affected `router` body
  after a passing unmodified baseline. It used `nice -n 10`, one mutant/compiler task
  and serial tests. There were no survivors, unviable builds or timeouts in this run.
  This narrow function-level mutation does not establish coverage of individual CSP
  source expressions or the JavaScript lifecycle.

Linux extension-host checks now use Xvfb by default; `--visible` is an explicit opt-in.
An earlier visible test window displayed VS Code's temporary disabled-extensions banner;
this affected the isolated test profile, not the maintainer's normal extension settings.
macOS/ARM64 runtime and operating-system browser handoff remain outside this local check.

## Automatic connection messaging follow-up, 2026-11-13

The 0.2.2 extension separates its internal sharing step from the advanced external-launch
command. Default editor/browser launches pass the current connection directly and do not
print TOML copy/paste instructions. Advanced sharing is labelled in the Command Palette
and points to Open Workspace for automatic setup.

Two regression tests first failed on the obsolete prompt. All 25 Node tests now pass,
including stopping and starting without a project config, a stable connection filename
and automatic handoff of each new bridge. The real VS Code check on Xvfb also stops and
restarts the native workspace without changing its TOML file, then verifies that an
independently launched native process reconnects through its original connection path.
Real tab rendering, call navigation and saved-task restoration remain qualified on Linux.
The 13 release-helper tests, formatting and diff checks passed; the 0.2.2 packaged payload
was verified against source by the packager. No Rust implementation changed in this
follow-up, so the prior native baseline and mutation evidence are unchanged.

## Shared Rust dependencies follow-up, 2026-11-13

The 0.3.0 preview admits local Rust dependency and standard-library definitions through
VS Code's existing providers. A failing native peer test first reproduced discarded
external Rust locations, and Node regressions reproduced the missing external query
methods. The installed `Option::ok_or_else` additionally requires a checked document-symbol
range because the released native grammar cannot extract its `[const]` signature.

| Check | Result |
| --- | --- |
| Formatting, workspace tests/doctests and Clippy | Passed: 764 tests passed, 15 optional integrations ignored |
| Node protocol/provider/launcher/command tests | 29 passed, including both dependency query methods, saved-source witnesses and nonregular source refusal |
| Release helper tests | 13 passed |
| Source and installed VSIX, ordinary editor workflow | Passed on virtual displays |
| Real VS Code rust-analyzer 0.3.3113 | Resolved `ok_or_else` into installed Rust 1.99.1 `core/src/option.rs`; native capture retained the exact method body |
| Real editor UI and persistence | Followed `convert` to `ok_or_else`, saved the two-bubble task and restored it after closing and reopening the tab |

The real Rust check copies the installed analyzer extension into an isolated profile.
It selects local sysroot source, sets offline Cargo, and disables checks, build scripts
and proc macros. Neither the maintainer's profile nor their language-server configuration
was changed. The final installed VSIX passed both ordinary and real Rust checks.

The first full native run exposed noncanonical paths in four new peer fixtures when
TMPDIR used a symlink. The fixtures now publish canonical dependency paths as required
by the protocol; the baseline includes the symlinked temporary-directory check.

Mutation runs used cargo-mutants 27.1.0, `nice -n 10`, `CARGO_BUILD_JOBS=1`,
`RUST_TEST_THREADS=1`, `-j 1 --jobserver-tasks 1`, and passing unmodified baselines.

| Pass | Scope | Caught by failing tests | Survived | Unviable builds | Timeouts |
| --- | --- | --- | --- | --- | --- |
| Initial | 37 candidates in dependency admission, range validation, capture and provider methods | 21 | 10 | 6 | 0 |
| Final | All 18 candidates in the same function scope after consolidation | 12 | 0 | 6 | 0 |

Survivors exposed validation duplicated by `Document::capture`. Content, range and
identifier-witness checks now use that shared invariant boundary. Separate tests enforce
the provider descriptor's bounded, single-line name. The bridge also delegates file size
and regular-file checks to the existing capture reader. No mutation exclusions were
added. These overlapping runs are not a unique-mutant total or full-module coverage;
compiler failures are separate from mutations caught by tests.

Native call analysis inside provider-range fallback functions can remain unavailable;
the exact source body is readable, and no call facts are fabricated. Other external
languages, virtual sources, other platforms/analyzer versions and performance remain
outside this qualification. No additional server, download, build or publication was
performed by the product workflow.

## Editor context-menu handoff, 2026-11-13

The preceding preview work was committed as `7369d63` before implementing this slice.
The 0.4.0 preview adds **Vizzi: Explore Call** to the source editor context menu and
Command Palette. Saved source and a UTF-16 word position bind to an exact captured call;
the tab receives only opaque snapshot/call IDs and uses ordinary call following.

| Check | Result |
| --- | --- |
| Formatting, workspace tests/doctests and Clippy | Passed: 766 tests passed, 15 optional integrations ignored |
| Extension and handoff Node tests | 35 passed |
| Release-helper tests | 13 passed |
| Browser regression suite | `npm test -- --no-build --no-desktop-bus` passed, including the new editor-link tests |
| Actual right-click menu | Selected Explore Call on `ok_or_else` in the real Rust editor; caller and captured `core` definition opened automatically |
| Repeat navigation | Preserved the same inner document, both bubbles and their geometry; named task restored after tab recreation |
| Installed 0.4.0 Linux x86-64 VSIX | Ordinary sharing and real analyzer menu/repeat workflows passed in temporary profiles on virtual displays |

The API regression covers exact position selection after an emoji, mismatched text,
unaddressable paths, non-call positions, stale IDs, capture refresh and exact 4 MiB source
through the HTTP route. A retained hidden tab initially exposed a zero-viewport reveal
race; delivery now waits for a usable viewport. Existing model semantics retain distinct
semantic contexts as separate evidence; repeat checks preserve that behaviour rather
than collapsing it. Current bridge ownership is checked before reusing a session.

The UI harness captures the painted menu before selecting it and publishes its JSON
signals atomically. A transient partial-signal read in the initial installed test was
fixed in the harness. The final installed right-click and repeat checks passed with
real rust-analyzer 0.3.3113 and local Rust 1.99.1 source, without changing the maintainer's
profile or language configuration.

Both cargo-mutants passes used passing baselines, `nice -n 10`, one mutant/compiler task
and serial tests. The initial complete pass tested 20 candidates: 16 caught, 3 survived,
1 unviable build and 0 timeouts. The surviving cases prompted delegation of path authority
to published snapshot keys and a capture-refresh regression that detects stale index reuse.
The final complete pass tested all 16 candidates in the handoff module: 15 caught,
0 survivors, 1 unviable build and 0 timeouts. No mutation exclusions were added. Counts
overlap across passes; this is module-scoped Rust evidence, not full-stack mutation coverage.

The action starts from saved captured project declarations. Top-level calls without an
enclosing declaration, source-file aliases and unsupported syntax remain explicit errors.
Other platforms and performance are outside this local qualification. The installer was
built locally; no publication or update of the maintainer's installed extension was performed.

## Silent editor navigation, 2026-11-13

The 0.4.1 preview removes successful editor-handoff notices from both the Call workspace
and Saved locations, including standalone launches. The direct-editor regression first
failed on the visible success banner. After removal, the direct-editor and companion
browser checks passed, verifying silent success, literal recorded arguments, unchanged
saved evidence and visible stale-source refusal. Formatting and diff checks passed.
No Rust implementation changed; no additional mutation coverage is claimed.

## VS Code revision comparisons, 2026-11-13

The 0.5.0 preview adds Source Control overflow and Command Palette actions for two
local revisions and saved working changes against HEAD. Selections are resolved to
immutable commits before launch. The existing native comparison service and review
event stream supply capture, errors, file inventory and source panes. The review form
now follows the displayed capture's base, target and scope.

- Native baseline: formatting, workspace tests/doctests and Clippy passed; 766 tests
  passed and 15 optional integrations were ignored. No Rust implementation changed.
- Extension suite: 49 tests passed, including picker direction/pinning/cancellation,
  Git discovery, detached repository wrapper identity, subfolder launch roots, working
  files without buffer saves and comparison navigation during ordinary startup.
- Full browser suite: `npm test -- --no-build --no-desktop-bus` passed, including
  review, refresh and call workspace checks. Four older editor-handoff tests still
  waited for the success notice removed in 0.4.1; they now check the HTTP result and
  recorded editor arguments, with silent success or retention of an earlier save notice.
- Release-helper suite: 13 tests passed. The Linux x86-64 VSIX was built with the
  matching release executable and installed successfully with VS Code's installer
  into temporary user and extension directories; the executable retained mode 0755.
- Real VS Code 1.146.0: the installed package's Source Control menu opened both
  comparison workflows. Two Quick Picks selected the first and second fixture commits;
  native metadata, the two-commit form, captured target source and changed-file inventory
  matched those selections. Working review included staged, unstaged and non-ignored
  untracked paths. A review-only launch did not start live call indexing.
- The installed package also passed ordinary shared definitions/references, reconnect,
  explicit browser handoff, editor-tab reuse, call following, task restoration and shutdown.

Real UI checks use temporary profiles and virtual displays. The menu test activates
actions while a source tab is visible, avoiding Chromium input routing through the
embedded webview. The built-in Git API returns fresh repository wrappers, so repository
matching uses their root rather than JavaScript object identity. A failing regression
first reproduced that discrepancy; another reproduced comparison navigation lost to
an overlapping ordinary launch. Initial manual ZIP extraction did not restore Unix
executable bits; package qualification uses the editor's actual installer.

VS Code's graph context menus still require proposed APIs. The supported menu and Git
API approach needs no additional runtime dependencies; the minimum editor version is
1.100 because its public Git log options support pagination. That minimum was checked
against the published API source, but only the installed 1.146.0 host was run locally.
No new Rust mutation, other-platform or performance qualification is claimed.

## Open editor files in the source browser, 2026-11-13

The 0.6.0 preview adds **Vizzi: Open in Source Browser** to the editor and editor-tab
context menus and Command Palette. The action captures the selected URI and available
cursor line before focus changes. It checks saved bytes, resolves the existing captured
file, and checks the pinned source response's snapshot, file, version, path and text.
The tab receives opaque identities and a line, then uses ordinary source navigation
and the inspector's windowed line reveal. The initial link does not record an unrelated
default file; later links retain the document and Back/Forward reading positions.

- Formatting, native workspace tests/doctests and Clippy passed: 766 tests passed and
  15 optional integrations were ignored. The only Rust change serves a static JS asset;
  there is no new native source-resolution API or additional Rust mutation qualification.
- All 58 extension tests and 20 focused browser link/navigation tests passed. Cases
  cover inactive-tab resources, visible cursor positions, dirty/divergent source,
  workspace aliases, rejected file aliases, encoded path characters, mismatched captured
  metadata/text, retained native diagnostics and source changes during navigation.
- The full browser suite, `npm test -- --no-build --no-desktop-bus`, passed after the
  final initial-link/history changes. Release-helper tests also passed (13 tests).
- Real VS Code 1.146.0 checks passed for the editor context menu at a comment outside
  any function, the inactive file-tab menu, cursor-line reveal, retained inner document,
  Back/Forward viewport restoration and reopening from the Call workspace. The editor
  test focuses the source text before right-clicking so Chromium routes native input
  and menu contexts to that editor.

The first integration exposed an incorrect raw-byte assumption about `/api/source`;
the adapter now consumes and checks its existing schema-2 JSON response. A corrected
peer fixture first reproduced the failure. History tests cover replacing the fragment's
browser entry with the accepted visit and rejecting changed anchors before publishing
a view. The 0.6.0 Linux x86-64 VSIX includes the matching native executable, guide and
new adapter, and installs through the official editor CLI with executable mode 0755.
Installation and UI checks use temporary profiles and virtual displays.

The installed VSIX passed the same source-browser menu/cursor/history workflow and
ordinary shared definitions/references, reconnect, explicit browser handoff, call
following and saved-task restoration. Parallel editor UI runs intermittently missed
menu input; the installed source-browser qualification passed when run on its own.
The installed revision-comparison menu/picker/review regression also passed when run
on its own.

Files must be saved local project files admitted by the current capture. No dependency
download, buffer save, new language server or arbitrary browser path selection is added.
Other platforms, minimum-version editor execution and performance remain unqualified.

## Native captured-source handoff, 2026-11-13

The 0.7.0 preview moves call-trail and source-browser entry behind one native module.
The [handoff contract](../configuration/HANDOFF.md) covers the three finite exchanges,
window ordering, resource bounds and refusal states. Editor-specific observations stay
in `editor-source.cjs`; `handoff.cjs` supplies one generic result loop. The two previous
per-action capture/status/source-response implementations and their mocked resolution
tests were removed. Saved bytes are checked at observation and before presentation;
pending exchanges check the original buffer without repeatedly rereading large files.

- Formatting, workspace tests/doctests and Clippy passed: 785 tests passed and 15
  optional integrations were ignored. Tokio's test-only clock feature supports exact
  expiry tests; no runtime dependency was added.
- All 62 editor tests passed, including real HTTP/stdio equivalence without VS Code,
  late-result rejection, native cancellation, expired-scope reattachment, outdated
  executable errors and overlapping editor actions during native launch.
- Native behaviour tests cover original-text matching, maximum-size saved source,
  scoped refresh coalescing for newly saved files, out-of-scope admission, fixed
  deadlines, sequence boundaries, unknown continuations, cancellation, owner shutdown,
  capture failures and a publication change while binding waits for compute capacity.
- The native handoff uses one Tokio time source for lease/deadline checks and waits.
  Paused-clock regressions first failed for exact scope and worker-start expiry under
  the original mixed-clock implementation, then passed after the correction.
- The complete browser suite passed against the new interface before the final clock
  and admission refinements; it was rerun against the final packaged native build.
  Release-helper tests passed (13 tests), and VSIX payload/guide/executable bytes and
  SHA-256 checksum were verified.

Mutation checks used `nice -n 10`, `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`,
`-j 1 --jobserver-tasks 1` and passing unmodified baselines. The initial run was
interrupted after exposing two missed source-size mutations and one test timeout;
that incomplete run is not a completed qualification. A complete handoff-module pass
then tested 97 mutants: 62 caught, 12 missed, 23 unviable builds and no timeouts.
The missed cases prompted stronger sequence, cancellation, path, owner-lifetime and
publication-race tests. The redundant byte-length predicate was removed because native
content identity already binds the length. Exact expiry received deterministic clock
coverage. The follow-up tested all surviving mutation classes and related conditions:
24 mutants, 21 caught, no survivors or timeouts and 3 unviable builds. Those three
replacements create unsupported `||` let-chain conditions. No mutation exclusions were
added. Counts overlap; the follow-up is targeted, and these runs do not establish
whole-codebase mutation coverage.

Real source-browser menus/cursor/history, real rust-analyzer standard-library call
exploration/repeat navigation and Source Control comparisons passed during development.
The 0.7.0 Linux x86-64 VSIX includes the matching native executable and installs through
the official editor CLI with executable mode 0755. Editor checks use temporary profiles
and virtual displays. Other editor runtimes, other platforms and performance remain
unqualified; the portable contract is exercised through both existing native transports.

The installed 0.7.0 payload passed all four checks: source/tab context menus with cursor
and Back/Forward retention; actual rust-analyzer `ok_or_else` exploration and repeated
navigation; ordinary shared definitions/references, reconnect, browser handoff and task
restoration; and Source Control working/revision comparisons. The UI harness paints text
focus before right-clicking and retries only stale menu opening/input. Comparison retry
checks inspect native capture or picker state first, avoiding duplicate actions. The
final native build also passed the complete browser suite and 62 editor tests.

## Presentation and startup review fixes, 2026-11-13

The 0.7.1 preview fixes both editor races found in review. Deterministic regressions
first reproduced edits to buffers and saved files after native resolution while webview
readiness was delayed, plus a buffer edit during the saved-source read itself. Delivery
now validates after readiness and immediately before posting, rechecking cancellation
and panel identity after that asynchronous validation. Buffer checks bracket the read.

Startup/provider-sharing failures propagate to the owning command. Source commands
report only for their current intent; a queued newer open proceeds after an older
failure. Shared failures are reported once for ordinary coalesced opens. Tests cover
obsolete failing startup followed by successful newer navigation, an active failure,
same-startup source intents and normal shared-failure reporting.

All 72 editor tests and 13 release-helper tests passed. The 0.7.1 VSIX was packaged
with the unchanged verified native binary, installed through VS Code's CLI in temporary
directories, and its checksum was verified. These fixes change editor glue only;
no additional Rust, native mutation or standalone browser qualification is claimed.

The installed package passed source/tab menus and browsing history, real rust-analyzer
call exploration/repeated navigation, normal shared-provider launch/reuse/task restoration,
and Source Control comparisons. One initial Rust-analyzer preflight timed out resolving
`ok_or_else` through VS Code before any Vizzi action ran; its isolated retry passed.


## Originating-editor navigation (2026-11-13)

Plugin launches now bind return navigation to their exact editor window through a
separate per-launch callback. Native source admission and standalone CLI behavior remain
covered by the existing tests. SHA-256 uses sha2 0.11.1, verified as the current stable
release before adding it; Node uses built-in crypto with no runtime npm dependency.

- Native baseline: formatting, all-feature workspace tests and doctests passed
  (793 passed, 15 explicitly ignored), followed by warning-free Clippy. Final focused
  checks cover eight origin-adapter tests and twenty companion tests, including a
  project file larger than the semantic provider allowance.
- All 98 Node editor tests passed. Cases include independent same-root windows,
  descriptor replacement, lost origin, authentication and frame limits, target-only
  dirty checks, aliases, edits during asynchronous presentation/reads, BOM handling,
  external files, expiration, disconnect cancellation and process-owned cleanup.
- The complete browser regression suite and Python/Rust/Ruby/C++ documentation
  walkthroughs passed. Browser cases verify adapter-supplied names in bubbles and
  saved locations, plus generic labels when investigation metadata is unavailable.
- VSCE 4.1.0 produced `dist/vizzi-vscode-0.8.0-linux-x64.vsix`. Runtime source,
  native executable, documentation and license bytes were checked against the package;
  its SHA-256 sidecar passed verification. Official VS Code CLI installation used only
  a temporary profile. The executable retained its executable mode.
- Installed VS Code checks passed on Linux, including an aliased workspace, the real
  embedded workspace, tab closure, Stop Sharing, explicit external-browser handoff,
  dirty-target refusal, and rust-analyzer `ok_or_else` exploration followed by return
  to the captured standard-library line. The external-browser launch itself is
  intercepted in the isolated host; HTTP callback delivery and Chromium rendering
  are checked separately.

Cold rust-analyzer startup intermittently failed to resolve the fixture before Vizzi
launched; successful runs exercised the complete installed workflow. The harness now
selects the known installed analyzer binary, explicitly starts it, limits its threads,
and reports analyzer state on failure. Same-root window isolation is qualified with
deterministic TCP peers rather than two simultaneous VS Code GUI instances. These
checks do not qualify other operating systems or every installed language extension.

### Origin adapter mutation evidence

All passes used `nice -n 10`, `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`,
`-j 1 --jobserver-tasks 1`, and a passing unmodified baseline. Scope covers the whole
new native origin adapter and `SourceWitness::from_bytes`, rather than changed lines.

| Pass | Completion | Caught by failing tests | Survived | Unviable | Timeouts |
| --- | --- | --- | --- | --- | --- |
| Initial adapter pass | Interrupted after 10 of 91 candidates | 7 | 0 | 1 | 2 |
| Bounded-peer continuation | All 84 remaining candidates; iteration excluded 8 earlier caught/unviable outcomes | 77 | 4 | 3 | 0 |
| Strengthened-test rerun | Both behavior-changing survivors | 2 | 0 | 0 | 0 |

The initial timeouts came from a test peer waiting for a connection that a mutated
request rejected before connecting. Bounding peer acceptance converted both into
ordinary failing-test evidence on the continuation. Survivors prompted regular-file
refusal diagnostics and invalid oversized fields on otherwise successful replies.
Across the passes, 92 distinct candidates produced 86 caught, four unviable and two
reviewed equivalent outcomes. Compiler failures and earlier timeouts are not counted
as tests catching mutations.

The two narrow equivalent cases change the serialized request frame preflight in
`OriginEditor::open` from `> FRAME` to `== FRAME` or `>= FRAME`. A validated descriptor
is at most 4096 bytes and the destination path at most 4096 bytes; even maximal JSON
escaping keeps this fixed-shape request below 32 KiB. Neither equality can be reached
at the 64 KiB frame limit. The defensive check remains. No function-wide mutation
exclusion was added. Mutation evidence does not prove correctness or cover all Rust
and JavaScript paths.
