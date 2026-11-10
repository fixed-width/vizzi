# Module cue validation

Contributor reference, 2026-11-10. This records the first native module-label/accent
implementation and optional Bubble Sets overlay. Behaviour and architecture are in
[MODULES_PRODUCT](../../docs/visual/MODULES_PRODUCT.md) and
[MODULES_TECH](../../docs/visual/MODULES_TECH.md).

## Verified behaviour

Rust Cargo roots, inline/external declarations, explicit paths and literal module-scope
includes establish captured module membership. Python modules use explicit package
chains. Types do not create modules. Ambiguous or unavailable membership stays neutral.
Tests cover manifest/line/body edits, cache versus clean construction, target defaults,
invalid paths/names, extraction bounds, and conservative tracking across context upgrades.
Native definition preparation reads the captured Cargo manifest after working-file edits
or deletion, keeping its inputs coherent with module identity.

The browser checks shared labels/accents/minimap colours, collision avoidance in small
sets, hover and keyboard peers, distinct call highlighting, unchanged camera/layout,
optional contours, manifest updates, pin/follow, legacy layouts and reload. SVG tests
check members, nonmembers, points near corners and edge midpoints across orientations,
offsets and world scales. A fake-worker regression covers lazy startup, coalescing,
stale replies, terminated worker isolation and disabling.

An actual pre-module executable was replaced by the current release on one browser
origin with the same source/cache: two live bubbles retained their IDs, positions,
dimensions and camera; module metadata hydrated, their relationship was reverified,
and navigation remained usable. This checks live upgrade recovery, not indefinite
retention of pinned captures across host restarts.

## Checks

- Formatting and workspace Clippy with warnings denied pass.
- Workspace tests, including doctests: 628 passed across 89 result summaries.
- Full browser suite passes; focused module/keyboard/geometry checks pass after later
  test strengthening. No production changes followed the full suite.
- Native module/context fixtures and the captured-manifest regression pass.
- No inspected repository build, language-server or editor launch occurs in these fixtures.

Compressed logs and mutation reports are in [results/modules/validation](results/modules/validation).
The normal runner includes the new browser checks. See the
[browser procedure](../browser/README.md#check-module-cues) for focused commands.

## Mutation evidence

All runs establish a passing unmodified baseline. Rust runs use `CARGO_BUILD_JOBS=1`,
`RUST_TEST_THREADS=1`, `nice -n 10`, `-j 1 --jobserver-tasks 1`. Stryker uses one runner
at lower priority. Mutation runs are serial.

| Native scope | Caught by tests | Survived | Build failures |
| --- | ---: | ---: | ---: |
| Entire initial modules.rs | 86 | 23 | 2 |
| Initial survivor families after stronger tests/style fixes | 33 | 0 | 6 |
| Added policy/context projection lines | 5 | 1 | 3 |
| Projection count predicate family after stronger fixture | 2 | 0 | 0 |

The final module-family run selects 39 mutations, including additional occurrences of
the same operators. Its six unviable mutations introduce invalid `|| let` chains after
Rust 2024 style fixes; they are not caught tests. Initial build failures instantiate
unimplemented Default values. The count survivor originally escaped because the fixture
had equal source/metadata counts; using unequal counts catches it. No native timeouts.
These runs overlap and must not be added as unique coverage. The full-file initial run
plus focused follow-ups is not a fresh exhaustive final-file mutation run.

JavaScript uses Stryker 10.1.1 over module-identity.js and module-contours.js, excluding
the pinned vendor library. Initial Node checks caught 247, timed out on one, and left
135 survivors. A browser activation control exposed that the first SVG mutation pass
did not propagate the selected mutation into Chromium. The harness now passes the
active reference before loading instrumented scripts; a real instrumented comparison
passes unmodified and fails under its selected mutation. Intermediate reports retain
that limitation explicitly. See [js-final.json.gz](results/modules/validation/js-final.json.gz) and
[JS_MUTATION_LIMITS.md](results/modules/JS_MUTATION_LIMITS.md) for final
counts, investigated survivors and coverage limits. Timeouts are separate from caught
mutations. No exclusions improve the reported score.

## Real-source replay

[replay-modules.mjs](replay-modules.mjs) copies the Tract workspace/minitosis manifests,
lib.rs and engine.rs into a temporary directory. Four bubbles form two module groups,
with two optional contours, distinct colours, unchanged positions and zero contour jobs
for camera-only changes. The final run occurs after agent mutation/compilation jobs end.

[replay.json](results/modules/replay.json) records source/binary hashes and the worker
round trip: about 22 ms including startup/scheduling in one local run. This is a
functional/performance probe, not baseline-laptop, whole-repository, tail-latency or
independent usability qualification. Screenshots remain temporary artifacts rather than
checked-in binaries.

Membership is bounded resident-source metadata. Unsupported languages, absent Cargo
roots, unsupported declarations, conflicts and exhausted limits can remain neutral.
Raw syntax is not configured compilation, runtime/import resolution or macro expansion.
Contours are approximate presentation, can be omitted, and are independently bounded
by a one-second worker lifetime. This does not establish complete geometry/resource
coverage or large-codebase rendering performance.

Dependencies were checked against published stable releases: toml 1.2.1+spec-1.1.0 and
bubblesets 1.0.2. The vendored ESM and licence match the installed pinned package;
[vendor notes](../../crates/vizzi/web/vendor/README.md) record provenance and checksum.
Generated debug artifacts (about 3 GiB) were removed after native checks; release and
installed development dependencies remain.
