# Second maintainer feedback: orientation and appearance

This record is for contributors assessing changes after `24d48df`. The
[behaviour](../../docs/prototype/ORIENTATION_PRODUCT.md) and
[implementation](../../docs/prototype/ORIENTATION_TECH.md) contracts define the result;
[the layout explanation](../../docs/explanation/repository-layouts.md) separates
published evidence from this project's design hypotheses.

## Result

The reported revision failures came from the disposable trial's single-commit shallow
history, not a separate revision parser. Git itself refused those parent expressions
there. The trial copy now contains 3,885 reachable commits from local objects. Its HEAD,
index bytes, working files and task document were preserved, and the original checkout
was unchanged. `--history` makes this an explicit option for future maintainer fixtures;
default scripted fixtures remain minimal. Application capture never fetches. Native and
browser regressions cover `HEAD~1`, `HEAD^^`, unique short hashes, both comparison sides
and useful errors for missing shallow history.

Informational/error message rows now have labelled close controls. Dismissal is local
presentation state: the same message/context stays closed, while a new context/message
can appear. Source, comparison and recovery semantics are unchanged. Empty availability
notes and map help are dismissible too; factual counters and disclosure contents remain
part of their respective views.

System is the CSS theme default. Light/Dark overrides persist when storage is available;
valid overrides are restored before paint. Media changes and other-tab preferences update
the view. The canvas reads the same palette tokens. Both palettes give syntax categories
stronger hues and type/keyword weights; code uses 12-pixel text with the existing 22-pixel
line metric. Measured foreground contrast passes 4.5:1 on plain, selected, added and
removed code backgrounds in Chromium.

The browser map now shows ordered nested directory frames and a virtualised directory
chooser. Collapse, focus/breadcrumbs, sibling dragging, buttons, keyboard reordering,
workspace-scoped preferences and reset operate on one scene. Compact paths retain actual
containment. Depth is bounded, while small cells retain file rectangles for zooming.
Native rectangles remain a compatible diagnostic projection. Metadata-only updates reuse
geometry/tree references. Refused file admission is atomic. Auto-fit tracks resize;
manually moved cameras retain their world centre.

## Verification and its limits

Formatting, workspace tests including doctests, and all-target/all-feature Clippy pass.
The native suite has **424** passing tests. The complete browser runner passed **215**
cases plus integrated startup/review/refresh/orientation flows; an additional three-sibling
move regression and final constructor assertions passed separately afterwards. Browser
coverage includes actual native revisions, literal styled text, system/override/storage
fallback, message lifecycle, drag/keyboard controls, narrow layouts and 2,000-directory
virtualisation. Frozen review materialisation with local history also passed its verifier.

[The compact validation record](results/orientation-validation.json) retains hashes,
individual mutation outcomes and performance observations. Mutation scopes include the
complete native revision-resolution functions, complete scene module, complete
appearance/message/bootstrap modules and directory controls. Every cohort passed its
unmodified baseline. Native compiler/jobserver tasks, test threads and mutant jobs were
serial and lower priority; browser mutations used one lower-priority worker. No exclusions
were applied. Timeouts/build failures are separate from failing-test catches. Stryker's
aggregate score includes timeouts; it is not presented here as the failing-test count.

Mutation review strengthened hierarchy/count/focus/breadcrumb, admission, metadata,
preference-boundary, palette, bootstrap, initial-message and directory-boundary tests.
It exposed the oversized-update state bug, which was reproduced and fixed. Surviving
variants remain; this is not full coverage or a proof of correctness. In particular:

- Scene survivors include valid alternative partitions/insets, redundant ordered-prefix
  checks, cache/performance variants and validation/state edges. Tests preserve structural
  and resource invariants rather than exact incidental coordinates. Residual mutation
  gaps remain explicit in the record.
- Appearance survivors include equivalent System attribute representations, redundant
  normalisation/whitespace handling and initial-state variants. Constructor/media callback
  assertions added after the broad follow-up are not credited as caught without a rerun.
- Directory-control survivors include event/default-action, styling/label, focus and
  virtualisation variants. The focused follow-up improves behavioral checks but does not
  establish exhaustive keyboard/drag coverage. The later three-sibling move case is kept
  separately from mutation counts.

## Standalone performance observations

Optimized observation ran after all test and mutation processes finished, with one
parser worker. A 2,000-directory generated repository and the pinned lpc-rs source copy
were exercised separately. Both retained the same captured source through pan, collapse,
expand, reorder, focus/breadcrumb and theme actions.

| Observation | 2,000 directories | lpc-rs (552 files) |
| --- | ---: | ---: |
| Pan rAF samples | 93 | 92 |
| Observed rAF p99 | 16.8 ms | 16.8 ms |
| Drawing CPU p50 / p99 | 0.9 / 2.0 ms | 0.6 / 1.0 ms |
| Rendered directory controls | 11 | 11 |
| Native RSS sample | 17.0 MiB | 33.2 MiB |

No long tasks were observed in the pan samples. The scripted layout actions, including
automation observation and two rAF callbacks, took about 64–67 ms; theme actions about
33–34 ms. These are small samples, not input/presentation latency guarantees or tail
qualification. Native RSS excludes the browser and is not a combined memory envelope.
The raw report, source/binary hashes and limitations are retained in the compact record;
local screenshots/raw data are under `/tmp/vizzi-orientation-performance`.

These observations are from Chromium on this development host. They do not establish
other-browser/OS support, the 16-GB laptop envelope, whole-repository review latency or
an unfamiliar developer's comprehension benefit. The maintainer should retry the actual
reported interactions; published hierarchy results are task-dependent.
