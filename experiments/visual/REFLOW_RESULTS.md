# Source reflow validation

For maintainers assessing the source-pane formatting change, 2026-11-09. Smart reflow
uses native language/token hints and local font/width layout. It preserves captured source
coordinates, syntax colouring, call navigation and neighbouring fragments. The
[design explanation](../../docs/explanation/source-reflow.md) distinguishes the Code Bubbles
research from Vizzi's conservative lexical policy; the [usage guide](../../docs/how-to/follow-a-call-trail.md)
describes Smart reflow and Original lines.

## Correctness

Formatting, workspace tests/doctests and Clippy with warnings denied pass: **584 tests
passed, 3 opt-in real-provider tests ignored** by the normal workspace command. Focused
follow-ups strengthened the same native test set. The complete browser suite passes.

Regression-first checks exercised:

- Rust, Python and C/C++ parser-derived boundaries; compound operators, literals, broad
  attributes, comment words, nested delimiters, qualified paths and incomplete syntax.
- Exclusive byte/line boundaries, Unicode, tabs, CRLF endings, opt-in projection, unavailable
  or limited analysis, clipping and per-line/window hint allowances.
- Width-dependent segment choice, bounded hanging indentation, widening, indivisible tokens,
  malformed metadata, unavailable font measurement and work/segment allowances.
- Exact captured-text copying through a synthetic clipboard event, including when focus
  moves outside the source pane. These checks do not qualify OS clipboard normalisation.
- Live inline call controls, selection-preserving deferred resize, source reading anchors,
  legacy pixel-state migration, original-mode persistence, light/dark themes and stale
  source rerendering. The existing horizontal-scroll check explicitly uses Original lines;
  the new tests cover Smart reflow and local resizing without source requests.

The first core/DOM/planner checks failed on missing interfaces. Further failing regressions
exposed nested literals hidden by broad styling, split compound operators, missing font
measurement and copying after focus moved. Correctness fixtures remain small and deterministic.

## Mutation evidence

All runs had passing unmodified baselines and used one mutant/compiler task, serial tests
and nice 10. The initial native scope included the complete new hint module plus changed
source-window lines: **108 mutants**. Its initial result was 62 caught, 16 surviving,
3 unviable and 27 timeouts. Assertions for break bounds, nesting, alignment and partial
qualified paths caught the functional survivors in targeted follow-ups. The final outcome
for those same 108 mutants is **77 caught, 3 unviable, 28 timeouts, 0 surviving**. Mutations
that stalled scanner progress were terminated by the harness. They are reported as timeouts,
not assertion catches; the unviable replacements did not compile.

The complete browser planner finished with **170 caught, 5 surviving, 0 timeouts/errors**.
The initial run had a timeout and more survivors. Tests were strengthened and the outer
loop was made explicitly bounded. Remaining survivors remove unobservable caught-error
messages, recompute deterministic font measurements instead of caching, replace the
missing-anchor fast path with an equivalent full-line measurement/fallback, or tighten the
measurement ceiling by one call within the stated limit. The cache/fast-path survivors
are functional equivalents; timing effects are not established by the unit tests.

The full visual-workspace model scope finished with **725 caught, 5 surviving, 0 timeouts/errors**.
Its remaining survivors are existing overlapping identity/context validations and equivalent
context forwarding after validation; no new reading-anchor validation survived. No mutations
were excluded. DOM/menu/copy event handlers have browser regression coverage rather than a
complete mutation scope. This evidence is not proof of correctness or universal language support.

## Real repository and layout cost

After other checks stopped, an optimised host with one parsing worker and isolated Chromium
opened the known `tract-plugin-pack` Minitosis process → render trail and GsMeter process.
The call opened directly with source-local evidence while retaining the caller. Resizing the
caller through widths 320, 448, 720 and 960 kept other views in place and made **zero source
requests**. No inspected source was modified; no repository build, editor or language server
was launched by this replay.

| Instrumented SourcePane layout update | Observation |
| --- | ---: |
| Samples | 64 |
| Median | 0.9 ms |
| 95th percentile | 2.4 ms |
| Maximum | 3.9 ms |
| Source requests during resizing | 0 |

These are one-session local observations, including synchronous SourcePane layout work,
not full-frame/GPU timing, an independent usability result or baseline-laptop qualification.
The first replay attempt stopped because its driver clicked an outdated search result;
it produced no latency result. The corrected driver waits for the current inventory and
pagination responses. That failed harness log is retained separately.

[Compressed logs, mutation outcomes, screenshots and replay metadata](results/reflow/checksums.json)
include the inspected source revision and release-binary hash. Reproduce in a new directory:

```sh
CARGO_BUILD_JOBS=1 cargo build --release --offline -p vizzi
nice -n 10 node experiments/visual/replay-reflow.mjs \
  /path/to/tract-plugin-pack /tmp/vizzi-reflow-replay
```
