# Fresh question trial: Tract Plugin Pack

For maintainers deciding the next search iteration. **The current focused heuristics do
not generalize adequately to these two fresh tasks.** They find a slider-use example for
the GUI question, but route the Minitosis signal-path question into GUI commit/check
handlers. Original matching offers a useful Minitosis file entry without exposing the
runtime chain. Neither result set provides the widget inventory or signal trace.

The [source-audited reference](TRACT_PLUGIN_PACK_SOURCE_REFERENCE.md) answers the two
questions from code, including a widget table and signal-path diagram. That reference
was assembled after the first results; it is not an answer produced by Vizzi or a model.

## Frozen protocol and scope

The maintainer supplied these exact questions:

1. “Show me the different gui widgets that are used by the plugin pack.”
2. “help me trace the signal path from input to output for minitosis”

The [cohort](library/tract-plugin-pack.json) was frozen before source-content inspection
or retrieval. It records Vizzi commit `adefa08`, the release-binary SHA-256, source revision
`951cf85413029423e036c641638b845edafe75cf`, the original clean worktree and evaluation policy.
The source was exported locally with `git archive`, then hashed. It was not built or run.
No ranking, query text, expansion, model weights or production code changed during this trial.
Both questions remain evaluation-only; a future repeat cannot be described as their first
unseen evaluation. The eight earlier reserved real-source cases remain unrun.

The export contains 445 regular files, including 231 Rust files, 123 Markdown files and
14 PDFs. The native host indexed/searched all 231 Rust files with zero **additional search
admission omissions**. This does not mean all 445 repository files were searched: the
workspace manifest, manuals and other unselected content are outside the indexed corpus.
There are no unexported submodules. The target project's own external dependencies are
outside the export.

The unchanged native viewer used one indexing worker at nice 10, no model and the fixed
8-result/4,000-excerpt-byte allowance. First acquisition was counterbalanced: focused then
original for question 1; original then focused for question 2. All four first responses
were archived and hashed before source-aware relevance auditing or browser navigation.

## First result quality

The post-run source audit identified eight shared widget/control families with production
uses and six primary audio-path anchors. These are finite agent-authored reference
anchors, not exhaustive semantic ground truth. They were selected after retrieval, so
this is a source-informed audit rather than a blind gold evaluation.

| Question | Focused search | Original matching |
| --- | --- | --- |
| Widget inventory | First result is GS Meter's slider-row macro and includes a `draw_slider` call. Other hits are mostly editor/window scaffolding or unrelated plugin code. **0/8** shared-family definition anchors. | Editor/window context and a shared `EditorHandle::new` result at rank 7; no inventory. **0/8** shared-family definition anchors. |
| Minitosis signal path | Starts with `MinitosisWindow::commit_output_text`, followed by UI text-edit, click-check and track-swap functions. **0/6** primary runtime anchors. | `Plugin for Minitosis` appears at rank 2; its short excerpt is the impl header/constants. It provides file orientation, but no runtime chain. **0/6** primary runtime anchors. |

The widget audit distinguishes “declared” from “used”: controls/dials/dropdowns/text entry,
MSEG editing and the shared effect editor have concrete use sites. The generic grid
selector is exported but has no production callers outside its own crate in the audited
revision. The old Vizia `nih-plug-widgets` crate is workspace-excluded. A simple inventory
of all widget-looking declarations would therefore include misleading entries.

The actual signal path is `Plugin::process → Minitosis::render → Processor::process_sample`,
with block modulation setup, an aligned dry path, selected-effect dispatch, wet Amp/Mix,
bypass/pending-switch handling and post-blend Output gain. Those functions and their
supporting files are already inside the indexed Rust scope. The failure to retrieve them
is not a missing-language or repository-size problem.

## Diagnosis

The source of the wrong intent is visible in the unchanged
[plan selection](../../crates/vizzi-search/src/discovery.rs):

- `plugin` alone can select the registration/example plan, although question 1 asks for
  a component inventory rather than adding a plugin.
- `trace` selects operation pools dominated by commit/send/dispatch and validation/check
  terms. That prior came from transaction workflows; it is inappropriate for this audio
  signal-path question. Generic input/output terms allow unrelated editor code to compete,
  without a required Minitosis implementation anchor.
- Eight ranked snippets do not enumerate shared component families or distinguish current
  production use, exported-but-unused APIs and excluded legacy crates. Manifest/document
  context needed for that distinction is not in the current search scope.

