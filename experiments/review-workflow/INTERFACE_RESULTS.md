# Initial browser review implementation: validation record

The implementation at `/review` coordinates changed/unchanged file pages, hunk and
change-range pages, two captured-source inspectors and lexical declarations. It
exposes explicit baseline/scope, capture labels, refresh/close, independent review
notifications and bounded comparison-specific history. See the
[interface reference](../../docs/prototype/REVIEW_INTERFACE.md) and
[review how-to](../../docs/how-to/review-local-changes.md).

## Verified behaviour

The first controller test failed against its stub before implementation. Subsequent
regressions failed before fixes for discarded-history recovery, duplicate visits on
filter changes and selecting the available base side of removed files. Deterministic
Node tests cover exact anchors, atomic acceptance, stale same-pair selections,
notification replacement/closure, absent/unavailable source, resource refusal,
200-row bounds, independent hunk/edit paging, declaration/side separation, viewport
metadata, 50-visit bounds and browser-history marker policy.

The shared inspector supplies the existing virtualisation, keyboard and fragment
mechanisms. The review reader checks token/comparison/side before the shared
document-window validator. Existing live window/controller/navigation tests remain
part of the Node regression suite. Native workspace tests, formatting and Clippy
are checked separately.

Final checks pass **88 Node tests** and **397 native workspace tests**, including
doctests, plus formatting and workspace Clippy with all targets/features and denied
warnings. These counts exclude the blocked Chromium tests.

The [two-variant controller replay](results/interface-fixtures.json) uses the normal
release executable's finite diagnostic adapter and the actual browser controller.
It checks the two changed paths and three/two expected Rust hunks, reads the changed
lookup policy, visits unchanged reader/path contracts and the existing regression
test, returns to an exact review anchor, refreshes and refuses an expired visit.
Returned source fragments are independently checked against pinned/working bytes.
The defective variant contains the broadened permission-error fallback; the control
does not. Both disposable fixtures reverify after exit. No analysed-project tooling
or build runs.

Reproduce the controller/native replay after preparing the disposable fixtures:

```sh
CARGO_BUILD_JOBS=1 cargo build --release --offline -p vizzi
node_modules/node/bin/node experiments/review-workflow/verify_interface.mjs \
  /tmp/vizzi-review-a /tmp/vizzi-interface-a
node_modules/node/bin/node experiments/review-workflow/verify_interface.mjs \
  /tmp/vizzi-review-b /tmp/vizzi-interface-b
```

Use fresh output directories outside the fixture workspaces. The replay reports its
scope explicitly: finite status probes synchronise the controller, without pretending
to exercise SSE wire delivery or a DOM.

## Mutation evidence

The [mutation record](results/interface-mutation.json) preserves scopes, source/test
fingerprints, baselines and outcome categories. Runs use one serial Node process at
lower priority. Stryker's normal launcher could not create its log socket in this
environment. The fallback uses the installed Stryker 10.1.1 instrumenter directly,
with no network sockets, an unmodified baseline, an instrumented baseline and a
known-mutant activation control. Test subprocesses use regular log files.

| Scope | Cases | Detected after follow-up | Reviewed equivalent survivors | Timeouts |
| --- | ---: | ---: | ---: | ---: |
| Complete new controller and review reader | 719 | 709 | 10 | 0 |
| Complete shared window validator/reader and notification adapter | 333 | 332 | 0 | 1 |

The final-source rollups use full-module runs plus targeted follow-ups on matching
production hashes; earlier development runs overlap and are not added. Each of the
ten remaining survivors has an individual invariant-based assessment in the JSON.
They remain reported as survivors, with no exclusions or claim that assertions caught
them. The shared timeout already logged an assertion failure but kept retry timers
alive; it is still a timeout outcome, not an additional detected case.

Full new controller/reader runs and survivor follow-ups exposed real history/busy-state bugs
and prompted boundary, source-binding, paging, cancellation and viewport assertions.
Survivors, timeouts, syntax failures and incomplete/blocked checks are recorded
separately; this does not establish complete coverage or a release qualification.
The instrumented baseline parses the whole switched program before individual
mutations run. A `SyntaxError` produced by mutated runtime code is a failed test,
rather than a compilation failure; the record notes a corrected initial classification.

The fallback can be run with:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  node_modules/node/bin/node experiments/browser/offline-mutations.mjs \
  experiments/browser/review-stryker.config.json /tmp/vizzi-review-mutation.json
```

## Browser acceptance still open

The real Chromium integration test is checked in at
[review.test.mjs](../browser/review.test.mjs). Both the host's loopback listener and
Chromium startup failed with `Operation not permitted` in this sandbox. The environment
also rejected escalation because sandbox approvals are disabled. No browser success,
screenshot, keyboard/layout check or rendered good/bad walkthrough is claimed.

Run the pending browser checks in an environment permitting a local test listener
and Chromium:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 npm test -- --review
```

Next, repeat the two-variant task through rendered controls, inspect the resulting
screenshots, and retain the learning walkthrough as a regression. Measure the actual
new views' payload/DOM bounds and interaction only after those browser checks pass.
Controller replay is guided/reference-aware evidence, not independent-operator
acceptance, human comprehension improvement or large-repository latency evidence.
