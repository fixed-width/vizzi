# Local dependency navigation validation

Contributor reference for the Rust-first dependency capture implementation, 2026-11-10.
The [behaviour contract](../../docs/visual/DEPENDENCIES_PRODUCT.md) and
[architecture](../../docs/visual/DEPENDENCIES_TECH.md) define its scope.

## Behaviour checked

- A repository call opens a function from a local path dependency. A further call opens
  a second dependency file, retaining the caller, zoom and layout.
- A cached registry package (`blake3`, using Vizzi's locked version) and installed standard
  library source (`core::ptr::null`) resolve into independent native captures.
- Root/dependency files with the same relative path keep distinct identities. Origins
  carry package/version when known; physical paths remain native.
- Captured source remains readable after disk edits. Semantic use/editor handoff verifies
  disk bytes; changed source is refused. Source references restore after browser reload
  and host restart only while the matching capture/source remains available.
- Closing/reopening source that has become available again recovers the bubble. A capture
  pinned to an earlier project observation stays separate from a current call trail.
- Local Cargo path references survive the private mirror, including dependency, dev/build,
  workspace, target-specific, patch and replacement tables. No fixture is built or fetched.
- Explicit sysroot source configuration avoids rust-analyzer's automatic rust-src installation
  branch. Child toolchain auto-installation is disabled. The protocol peer checks the settings.

The browser regression first exposed transient admission failures on reload. Separate bounded
dependency reads and bounded retries for read-only requests fix the lost-source panes;
editor/writing requests never retry. Work permits stay with the blocking operation if the
HTTP request is cancelled. All source/file access is through native references, not a browser path.

## Correctness checks

Formatting, workspace Clippy with warnings denied, full browser checks and the native workspace
suite pass. The recorded native workspace run has **641 passing tests**, six opt-in tests ignored,
and 90 successful result summaries, including doctests. All **six real rust-analyzer integration
tests** also pass explicitly with installed prerequisites. Focused source-origin, retry, recovery,
capture-budget and path-table tests were strengthened after the broader runs; no production
change followed the shipped browser/native checks.

[Compressed evidence](results/dependencies/validation) includes the failing regression cases,
passing native/browser/protocol checks and mutation outcomes. The real browser test is opt-in:
`npm test -- --dependencies`; see [the browser procedure](../browser/README.md#check-dependency-navigation).
It creates temporary source/cache directories and never launches VS Code. Native editor tests
verify the returned location and changed-file refusal rather than opening the desktop editor.

## Native mutation checks

Runs use one mutant/compiler job, serial tests and lower priority:
`CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo mutants -j 1 --jobserver-tasks 1`.
Passing unmodified baselines precede interpretation. After 82 completed cases, the initial
run was interrupted to remove build debug symbols; that partial run is retained separately.
An iterate run excludes its 67 caught/unviable cases and completes the remaining 56 cases.
Stronger boundary/table-kind fixtures then rerun all 27 survivors. Results overlap and are
combined by exact mutation identity in [native-summary.json](results/dependencies/native-summary.json).
The cumulative two-file scope is all 123 generated cases in dependency_sources.rs and
rust_dependencies.rs:

| Outcome | Count |
| --- | ---: |
| Caught by failing tests | 108 |
| Survived | 3 |
| Unviable/build failures | 12 |
| Timeouts | 0 |

The unviable cases are reported separately, never as test kills. This is cumulative evidence,
not a fresh final full-file run. Additional host/provider/HTTP adapters are covered by interface
and real-provider tests, not by an exhaustive mutation run.

The two path-length survivors change the >4,096-byte guard in acquisition; the exercised Linux
filesystem prevents constructing an admitted exact/over-boundary source for these changes.
They remain platform coverage limits, not proven equivalent across platforms. The CurDir match-arm
survivor is unreachable for the current host's canonical absolute roots and translated absolute
dependency paths: Path::components removes interior current-directory components. No source
exclusions suppress these raw outcomes.

## JavaScript mutation checks

The broad serial Stryker run covers all of source-origin.js, read-retry.js and visual-model.js:
1,169 cases, **1,079 assertion kills, 88 survivors and two timeouts**. Focused follow-up results
and remaining limitations are recorded in [JS_LIMITS.md](results/dependencies/JS_LIMITS.md).
The broad model run includes substantial existing behavior beyond this feature. There is no
coverage-analysis claim; DOM/worker code is checked through browser behavior rather than mutated.

## Limits

Rust is the first adapter. Other languages retain their previous navigation capabilities.
Local source is required; virtual destinations and unavailable/generated declarations may
remain outside. The stable Tree-sitter Rust grammar (0.24.2, verified with cargo info) does not
emit every recent declaration. In the installed standard library, core::cmp::max uses `[const]`
trait bounds: rust-analyzer resolves the file/position, but the native declaration index cannot
represent that function. It therefore stays unavailable instead of selecting a same-name function.
The passing standard-library test uses a supported declaration; it is not universal std coverage.

Dependency modules may be unknown; package provenance is not invented module membership.
Captures are not watched. The LRU retains at most 32 captures/32 MiB/32,768 declarations with
a 2 MiB per-file allowance. Restored references require matching analysis/context and bytes;
expiry on source or analyzer changes is explicit. These are cache allowances, not an RSS cap.
No baseline-laptop, large-project tail-latency or whole-repository performance qualification is
claimed by these functional checks. Repository indexing does not expand into dependency trees.
