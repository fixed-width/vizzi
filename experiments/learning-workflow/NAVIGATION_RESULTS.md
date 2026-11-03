# Back/Forward preserves the learning task's reading context

The [baseline](RESULTS.md) lost the selected function and reading position during a
cross-file investigation. Back/Forward now restores that location. The repeated guided
lpc-rs task uses the same pinned source archive.

| Return to the compiler read policy | Baseline file-list return | New Back navigation |
| --- | --- | --- |
| Selected declaration | None | read_file, line 151 |
| Source scrollTop | 0 | **3234, exactly matching departure** |
| Visible source | File start | Prior reading position, lines 148–184 |
| Browser Back after the investigation | Left for about:blank | **Stayed inside the workspace** |

The mechanisms differ deliberately: fresh file selection keeps its existing behaviour;
an explicit return restores a saved location. This is reference-aware functional
automation, not an unfamiliar-developer study or task-time comparison.

[Watch the recording](results/navigation/walkthrough.webm),
[inspect the restored location](results/navigation/07-history-return.png), or
[inspect browser Back](results/navigation/12-browser-back.png).

## Behaviour and ownership

The [navigation contract](../../docs/prototype/NAVIGATION.md) defines 50 metadata-only
visits, branch truncation and explicit expiry. Records hold identities, filters and
reading positions, without source bodies or snapshot leases. Browser markers contain
only a session/key; reload starts a new trail.

Returns follow the latest snapshot. Matching document/declaration versions restore the
inspector position, keyboard cursor and long-line fragment. Changed contexts show a
notice and reveal the current declaration or file start. Missing files reject the
server's selection fallback before it replaces the current view. Failed returns remain
traversable/retryable; fresh actions can supersede pending returns.

The saved filter is the last displayed filter before departure. It can leave the selected
file outside the overview, consistently with ordinary filtering. The source heading
still identifies the selected file.

The navigation module owns trail state and intent. The controller carries intent through
coalescing/retry and rolls back failed return queries. The inspector owns viewport capture
and restore. No new analysis fact, SCM dependency or native retention policy is introduced;
the host serves one additional JavaScript asset.

## Correctness evidence

TDD first records missing controls. Further regressions expose a rejected-return/publication
race and a removed declaration in the current file retaining an unrelated scroll position.
Both are fixed. Changed-context returns reveal current code instead of remapping an old
cursor or reusing stale coordinates.

Formatting, **302 Rust tests**, doctests, Clippy and **88 Node/browser checks** pass.
Real-browser checks cover toolbar/native history, exact restores, other-file edits,
changed/removed declarations, missing files, request failure/retry and fresh selections
superseding delayed returns. DOM tests add bounds, focus/cursor state, long-line fragments,
horizontal position, empty views and changed-context resets.

Complete navigation/controller modules and the inspector's complete position/prepare/apply
methods yield **647 final mutation outcomes: 614 killed, 33 timeouts, zero survivors,
no exclusions or run errors**, after passing baselines. Timeouts break progress and are
separate from test kills. Exploratory findings strengthened state/viewport checks and
simplified redundant handling. An initial inspector run lacked Chromium mutation activation
and was discarded; final tests use the installed Stryker constants in the browser.

App wiring, markup/CSS and native asset plumbing are outside mutation scope and are
exercised by end-to-end/native tests. Evidence does not prove correctness, historical-source
retention, unknown-language semantics or unfamiliar-user usefulness.

## Reproduce and inspect

```sh
npm test -- --learning-walkthrough --return-history \
  --source ../lpc-rs \
  --revision 1a4f5b65dd2a269298748df2dfd68092a169a8d9 \
  --output /tmp/vizzi-learning-navigation-replay
npm test -- --navigation
npm test -- --navigation-inspector
npm test -- --navigation-mutations
npm test -- --navigation-inspector-mutations
```

Choose a new replay output directory. The original checkout stays unchanged and no
analysed-project tooling runs. One observation is retained throughout the recording;
408 visible reference-row occurrences are independently checked. Scripted pacing and
warm caches do not qualify performance.
One response body was unavailable to the metadata collector after navigation/cancellation;
the raw diagnostic is retained. The response audit is therefore partial. There are no
page-script errors, and displayed-source checks do not depend on collecting that response.

[Raw UI states](results/navigation/observations.json), [summary](results/navigation/summary.json),
[mutation outcomes](results/navigation/mutation.json) and
[source/artifact hashes](results/navigation/source-sha256.json) retain the evidence.
Full traces are generated locally; video and checkpoints are committed.

Next is the contextual-review fixture and comparison contracts in the
[milestone plan](../../docs/plans/product-milestones.md). Declaration lookup and structural
views remain separate candidates guided by future task evidence.
