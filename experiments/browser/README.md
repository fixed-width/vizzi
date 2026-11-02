# Validate browser interaction

Use this procedure to exercise the real browser UI and measure its reference renderer
while the native engine refreshes source. It builds Vizzi itself; analysed projects are
never built. Browser results and their limits are explained in [the report](RESULTS.md).
The latest delivery measurements are in [the notification report](NOTIFICATIONS_RESULTS.md).
Full Linux/PyTorch interaction results and current outline/edit improvements are in the
[responsiveness report](../qualification/RESPONSIVENESS_RESULTS.md). The
[retention report](../retention/RESULTS.md) adds a same-build enabled/disabled browser
comparison and documents the remaining periodic-recovery outlier.

## Install and check

From the Vizzi checkout:

```sh
npm ci
npm test -- --install-browser
npm test
```

The lockfile pins Playwright 1.64.1, StrykerJS 10.1.1, and a stable Node 26.13.0
development runtime. The scripts use the local runtime without changing the global
Node installation. A modern global Node avoids bootstrap engine warnings from Babel;
the tested runtime satisfies its requirements. These are development dependencies;
the compiled application serves local assets without Node or a package server.

The browser installer downloads [Playwright's matching Chromium builds](https://playwright.dev/docs/browsers). Do not use
`--ignore-scripts` when installing the packaged Node runtime. An existing current Rust
toolchain and cached Cargo dependencies are required: tests build the release host
with `cargo build --release --offline -p vizzi`.

`npm test` runs request-controller tests, timing-summary tests, outline DOM tests, and browser checks
against a real native HTTP host on an ephemeral loopback port. Tests cover readable
labels, transformed map picking, linked source/outline selection, reader scroll
continuity, source virtualisation, narrow layout, unsupported encoding, syntax recovery,
literal source text, map resizing when the file list opens, and live-stream recovery
after restarting a host with connected clients. The client makes no status-polling
requests. Temporary source/cache directories are
removed after each run.
The standalone outline tests use real Chromium without the native host; an integration
fixture also checks 50,000 declarations through the actual HTTP/source-selection path.

## Check the actual rendering environment

```sh
npm test -- --capabilities
npm test -- --capabilities --headed
```

The headed command requires an accessible desktop display. Compare the reported GPU
device and feature status. The headless build can select SwiftShader on a machine whose
headed browser uses hardware acceleration. A generic sandboxed shell can also hide
device nodes that an approved browser-test process can access; use the test process's
own results to describe its capabilities.

## Collect interaction measurements

```sh
npm test -- --measure \
  --source /tmp/vizzi-perf-corpus/lpc-rs \
  --revision 8d3b1f54b36eae33f689bae3e42c716647000e4d \
  --output /tmp/vizzi-browser-software.json
```

Repeat with `--headed` and a different output path for the desktop browser. The driver
always generates a 10,008-file Rust workspace with a 2,000-tile overview; `--source`
adds a reference-repository copy. Its originals remain untouched. Supplied source
directories should be pinned archives without build outputs. The reported manifest
fingerprint hashes normalised paths and source-content SHA-256 values before edits.

Each workload gets three four-second steady-state runs and three runs that change a
batch of Rust files during pan/zoom. Periodic native reconciliation stays active in
both conditions. The driver submits trusted Playwright input at a target 60 Hz with
bounded requests in flight. Raw submission and completion times expose actual cadence.
It records independent rAF intervals, observed draw intervals, CPU draw submission,
status-observed pending intervals, label counts, picking costs, heap samples, native
RSS, browser request counts, and five single-file update observations. The driver still
queries status from Node to classify pending periods; this is separate from the
browser client, which consumes change notifications. Screenshots accompany the JSON report.

Use `--samples 1 --duration 1000` for a harness smoke test. Durations are milliseconds;
allowed ranges are 1–10 samples and 1,000–30,000 ms. Existing reports and screenshots
are never overwritten. Local scratch results belong in `results/local/`, which Git
ignores. Performance runs should have no competing tests, mutation jobs, or benchmarks.
Every run generates a UUID recorded as `options.probeId` in source-edit comments, so
persisted caches cannot reuse a prior run's probe content. `--probe-id` overrides it
for deliberate replay; use a new value when measuring fresh parsing.

These timings do not establish physical presentation or GPU execution latency. The
write-to-update observation ends after matching visible source DOM and two subsequent
rAF callbacks. Five such samples do not establish tail latency. A backend feature flag
is also insufficient to establish a usable WebGPU adapter: the report records an
actual adapter request separately.

## Run focused mutation testing

```sh
npm test -- --mutations
```

Stryker tests the complete request-controller and live-update adapter modules using
the Node behavioural tests.
The unmodified baseline must pass first. Its threshold rejects surviving mutants;
timeouts are still reported separately from assertion failures. The
[command runner](https://stryker-mutator.io/docs/stryker-js/configuration/#commandrunner-object)
does not provide per-test coverage mapping. JSON output goes to
`results/local/controller-mutation.json`. Painting, CSS, the native engine, and benchmark
orchestration are outside this mutation scope; browser regressions have their own tests.

For the complete virtual outline module:

```sh
npm test -- --outline-mutations
```

This runs the real-Chromium DOM tests against instrumented `outline.js`, using the
actual stylesheet. The harness explicitly forwards Stryker's active-mutant constants
into the page before importing the module; a Node environment variable alone cannot
activate browser mutations. Reports go to `results/local/outline-mutation.json`.
CSS, app orchestration, native HTTP integration, and benchmark code are outside this
mutation scope. Its final results and investigated gaps are recorded in the
[responsiveness report](../qualification/RESPONSIVENESS_RESULTS.md).

For native delivery and its HTTP adapter:

```sh
cargo mutants -p vizzi --no-config \
  --file 'crates/vizzi/src/{changes,application,server}.rs' \
  --jobs 4 --timeout 15 --build-timeout 180 -- --lib
```

This scope covers initial delivery, coalescing, closure, stale publication, pinned views,
and diagnostic limits. Tests exercise complete modules; unviable generated replacements
are recorded separately from assertion failures.

The existing [native checks](../prototype/README.md#verify-correctness) remain required.

For the complete window-reader/controller mutation scope, run:

```sh
npm test -- --window-mutations
```

This uses Node behavioral tests and writes `results/local/window-mutation.json`.
Inspector DOM painting/CSS are outside that mutation scope; native HTTP/browser
regressions exercise navigation, pinning, fragments, and live updates. The
[windowed report](../windows/RESULTS.md) records scopes, limits, and response-byte audits.
