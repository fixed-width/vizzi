# Replay the baseline codebase-learning task

Use this how-to to record the current map/source/outline UI answering the first
[product-milestone question](../../docs/plans/product-milestones.md). It is a functional
walkthrough, not a performance benchmark or a newcomer study. The
[report](RESULTS.md) explains the navigation gaps.

From the workspace root, with existing [browser prerequisites](../browser/README.md)
and a local lpc-rs repository containing the pinned commit:

```sh
npm test -- --learning-walkthrough \
  --source ../lpc-rs \
  --revision 1a4f5b65dd2a269298748df2dfd68092a169a8d9 \
  --output /tmp/vizzi-learning-replay
```

Choose a new output directory. The runner archives that exact Git revision into a
temporary directory, builds Vizzi's release host and starts it with a fresh analysis
cache. It does not change the original checkout, execute lpc-rs, enable C-family capture
or analyse LPC. Default Rust/Python capture remains in effect.

Add `--return-history` with another output directory to exercise the delivered
navigation feature. See the [updated recording](NAVIGATION_RESULTS.md). Original baseline
artifacts describe commit b866b1f; check out that commit to reproduce its lack of history.

Playwright navigates the real UI using the file filter, accessible file list,
declaration outline and source scrolling. It records screenshots, video, raw UI states
and a full Playwright trace. Response metadata audits snapshot binding and filter
readiness; it does not supply navigation targets or answers.

The route knows the reference files and declaration names. It resets the outline to
Home or End for each target, and can match a button's full accessible text when its
visual label is ellipsised. These choices make replay reproducible; action counts and
duration are not estimates of novice effort or the shortest route. Deliberate search
and Back probes are capability checks.

After the route, displayed reference-file rows are checked against the archived source.
The runner records hashes and checks that the original checkout's status is unchanged.
Host, browser and temporary source/cache are closed; output artifacts remain.

## Inspect the artifacts

- `observations.json`: question, operator limitations, pin/archive hash, ordered UI
  states, visible source rows, outline scans and public response bindings.
- Numbered PNGs: unedited checkpoints in route order.
- `walkthrough.webm`: recorded page interaction.
- `trace.zip`: full local Playwright trace. It is omitted from the committed bundle
  because it is much larger; the summary records its size and hash.

Open a generated trace with the installed viewer:

```sh
npm exec playwright show-trace /tmp/vizzi-learning-replay/trace.zip
```

The checked-in [recording](results/walkthrough.webm), [observations](results/observations.json),
[summary](results/summary.json) and [hashes](results/source-sha256.json) retain the baseline.
Fresh runs have different temporary roots/ports, so opaque workspace/file/snapshot IDs
can differ at the same source revision.

## Use the question with an unfamiliar developer

Give the operator only this task:

> How does the compiler obtain source, decide whether to retry a path with the c
> extension, and report a read failure? Explain the behaviour and cite the source
> and tests that support your answer.

Provide a pinned workspace through the [normal launch procedure](../../docs/how-to/run-workspace.md).
Keep the oracle, scripted route and results hidden until the session ends. Record
familiarity, questions, navigation, incorrect conclusions and uncertainty. Human
comprehension benefits require separate evidence from this guided agent replay.
