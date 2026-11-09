# Function display-margin validation

For maintainers checking removal of enclosing file indentation, 2026-11-09.
Function bubbles now start at the code gutter in Smart reflow and Original lines.
Internal indentation remains relative to the function. The captured declaration line
supplies one baseline for the whole fragment; paging and restoring a later section do
not infer a new baseline from the body.

The implementation interprets tabs at four-column stops, hides original leading bytes
in the DOM, and displays the remaining relative indentation as decorative width. This
keeps original source strings, styles, call identities and byte/line coordinates intact.
Copying from a visible line start restores the hidden original prefix. Ordinary source
windows and source/cache identities are unchanged. A later-section restore fetches one
bounded pinned declaration window; paging with a known baseline and resizing do not.

## Verification

Formatting, Clippy with warnings denied, workspace tests/doctests and the full browser
suite pass: **592 native tests passed, 3 opt-in provider tests ignored** in the normal
workspace command. An initial native run failed the existing six-second filesystem
reconciliation deadline while checks overlapped. That test passed alone, and the complete
native suite passed on rerun; no reconciliation code or deadline was changed. Its initial
failure and follow-up logs are retained separately.

Focused tests cover tab/mixed-space columns, unindented and shorter-indented lines,
clipped/incomplete headers, fallback font measurement, gutter alignment, literal source
copying with CRLF, syntax spans, live inline call actions and original editor positions.
The mixed Rust/Python/C++ fixture verifies the baseline across a long function's later
section and a reload, both layout modes, and local resizing. Tests use a recording editor.
The initial interface/DOM checks reproduced the missing margin support and lost prefix
when copying from the visible start; both were fixed before integration checks.

The complete new pure margin module and the complete reflow planner were mutation-tested
with one lower-priority worker. Unmodified baselines pass. Final results are **235 caught,
7 surviving, 0 timeouts, 0 errors**, with no exclusions. One survivor removes a redundant
regex start anchor: the zero-or-more prefix still first matches at position zero. Six
planner survivors are functionally equivalent defaults/coercions, deterministic measurement
cache/fast-path changes, unobservable caught-error messages or a one-call tighter work
ceiling within the public limit. Timing effects of cache changes are not established by
unit tests. The initial scanner-loop timeout was eliminated by using a bounded string
iterator. DOM/controller paths have browser checks rather than a complete mutation scope.

## Real repository observation

After other checks stopped, the optimised host and isolated Chromium replay opened the
known Minitosis process → render trail and GsMeter process in `tract-plugin-pack`.
Screenshots show the function headers aligned at their code gutters and preserved body
nesting. The caller and other panes retain their positions. Across **69** instrumented
layout updates, median cost was **1.0 ms**, 95th percentile **1.9 ms**, maximum **4.6 ms**,
with **zero source requests during resizing**. These are local one-session observations
of SourcePane layout work, not full-frame/GPU or baseline-laptop qualification.
No inspected source, editor windows or repository builds were changed/launched by this replay.

[Logs, mutation outcomes, screenshots and checksums](results/function-margin/checksums.json)
include the source revision and release-binary hash. Reproduce the known-source observation
with `experiments/visual/replay-reflow.mjs ROOT NEW_OUTPUT`. See the
[usage guide](../../docs/how-to/follow-a-call-trail.md) and
[architecture](../../docs/visual/TECH.md) for the behaviour and source-binding rules.
