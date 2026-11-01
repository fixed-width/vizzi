# Validate browser interaction

Use this procedure to exercise the real browser UI and measure its reference renderer
while the native engine refreshes source. It builds Vizzi itself; analysed projects are
never built. Browser results and their limits are explained in [the report](RESULTS.md).

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

`npm test` runs request-controller tests, timing-summary tests, and browser checks
against a real native HTTP host on an ephemeral loopback port. Tests cover readable
labels, transformed map picking, linked source/outline selection, reader scroll
continuity, source virtualisation, narrow layout, unsupported encoding, syntax recovery,
literal source text, map resizing when the file list opens, and connection recovery. Temporary source/cache directories are
removed after each run.

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
RSS, and five single-file update observations. Screenshots accompany the JSON report.

Use `--samples 1 --duration 1000` for a harness smoke test. Durations are milliseconds;
allowed ranges are 1–10 samples and 1,000–30,000 ms. Existing reports and screenshots
are never overwritten. Local scratch results belong in `results/local/`, which Git
ignores. Performance runs should have no competing tests, mutation jobs, or benchmarks.

These timings do not establish physical presentation or GPU execution latency. The
write-to-update observation ends after matching visible source DOM and two subsequent
rAF callbacks. Five such samples do not establish tail latency. A backend feature flag
is also insufficient to establish a usable WebGPU adapter: the report records an
actual adapter request separately.

## Run focused mutation testing

```sh
npm test -- --mutations
```

Stryker tests the complete request-controller module using the Node behavioural tests.
The unmodified baseline must pass first. Its threshold rejects surviving mutants;
timeouts are still reported separately from assertion failures. The
[command runner](https://stryker-mutator.io/docs/stryker-js/configuration/#commandrunner-object)
does not provide per-test coverage mapping. JSON output goes to
`results/local/controller-mutation.json`. Painting, CSS, the native engine, and benchmark
orchestration are outside this mutation scope; browser regressions have their own tests.

The existing [native checks](../prototype/README.md#verify-correctness) remain required.
