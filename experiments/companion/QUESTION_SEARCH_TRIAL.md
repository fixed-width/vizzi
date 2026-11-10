# Question-search trial instructions (archived)

This browser workflow is retired. These instructions describe revision `7cb4956`; use
that checkout to reproduce the historical UI trials. Current Vizzi has no question-search
entry point or question editor. Use the [visual call workspace](../../docs/how-to/follow-a-call-trail.md)
for current usage.

For a maintainer evaluating whether Vizzi helps find useful source in an unfamiliar
repository. This trial returns source starting points, rather than a generated answer.
It uses no model or GPU inference. Any local repository can be used; it need not be one
of the three reference repositories. Rust and Python have the most exercised analysis.
Mixed repositories retain the configured language and source-coverage limits.

## Prepare fresh questions

Before opening search results, record the repository/revision and two or three questions
you would actually ask. Include a general orientation task and a repository-specific
workflow when useful. Keep these questions out of retrieval tuning. Existing efun and
transaction questions are development examples, not fresh tests.

For each question, record who wrote it, familiarity with the repository and any prior
inspection of source or search output. An agent-authored, source-aware question is useful
for a reproducible check but does not establish independent maintainer acceptance.
There is no need to invent a gold answer before asking; inspect source and record relevant
locations afterwards. Preserve the originally written wording.

## Run the trial

Build and launch the native host, with state outside the inspected checkout:

```sh
cargo run --release -p vizzi -- \
  --root /path/to/repository --cache /tmp/vizzi-question-analysis \
  --investigation-file /path/to/notes/question-trial.json --workers 1
```

Open the printed address with `/investigate?trial=search` appended. The default overview
also has **More → Try question search**. Type a question and choose **Search sources**.
Groups are heuristic candidates, not verified calls or complete code paths. Check the
searched-file scope: this trial searches captured indexed source, excluding unselected
documentation and files outside its encoding/size/count allowances. When capped, files
are admitted in path order, so relevant later paths can be absent.

Choose **Inspect source** to see the captured version. **Keep this location** retains
useful evidence in **Saved locations**. **Keep question** separately saves the input text;
searching alone never replaces a saved investigation question. After keeping evidence,
**Open in VS Code** uses the existing verified working-file handoff. Use
`--editor-code /path/to/code` if `code` is not on PATH. The handoff opens a live file and
cannot inspect an unsaved editor buffer. No editor opens merely from searching.

Expand **Trial comparison** and select **Original matching**, then submit the same question
to compare ordering against the focused search. Use the same source revision. Try the two
orders in alternating first positions across questions, and record that seeing the first
order may help with the second. This is a paired navigation trial, not a blinded study.

If source changes, search again before keeping a result. Saved evidence stays bound to its
captured version; it does not silently move to new bytes. Errors can be dismissed and
queries retried. No matches means no matches within the displayed scope.

## Record the outcome

For each ordering and question, record:

- Whether a useful starting point appeared, its position and verified source location.
- Results inspected, locations kept, editor handoffs, rephrasing and manual browsing needed.
- Time to the first useful location, including unsuccessful searches.
- Missing languages, files or supporting relationships, and misleading group suggestions.
- Whether the retained context helped resume the task beside the editor.

Keep raw observations separate from your final judgment. Compare with finding the same
kind of information in the editor alone, noting familiarity and order effects. Export
saved evidence using **More → Export investigation** if no investigation file was configured.
Stop the host with Ctrl-C afterwards.

The [behaviour contract](../../docs/companion/SEARCH_PRODUCT.md) and
[architecture](../../docs/companion/SEARCH_TECH.md) describe the bounds and version guarantees.
Automated replay and optimized-build measurements are recorded in
[the implementation results](QUESTION_SEARCH_RESULTS.md);
they do not replace a fresh-question maintainer trial.

## Reproduce the automated checks

The focused browser replay uses a recording editor executable and closes its own host:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  node experiments/browser/question-search.test.mjs
node crates/vizzi/web/question-state.test.js
```

For optimized native/browser timing, first finish builds, tests and mutations. Build the
release binary, then use a new output directory and a fixed source checkout:

```sh
CARGO_BUILD_JOBS=1 nice -n 10 cargo build --release -p vizzi
nice -n 10 node experiments/companion/measure-question-search.mjs \
  --source /path/to/pinned/lpc-rs --output /tmp/vizzi-question-measure
```

This timing script deliberately replays the three known development questions. It checks
literal excerpts and runs a source-aware, two-file save/handoff replay using a recording
editor. Its results do not measure fresh-question relevance. Keep it separate from the
maintainer trial above. Linux process-tree memory can be observed by wrapping the command
with `vizzi-envelope --observe-only /tmp/vizzi-question-memory.json --`.

For a completed first-acquisition example, see the [Tract Plugin Pack trial](../../experiments/companion/TRACT_PLUGIN_PACK_FRESH_RESULTS.md).
Its two cases are now observed evaluation cases; reuse them as regressions rather than
calling a later repeat fresh. The report separates returned clues, source-audited answers
and browser replay from independent maintainer task completion.
