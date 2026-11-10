# Workspace control and expansion validation

For maintainers checking the workspace interaction cleanup, 2026-11-09.

Workspace actions and Settings are separate. Canvas views are called bubbles; retained
anchors are saved locations. The example button and inline arrow labels are removed.
Arrow hover/accessibility details and line styles retain current/uncertain/historical
meaning. Minimap rectangles identify their qualified function and source path/line.
Definition navigation preserves zoom, and explicit Fit workspace changes scale.

Height and width expansion use the current loaded section, without source transport.
The chosen bubble stays anchored; a deterministic local push makes room downstream,
including a 24-pixel forward gap. Width caps at 1,600 and height at 2,400 canvas pixels;
larger displayed content remains scrollable. Uncollapsing also makes room. Unaffected
bubbles, source identities and the camera remain independent of expansion.

Formatting, 605 native tests/doctests (3 opt-in tests ignored), Clippy with warnings denied,
and the complete browser suite pass. Twelve focused layout tests cover axis growth,
cascading displacement, unrelated/pre-overlapping bubbles, exact edges, persistence,
atomic bounds rejection, collapsed views, zoom and destination visibility. Browser checks
cover actual menus, tooltips, labels, source-only measurements and unchanged request counts.
Initial regressions reproduced zoom changes, missing controls and the touching-edge gap.

All interpreted mutation runs have passing baselines and one lower-priority worker.
The broad model/layout/coordinator pass finished with 1,220 caught and 36 surviving.
After the touching-edge fix, the complete layout module was rerun separately:

| Final scope per module | Caught | Surviving | Timeouts/errors |
| --- | ---: | ---: | ---: |
| Workspace presentation model | 1,023 | 18 | 0 |
| Live update coordinator | 76 | 9 | 0 |
| Complete final expansion module | 123 | 9 | 0 |

The scopes are separate archived runs, not a combined fresh full pass. Expansion survivors
are diagnostic strings, the redundant missing-ID guard (later access still rejects),
finite-input guards under the validated model interface, and equivalent bounded-loop/
nearest-versus-farthest blocker traversal. Model/coordinator survivors are overlapping
validation/supersession guards, equivalent optional/default/context handling and current-link
replacement. Empty reveal is also a no-op without its early return. No exclusions hide
these outcomes. SourcePane measurement and controller DOM behavior have browser regression
coverage rather than a complete mutation scope. This is evidence, not whole-project proof.

The known Minitosis process → render replay uses a temporary source copy, verifies
expansion without zoom changes and minimap titles, and captures both themes. Visual
inspection caught the forward-gap edge case before completion. It launches no inspected
repository build, editor or language server and makes no timing or independent usability
claim. Reproduce with `experiments/visual/replay-controls.mjs ROOT NEW_OUTPUT` after building
the release host. [Logs, mutation outcomes, screenshots and source/binary hashes](results/controls/checksums.json)
are archived compactly. See the [usage guide](../../docs/how-to/follow-a-call-trail.md),
[behaviour contract](../../docs/visual/PRODUCT.md) and [architecture](../../docs/visual/TECH.md).