The candidate headings accurately label heuristic pool origins, but “Entry and use
candidates” above a GUI text-commit callback can still mislead an unfamiliar reader.
Low latency and valid source bindings do not compensate for that relevance failure.
This result evaluates the existing heuristics; it does not test a small model's ability
to route or rank these tasks.

## Browser workflow and retained evidence

A later source-informed, headless replay reproduced all four first result sets exactly,
including their grouped visible order. It inspected the first result in each ordering,
then kept one usage/orientation clue per question: the focused slider result and the
original Minitosis impl result. These were separate investigations with the exact saved
questions. Both editor handoffs used a recording executable; no desktop editor or target
plugin was launched. There were no page errors, and owned hosts/browser instances closed.
This establishes function, not independent navigation success or comprehension.

Retention exposed another limit. The slider search excerpt spans lines 280–292 and
contains `widgets::draw_slider`; the saved location captures lines 280–285, before that
call. The Minitosis saved excerpt contains the impl header/constants at lines 184–189,
not its later `process` method. Snapshot/file/version identity is correct, but saving a
location's default six-line window does not necessarily retain the code that made the
search result useful. Navigation anchors and selected evidence spans need distinct
consideration. See the [retention audit](results/tract-plugin-pack-fresh/retention-audit.json).

## Costs and verification

These are single first-observation request times, not latency distributions:

| Request order | Question / selector | Full-body delivery |
| --- | --- | --- |
| 1 | Widgets / focused | 849 ms, including initial search-cache preparation |
| 2 | Widgets / original | 232 ms |
| 3 | Minitosis / original | 230 ms |
| 4 | Minitosis / focused | 266 ms |

Source activation took 1.03 s. The cold first request cannot be used to infer a focused
versus original speed difference. No memory or GPU qualification was performed here.

Verification checked frozen cohort/source/binary hashes, exact question and observation
bindings, finite result/excerpt budgets, identical API/browser results, grouped display
order, previews, saved investigations and recording-only handoffs. Production Rust/frontend
code is unchanged; the previous 507-test/browser/Clippy/mutation baseline is retained,
not claimed as newly rerun. The acquisition/replay scripts were syntax-checked and executed
against the real frozen source; no new mutation run was used to qualify these experiment
coordination scripts.

## Artifacts and reproduction

- [Prepared source manifest](results/tract-plugin-pack-fresh/prepared.json),
  [first paired responses](results/tract-plugin-pack-fresh/first-results.json), and
  [first-response hashes](results/tract-plugin-pack-fresh/first-results-sha256.json).
- [Post-run source anchors](results/tract-plugin-pack-fresh/source-audit.json),
  [anchor assessment](results/tract-plugin-pack-fresh/assessment.json),
  [browser replay and saved investigations](results/tract-plugin-pack-fresh/replay.json).
- [Focused GUI view](results/tract-plugin-pack-fresh/browser/tract.gui-widget-inventory/focused.png),
  [original Minitosis view](results/tract-plugin-pack-fresh/browser/tract.minitosis-signal-path/original.png),
  [focused Minitosis view](results/tract-plugin-pack-fresh/browser/tract.minitosis-signal-path/focused.png).
  Additional inspection/saved-view captures and logs are retained beside these.

For a prepared immutable export, use **new** output directories to preserve first results:

```sh
nice -n 10 node experiments/companion/fresh-questions.mjs \
  /path/to/prepared.json /tmp/new-fresh-first
nice -n 10 node experiments/companion/replay-fresh-questions.mjs \
  /path/to/prepared.json /tmp/new-fresh-first/first-results.json /tmp/new-fresh-browser
```

The replay contains source-informed choices for this cohort; use it as functional evidence,
not a generic independent-evaluation driver. The prepared manifest records its local export
and archive; re-create that revision and hashes for another machine. Repeating these cases
is a regression check, not fresh held-out evidence.

## Consequence for the next step

Keep the trial opt-in. Before expanding the transaction-derived keyword pools or adding
inference, define general inventory and flow-search behaviour using separate development
cases. Require a repository/component anchor for flow exploration, preserve original search
as a control/fallback, and evaluate meaningful retained spans separately from source-version
correctness. A small model can still serve as an optional intent/selection layer; these two
cases show what that layer must improve, not that it already works.
