# First application measurements

The first native application loads the pinned lpc-rs Rust scope in **346–373 ms with
an empty analysis cache**, or **27–34 ms with persisted analysis**. Three live edits
produced coherent diagnostic view responses in **48.7–49.3 ms**. These are initial
native measurements; browser presentation and large mixed-language workspaces remain
unqualified.

## Workload and method

Run on 2026-10-31 with Rust 1.99.1, four parser workers, an Intel i7-13700H, and
approximately 62.5 GiB RAM. This machine exceeds the 16 GB laptop baseline and exposes
no hardware GPU to the sandbox. Source revision:
`8d3b1f54b36eae33f689bae3e42c716647000e4d`.

The driver copied the pinned archive, discovered **551 Rust files / 7,562,179 bytes**,
and left the original source untouched. It did not analyze LPC or execute source-project
build scripts. OS file caches were warm. Each of three cold runs used a separate empty
analysis cache; three warm runs started fresh processes against persisted facts. See
[the procedure](README.md) and [raw results](results/lpc-rs-2026-10-31.json), which record
the executable SHA-256 and analyzer context fingerprint.

| Operation | Observed time | Analysis work |
| --- | ---: | --- |
| Empty-cache capture through snapshot creation | 346.4–372.5 ms | 551 files parsed, facts extracted and cached |
| Empty-cache process start through exit | 352.8–379.9 ms | Includes host startup and teardown |
| Persisted-cache capture through snapshot creation | 27.2–34.4 ms | 551 cache hits, zero parses |
| Persisted-cache process start through exit | 34.5–42.3 ms | Fresh process, including teardown |
| In-memory no-op reconciliation, one sample | 19.9 ms | 551 documents reused, zero parses |
| One-file edit through snapshot creation | 17.2–20.0 ms | One parse, 550 documents reused |
| File write through observed publication | 45.1–46.2 ms | Includes debounce and status polling by the driver |
| File write through coherent view receipt | 48.7–49.3 ms | Includes diagnostic transport and view serialization |

Cold one-shot peak RSS was **37.2–37.5 MiB**, persisted-cache one-shot peak was
**25.2 MiB**, and the live process reached **33.8 MiB**. Live RSS includes retained
snapshots; unchanged documents share immutable allocations across observations.
`/usr/bin/time` measured one-shot peaks; `/proc` reported the live process high-water mark.

The live probe selected `profiler` in `benches/bench_arithmetic.rs`. Three comment edits
preserved its declaration selection and revealed the updated captured text. A pinned
request continued to return the original source after the first edit. Snapshot identities
matched across every cold and persisted-cache load in this run.

## Correctness evidence

Workspace formatting, 72 Rust tests, doctest checks, Clippy with warnings denied,
and three JavaScript request-controller tests pass. The native CLI test runs the live
stdio host, atomically replaces a source file, and verifies declaration continuity and
pinned source. HTTP handler tests run through the same router in process.

The final mutation run covered complete core model/analyzer/query/publication modules
and native source/cache/engine/application modules: **195 mutations**, **156 caught by
failing tests**, **38 unviable**, **one timeout**, and **zero survivors**. No exclusions
were added. The timeout removes a parser-traversal termination check, producing a
nonterminating parse; it is counted separately. Unviable generated replacements did
not build and provide no behavioral-coverage evidence. See the
[mutation summary](results/mutations-2026-10-31.json) for scope and non-caught cases.

Earlier passes exposed missing assertions for newline boundaries, layout packing,
coverage, and refresh metadata. Those tests were strengthened before the final run.
Scheduler, transport, and browser mutations remain outside this scope; macro-generated
identity code is also outside cargo-mutants' generated mutations.

## Implications and limits

Content/context cache reuse and immutable source publication work together at this
scope. The initial host still reads and hashes the whole discovered Rust scope on every
refresh; it does not yet use dirty paths to avoid unchanged reads. Changed files receive
a fresh syntax parse. The earlier targeted-manifest and incremental-Tree-sitter
experiments are not wired into this application slice.

The browser polls status every 500 ms while visible, so native response timings cannot
be quoted as edit-to-visible latency. Labels, picking, pan/zoom, source virtualization,
background parsing interference, and frame-time tails need actual browser measurements.
The current sandbox blocks listeners and Chromium IPC. Handler and controller tests
do not validate Canvas painting, browser layout, accessibility, or GPU rendering.

The Canvas2D reference view does not settle the production renderer. Next qualification
should compare labeled views and interaction under background work on the baseline
integrated-GPU laptop. Rust/Wasm/wgpu remains a candidate; its browser target has not
been built here. Linux C and PyTorch mixed-language application indexing are future
adapter work, separate from their existing parsing experiments. Three samples per
operation do not establish latency tails or general scaling guarantees.
