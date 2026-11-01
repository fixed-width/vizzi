# Browser interaction measurements

This is the polling-era baseline. Subsequent [notification measurements](NOTIFICATIONS_RESULTS.md)
repeat the workload after replacing browser polling with latest-state subscriptions.

Actual Chromium now exercises the linked reference view with both software rendering
and Intel Iris Xe acceleration. During background refresh, synthetic-workspace CPU
draw-submission p99 was **1.1 ms in SwiftShader** and **1.2 ms in Intel Chromium**,
with a bounded 2,000-tile overview. These do not measure GPU execution or physical
presentation latency.

Update delivery is a clearer current bottleneck: lpc-rs edits reached visible source
DOM plus two rAF callbacks in **132–554 ms**, compared with the earlier approximately
49 ms native diagnostic response. The browser's 500 ms polling interval contributes
substantially to that difference.

## Environment and method

Recorded on 2026-11-01 UTC (2026-10-31 PDT), with Chromium **156.0.8078.4**,
Playwright **1.64.1**, Node **26.13.0**, and the release Rust host using four parsing
workers. The i7-13700H machine has 20 logical CPUs and approximately 62.5 GiB RAM.
It does not establish behaviour on the project's 16 GB baseline under memory pressure.
Playwright used its Ubuntu 24.04 fallback build and reported that the host OS was not
officially supported; the successful checks describe this tested configuration.

The headless build selected ANGLE SwiftShader. Headed Chromium selected
`Mesa Intel(R) Iris(R) Xe Graphics (RPL-P)`, Mesa 25.2.8, with Canvas2D acceleration,
GPU compositing, and rasterisation enabled. The approved test process could access
DRM devices that direct sandboxed shell probes could not see. Earlier claims that the
host could not run a browser or access hardware graphics were too broad; they described
the direct shell environment, not the browser-test workflow.

Headed feature status marked WebGPU enabled, but actual requests obtained no usable
adapter in either mode. That feature flag does not qualify a Rust/Wasm/wgpu browser
backend. No WebGPU or WebGL renderer was implemented or compared.

| Workspace | Rust files | Original bytes | Overview bound | Background edit batch |
| --- | ---: | ---: | ---: | --- |
| Generated Rust | 10,008 | 4,670,156 | 2,000 tiles | 8 files, 128,008 functions, 4,302,376 bytes |
| lpc-rs | 551 | 7,562,179 | 551 tiles | 32 files, 2,535,501 bytes |

lpc-rs revision: `8d3b1f54b36eae33f689bae3e42c716647000e4d`. All edits used
disposable copies. LPC was excluded and no analysed-project builds ran. The generated
workload contains another 10,000 small declarations outside its large files. Each
explicit refresh reparsed all eight or all 32 changed files respectively.

Each condition used three four-second runs, panning for the first half and zooming
for the second. The driver recorded **240–241 submitted inputs per run**, targeting
60 Hz with bounded requests in flight. Initial calibration exposed serial automation
acknowledgements limiting zoom to approximately 20 Hz; the final measurements use
the corrected, paced driver. Startup and initial indexing are outside these timings.
Normal two-second reconciliation stays active in both steady and refresh conditions.

Raw reports contain source-manifest, executable, and app fingerprints, input submission
and completion times, timing samples, memory metrics, and renderer capabilities:
[software](results/chromium-software-2026-11-01.json) and
[Intel](results/chromium-intel-2026-11-01.json). Follow the [procedure](README.md) to
reproduce them. Screenshots show
[Intel lpc-rs](results/chromium-intel-2026-11-01-lpc-rs-workspace.png) and
[the generated workspace](results/chromium-intel-2026-11-01-synthetic-10k-workspace.png).
The recorded Git revision is the base `aae2cd2`; measurements used this change's
working tree, identified by the executable and app fingerprints, rather than a clean
checkout of that base commit.

## Interaction while refreshing

Nearest-rank percentiles combine each condition's three runs. CPU times cover the
JavaScript Canvas command loop. Observed draw intervals record changes to its completed
draw record at successive rAF opportunities, separately from independent rAF scheduling.

