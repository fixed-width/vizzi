# Visual call-trail prototype: validation record

For maintainers assessing the first spatial interaction, 2026-11-08. The
[usage guide](../../docs/how-to/follow-a-call-trail.md) covers the workflow; the
[product](../../docs/visual/PRODUCT.md) and [technical](../../docs/visual/TECH.md) contracts
separate requirements from this record.

The prototype opens captured function declarations concurrently on `/workspace`, with
candidate-call links, stable placement, local reading state, movement/resize/collapse,
camera/overview controls and browser-local restore. It reuses the Rust source engine,
structured metadata, styled windows, current-only keep and verified editor interfaces.
This is the first source-view slice; grouping and other representation/revision kinds
remain future work. Production rendering and independent usefulness remain unqualified.

## Real-source replay

The [recording](results/result.json) uses the unchanged Tract Plugin Pack revision
`951cf85413029423e036c641638b845edafe75cf`, with one analysis worker and an optimised native
host. No repository build, source edit, model or real desktop/editor launch occurs.
The [script](replay.mjs) deliberately selects known Minitosis candidates; this is a
functional replay, not an unseen question or human trial.

| Visible source step | Candidate declarations shown | Retained caller |
| --- | ---: | --- |
| `process → render` | 4 | geometry, DOM node and reading offsets |
| `render → Processor::process_sample` | 8 | geometry, DOM node and reading offsets |
| `render → Processor::begin_block` | 2 | geometry, DOM node and reading offsets |
| `Processor::process_sample → DryDelay::next` | 8 | geometry, DOM node and reading offsets |

Five panes and four links remain open together; reload restores the panes and scrolled
`render` source. No browser page errors were observed. The [light overview](results/working-set-light.png),
[dark overview](results/working-set-dark.png), [reading view](results/reading-dark.png) and
[source-bound presentation document](results/workspace.json) record that state. Eight is
the existing candidate page bound; it is not proof all possible destinations are shown.
The replay does not establish a complete runtime input/output path or resolve receiver types.

After other test/mutation processes stopped, the same optimised-host replay recorded
90 headless-browser frame intervals while panning five panes: median 16.7 ms, 95th percentile
16.8 ms, maximum 17.0 ms. These are single-session display intervals, not handler costs,
large-repository/24-pane qualification or a hardware-GPU benchmark. Native timings remain
separate from interaction; this result does not select the production renderer.

Reproduce with a new output directory after building the release host:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 node experiments/visual/replay.mjs /path/to/tract-plugin-pack /tmp/new-visual-replay
```

## Correctness and mutations

TDD first reproduced missing canvas/model behaviour. Focused tests cover unchanged
caller/branch geometry and reading offsets, candidate witnesses, non-overlapping placement,
revisits/cycles, all view/link allowances, camera controls, invalid/boundary persistence,
source-version guards, independent copies and rejected-operation atomicity. Browser tests
also exercise ambiguity, retained DOM/scroll, keyboard movement/resize, capture and
recording-only editor handoff, reload, stale actions, system light/dark themes and narrow
viewports. A delayed-reply regression first demonstrated an older source section replacing
the newer selection; the final controller keeps only the latest request's section.

Final checks passed:

- Formatting and Clippy for all workspace targets, with warnings denied.
- Workspace tests including doctests: 533 passed. [Native log](results/native-tests.log.gz).
- Complete browser suite including the new workspace and long-section checks.
  [Browser log](results/browser-tests.log.gz), [Clippy log](results/clippy.log.gz).
- Unmodified model baseline followed by Stryker: 643 mutants, **641 killed by failing tests**,
  **2 equivalent survivors**, 0 timeouts, 0 errors; no exclusions. One low-priority worker.
  [Raw report](results/model-mutations.json.gz).

The remaining survivors remove the leading anchor in the restored view-ID regex or the
link-ID string-type check. The following numeric view-ID bound rejects any additional `v`
in a prefix; the canonical link-key strict equality rejects non-string IDs. These checks
remain enabled in the source and raw report. The mutation configuration records the raw
score (99.69%) without turning a percentage threshold into correctness proof. Mutations
cover the presentation model, not every DOM/controller branch. No new Rust behaviour beyond
asset routing was introduced; cargo-mutants was not run on unchanged backend logic.

Run the model test with `node crates/vizzi/web/visual-model.test.js`, the focused browser
check with `npm test -- --visual`, and model mutations with:

```sh
nice -n 10 node_modules/.bin/node node_modules/@stryker-mutator/core/bin/stryker.js run experiments/browser/visual-stryker.config.json
```

## Build-output cleanup

Before implementation, ignored compiler output occupied approximately 38 GiB of a 39 GiB
checkout. `cargo clean` and removal of verified ignored mutation/Stryker scratch reduced the
checkout to about 508 MiB. Source, tracked research/results, installed dependencies and
private participant notes were preserved. An old runnable binary was temporarily kept for
the red browser test and removed after rebuilding.

After fresh release/test/Clippy builds, the checkout occupies about **3.1 GiB**: development
output about 2.3 GiB and release output 275 MiB. Default development/test builds now retain
line tables and disable incremental compiler caches. This trades variable debug information
and some recompilation speed for smaller output; source-index incrementality and release
measurements are unaffected. The [cleanup record](results/cleanup.json) and
[contributor guide](../../docs/how-to/keep-build-output-small.md) describe the scope.

The next decision is the maintainer's assessment of concurrent context, branching and
spatial arrangement. No additional model/search tuning is required to obtain that feedback.
