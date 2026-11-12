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
