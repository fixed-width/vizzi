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
