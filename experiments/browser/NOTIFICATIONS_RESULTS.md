# Versioned change-notification measurements

This records the notification-only baseline. Subsequent
[dirty-capture measurements](../incremental/RESULTS.md) repeat the software workload
after avoiding unhinted source reads.

Replacing browser status polling with latest-state notifications reduced median
lpc-rs update observations from approximately **402 ms to 85–86 ms** in these runs.
The browser made **zero `/api/status` requests**. Source and outline still arrive in
one snapshot-pinned response, and reconnects preserve selection and recover changes
made while the host was unavailable.

## Method and endpoints

This repeats the [browser reference procedure](README.md) and compares with the
[polling baseline](RESULTS.md). Both use Chromium 156.0.8078.4, Node 26.13.0, Playwright
1.64.1, four parsing workers, a 1440 × 900 viewport, and the same source manifests.
The i7-13700H host has approximately 62.5 GiB RAM. Headless Chromium uses SwiftShader;
headed Chromium uses Intel Iris Xe through Mesa 25.2.8. These do not qualify the
16 GB baseline under memory pressure or physical display presentation.

The synthetic workspace contains 10,008 Rust files and 4,670,156 bytes, with an
overview capped at 2,000 tiles. Background runs change eight files with 128,008
function declarations. lpc-rs contains 551 Rust files and 7,562,179 bytes at revision
`8d3b1f54b36eae33f689bae3e42c716647000e4d`; its background batch changes 32 files.
Original sources remain untouched and no analysed-project build scripts run.

Each condition has three four-second pan/zoom runs and five source-edit observations.
The source-edit endpoint is matching visible source DOM followed by two rAF callbacks,
including native debounce, source capture/analysis, transport, and browser scheduling.
It is not GPU completion or physical presentation. Five observations do not establish
tail latency. Before/after runs were collected at different times on the same host;
they are not a randomised statistical comparison.

## Source-edit observations

| Workspace / renderer | Polling median | Notifications median | Notifications observed range |
| --- | ---: | ---: | ---: |
| lpc-rs / SwiftShader | 402 ms | 85 ms | 80–88 ms |
| lpc-rs / Intel | 403 ms | 86 ms | 82–94 ms |
| Generated / SwiftShader | 475 ms | 169 ms | 152–172 ms |
| Generated / Intel | 402 ms | 155 ms | 150–186 ms |

Polling phase no longer adds up to another 500 ms to a completed refresh. The larger
workspace still rereads its Rust scope and rebuilds snapshot identity on every refresh;
notifications do not optimise that native work. Its remaining update delay is therefore
not eliminated by this transport change.

Each workspace/mode recorded eight `/api/events` requests and 17 `/api/view` requests
over the complete procedure, including deliberate page reloads, picking, and edits.
None were `/api/status` requests. The Node measurement driver separately samples status
to classify background activity; those probes are instrumentation, not browser polling.

## Interaction during background parsing

Nearest-rank percentiles combine the three refresh runs. CPU submission covers the
JavaScript Canvas command loop, not GPU execution.

| Workspace / renderer | CPU draw p99 / maximum | rAF p99 | Observed draw p99 / maximum |
| --- | ---: | ---: | ---: |
| lpc-rs / SwiftShader | 0.5 / 1.0 ms | 16.8 ms | 16.8 / 50.1 ms |
| lpc-rs / Intel | 0.6 / 2.2 ms | 16.8 ms | 16.8 / 50.0 ms |
| Generated / SwiftShader | 1.1 / 2.7 ms | 16.8 ms | 16.8 / 50.1 ms |
| Generated / Intel | 1.1 / 9.5 ms | 16.8 ms | 16.8 / 50.1 ms |

The remaining draw-cadence gaps and the 9.5 ms submission outlier are retained in the
raw results. Stable rAF percentiles alone do not establish stutter-free presentation.
These bounded reference-view measurements do not compare rendering backends, dense
edges, very large outlines, or other browser engines. WebGPU adapter requests still
returned no usable adapter; renderer selection remains open.

Raw records and screenshots are in the
[software report](results/notifications-software-2026-11-01.json) and
[Intel report](results/notifications-intel-2026-11-01.json). The reports identify base
commit `4c3c8ee`; they measure the notification implementation's working tree, with
exact app and executable fingerprints. Source-manifest fingerprints match the baseline.

## Correctness and mutation evidence

The [notification contract](../../docs/prototype/CHANGES.md) separates native delivery,
HTTP adaptation, and browser consumption. Native subscriptions retain one latest
metadata value. SSE carries no source/fact payload and requires no replay log. A fresh
connection receives current state; snapshot-pinned view requests fail explicitly if
their version expired. Obsolete responses cannot reconcile the current selection.

Tests cover coalescing, independent and late subscribers, cross-task wakeup, duplicate
suppression, terminal closure, status ordering, decimal revision precision, malformed
messages, cancellation, reconnection backoff, and pinned views. An actual Chromium
test stops a host with a connected stream, edits source while offline, restarts on the
same port, and verifies new source and the original declaration selection. Graceful
shutdown must exit normally; the harness fails if it needs to kill the host.

Formatting, **81 Rust tests**, doctest checks, Clippy with warnings denied, and
**28 Node/browser checks** pass. Mutation runs use complete affected modules rather
than only changed lines, with passing unmodified baselines and no exclusions:

| Scope | Generated | Assertion failures | Unviable | Timeouts | Survivors |
| --- | ---: | ---: | ---: | ---: | ---: |
| Native changes, application, and HTTP/diagnostic modules | 52 | 26 | 26 | 0 | 0 |
| Browser controller and reconnection adapter | 313 | 297 | 0 | 16 | 0 |

See the [native mutation record](results/notifications-native-mutation-2026-11-01.json)
and [browser mutation record](results/notifications-js-mutation-2026-11-01.json).
Unviable generated replacements provide no behavioural evidence. Timeouts are distinct
from assertion failures. Canvas/CSS, the native watcher scheduler and CLI signal handler,
and benchmark orchestration are outside these mutation scopes; live browser tests also
exercise the shutdown and restart path. Mutation results are evidence, not proof.

## Next architectural work

Notification delivery now has a small native interface that future editor adapters can
consume independently of SSE. The next indexing improvement is to carry dirty-path
hints into verified file capture, retaining full reconciliation for ambiguous or missed
events. Whole-scope reads and snapshot construction remain visible in the synthetic
update observations. Any such optimisation should be checked against a clean capture,
including same-size edits, atomic replacement, deletions, and superseded work.
