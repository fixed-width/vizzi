# Verified dirty-path capture results

Ordinary lpc-rs edits now reopen **one file / 4,091 bytes**, retaining 550 captured
documents instead of rereading all **551 files / 7,562,179 bytes**. In the generated
10,008-file browser workload, each observed single edit reopened one file and retained
10,007 documents. Incremental snapshots matched fresh full captures on both workloads.

## Implementation and recovery

The [capture contract](../../docs/prototype/INCREMENTAL_CAPTURE.md) keeps hints separate
from content proof. Hinted files are reopened and hashed regardless of size or timestamp.
Unhinted bytes/facts come from the published baseline. Scope/context mismatches, unknown
parents, indexed subtrees, ambiguous events, watcher errors, and hint overflow reconcile.
Both named sides of a rename are useful hints; folder topology still requires discovery.

Taking a job does not consume its paths. Only accepted publication acknowledges them;
cancelled, failed, or superseded work leaves the hints pending. Full recovery stays on
a deadline during sustained writes, and watch registration is repaired during complete
passes. New watches trigger another pass to close their discovery gap.

Capture preflights candidates before reading bytes so a topology fallback does not first
perform a partial batch's reads. Existing symlinks are excluded. Filesystem observations
remain non-atomic against concurrent mutation; this does not add a capability-based
filesystem sandbox or close every path-substitution race.

## Native observations

The native driver used the pinned lpc-rs revision
`8d3b1f54b36eae33f689bae3e42c716647000e4d`, four workers, warm OS caches, and an owned
copy. Three edits each reported one parse, one read, 550 unread retained documents,
and a non-reconciled capture. Each was checked against a fresh full capture from a
second process using the same source and analyzer configuration.

| Endpoint | Three observed values/range |
| --- | ---: |
| Candidate validation and source read | 0.147–0.248 ms |
| Complete incremental engine attempt | 4.71–6.46 ms |
| Write through coherent diagnostic response | 34.21–37.38 ms |

These are three observations, not latency tails. They include context checks and
snapshot construction where indicated; they do not measure physical storage durability.
See the [native record](results/lpc-rs-native-2026-11-01.json) for exact timings,
cache counts, bytes, source revision, and executable fingerprint.

## Browser observations

This repeats the [notification procedure](../browser/NOTIFICATIONS_RESULTS.md) in
software-rendered headless Chromium 156.0.8078.4 on the same i7-13700H/62.5 GiB host.
The viewport is 1440 × 900, with three four-second runs per condition and five source
edits. Native reconciliation remains active; Node instrumentation samples status,
while the browser itself makes zero status-polling requests.

| Workspace | Notifications-only median | Dirty capture median | Dirty-capture observed range |
| --- | ---: | ---: | ---: |
| lpc-rs, 551 Rust files | 85 ms | 77 ms | 70–86 ms |
| Generated, 10,008 Rust files / 2,000-tile overview | 169 ms | 109 ms | 99–122 ms |

Every observed source edit in this run used one reopened file and a partial capture.
The generated workload's candidate phase was approximately 0.99–1.03 ms, but complete
engine attempts still took 29.39–33.38 ms. Removing whole-scope I/O does not remove
building maps/snapshots and hashing serialized facts for unchanged documents.

During background parsing, CPU Canvas submission p99 was 0.4 ms for lpc-rs and
0.9 ms for the generated view; independent rAF p99 remained 16.8 ms. These are bounded
reference-view scheduling/CPU observations, not GPU execution or physical presentation.
Five edit observations and short interaction runs do not establish general tail latency.

See the [software browser record](../browser/results/capture-software-2026-11-01.json)
and [workspace screenshot](../browser/results/capture-software-2026-11-01-lpc-rs-workspace.png).
Source manifests match the notification baseline. Reports identify base commit `f3944bd`
and the measured working tree through app/executable fingerprints.

The headed Intel run failed the minimum-frame check during the first refresh condition
and produced no report. One retry after explicitly foregrounding the test page failed
the same check. Its cause was not established; no new Intel/GPU performance claim is
made. Earlier successful Intel measurements remain a separate baseline, not evidence
for this implementation's hardware timing. The software run used the existing page
setup; foregrounding was added for the retry and does not change the application binary.

## Correctness and mutation evidence

Tests compare hinted results with clean capture and independently specified source/facts.
Silent unhinted edits intentionally retain their old captured value until reconciliation.
Live process checks cover atomic saves, new-directory watches, and full reconciliation
while writes keep arriving every 75 ms. Unit tests exercise dropped-host closure,
registration repair/failures, exact hint limits, malformed events, and context mismatch.
OS queue overflow and adversarial filesystem races were not induced.

| Complete-module mutation scope | Tested | Assertion failures | Unviable | Timeouts | Survivors |
| --- | ---: | ---: | ---: | ---: | ---: |
| Source, engine, invalidation journal | 123 | 109 | 11 | 3 | 0 |
| Host including live CLI integration | 30 | 19 | 10 | 1 | 0 |

Both baselines passed. One narrowly documented equivalent mutation is excluded because
a validated file path cannot equal a directory prefix ending in `/`. Timeouts and
unviable replacements are not counted as assertion failures. See the
[capture mutation record](results/capture-mutation-2026-11-01.json) and
[host mutation record](results/host-mutation-2026-11-01.json). Mutation results are
evidence rather than proof. Browser JavaScript was unchanged in this implementation.
Formatting, 104 Rust tests, doctest checks, Clippy with warnings denied, and 28
Node/browser checks pass. No new dependencies or unsafe code were added.

## Next indexing work

Cache an immutable document's analysis fingerprint instead of serializing all unchanged
facts again when constructing a new snapshot. The generated workload's approximately
1 ms capture phase versus approximately 30 ms engine attempt makes that remaining
work visible. Preserve content/context identity and clean-capture equivalence when
changing snapshot construction. The production renderer, mixed-language application
indexing, and 16 GB baseline remain separate qualification work.