| Renderer / workspace | rAF samples | rAF p99 | Draw-interval samples | Draw p99 / maximum | CPU draw p99 / maximum |
| --- | ---: | ---: | ---: | ---: | ---: |
| SwiftShader / generated | 727 | 16.8 ms | 711 | 16.8 / 50.0 ms | 1.1 / 2.4 ms |
| Intel / generated | 729 | 16.8 ms | 715 | 16.8 / 50.1 ms | 1.2 / 2.6 ms |
| SwiftShader / lpc-rs | 728 | 16.8 ms | 714 | 16.8 / 50.0 ms | 0.4 / 1.7 ms |
| Intel / lpc-rs | 729 | 16.8 ms | 714 | 16.8 / 50.0 ms | 0.6 / 2.0 ms |

Steady-state rAF p99 was also 16.8 ms in all four cases. Some observed draw gaps
reached 50.1 ms while independent rAF intervals stayed below 16.9 ms.
Draw-cadence gaps, driver handoff, input coalescing,
and display presentation cannot be resolved from rAF p99 alone. These data do not
establish universally stutter-free rendering.

Refresh conditions included 94/99 rAF intervals classified as pending for the
software/Intel generated workloads and 31/36 for lpc-rs; their p99 was 16.8 ms.
Classification uses approximately 25 ms status observations, not exact worker-phase
traces. These short pending windows do not establish tails under sustained saturation.
Final frames had 130 labelled tiles in generated views and 36 in lpc-rs, with all
overview rectangles available for culling. This does not render 10,008 simultaneous
labels or dense relationship graphs.

Picking had a 0.1 ms maximum at the browser timer's resolution; zero-valued samples
mean below that resolution. Native peak RSS reached approximately **295 MiB** for the
generated workload and **75 MiB** for lpc-rs across both modes. Renderer JS heap samples
were approximately 9–12 MiB. These are point samples affected by GC and reloads, not
browser-process memory peaks.

## Source-update observations

Five edits per workspace/mode used different delays to vary polling phase. Measurements
end after the marker appears in virtualised source DOM and two subsequent rAF callbacks.
They describe a render-ready observation, not physical display presentation.

| Renderer | Workspace | Median | Observed range |
| --- | --- | ---: | ---: |
| SwiftShader | Generated | 475 ms | 288–649 ms |
| Intel | Generated | 402 ms | 150–553 ms |
| SwiftShader | lpc-rs | 402 ms | 132–553 ms |
| Intel | lpc-rs | 403 ms | 132–554 ms |

Five samples do not establish tail latency. The [native report](../prototype/RESULTS.md)
used different endpoints; its approximately 49 ms result is not browser-visible latency.
Native completion, polling phase, request/JSON handling, DOM updates, and frame
scheduling all contribute here.

## Correctness evidence

Eight actual-browser checks and nine Node behavioural/statistical checks pass. Tests
first reproduced missing initial labels, source jumping to the declaration after an
edit, and a connection error that persisted after recovery. A narrow-layout check
also exposed intrinsic canvas-height sizing. The fixes retain the existing workbench
palette and panel arrangement while keeping readable labels and a correctly sized map.
Opening the file list also changed the map viewport without resizing its drawing
buffer; a failing regression test now verifies that both CSS size and backing pixels
follow the actual viewport rather than only the outer layout.

Publication preserves file/declaration selection and reader scroll position. Explicitly
reselecting a declaration reveals it. A failed selection retries after recovery even
if the snapshot is unchanged. Request tokens reject obsolete view/status responses
and stale failures. Source virtualisation, transformed picking, narrow layout, syntax
and encoding limits, and literal source text are exercised in Chromium.

The complete request-controller mutation scope has **55 mutations**, **50 killed by
failing tests**, **five timeouts**, **zero survivors**, and **zero errors**, with a
passing unmodified baseline and no exclusions. Timeouts are separate evidence even
though Stryker includes them in its score. See the
[mutation record](results/controller-mutation-2026-11-01.json). The command runner
invokes seven Node tests without per-test coverage mapping. Canvas/CSS, native code,
and benchmark orchestration are outside this mutation scope. Formatting, 72 Rust
tests, doctest checks, and Clippy pass; no native implementation changed.

## Implications

The bounded, labelled Canvas2D view provides a useful reference on this Intel-equipped
host. Background parsing coexists with interaction at this measured scope. Production
renderer choice, large declaration outlines, dense edges, other browser engines, Wasm,
and the 16 GB baseline under pressure remain unqualified.

The next application change should replace browser polling with versioned change
notifications while retaining snapshot-pinned queries and late-response rejection.
This addresses measured delivery latency and provides a host interface useful to
future editor integrations. Renderer comparison remains open, including establishing
a usable WebGPU adapter before evaluating a wgpu browser implementation.
