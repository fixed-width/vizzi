# Function-history and captured-call qualification

Contributor observations for [issue #5](https://github.com/fixed-width/vizzi/issues/5),
collected on 2026-11-16 Pacific / 2026-11-17 UTC. The
[procedure](README.md) reproduces the workloads; [raw evidence](results/) and
[condition-specific summaries](results/summary.json) retain individual samples.

History, adjacent comparisons, source paging and captured same-file calls completed on
the selected Linux, PyTorch and lpc-rs functions. The full Linux process reached about
4.33 GiB sampled RSS. This is an observation on a 62.5 GiB host, not qualification of the
16 GiB laptop baseline. No latency budget is established from these results.

Full-corpus function lookup omitted all selected Linux/PyTorch files before history could
be entered through Add bubble. Native outline identities allowed the endpoint measurements
below. This user-facing reachability defect is published as
[issue #12](https://github.com/fixed-width/vizzi/issues/12), with its
[reproduction and coverage](FINDINGS.md). These measurements do not qualify the complete
browser workflow on those full repositories.

## Inputs and environment

| Input | Pinned revision | Captured files | Read input bytes | Available Git history |
| --- | --- | ---: | ---: | --- |
| Linux | `cbbddb6b08cc9900a87cccaa9b22bbe06aa4f50a` | 66,069 | 1,495,504,526 | Ordinary shallow checkout, depth request 128; 177,683 reachable commits |
| PyTorch | `1f4d2d5e820dd870626dca4f9603e3a49879dbde` | 10,620 | 186,250,450 | Ordinary shallow checkout, depth request 8,192; 8,192 reachable commits |
| lpc-rs | `f92dd9826537d3d503c9d08586df37c9a3e1275e` | 579 | 7,772,804 | Complete local checkout; 3,910 reachable commits |

Input bytes include selected project metadata. Linux used explicit raw C assignment with
the eight C++ overrides and exclusions in the existing full-corpus policy. PyTorch used
raw C++ assignment with its 194 C overrides; 435 captured files were source-only. These
assignments do not identify an active compiler/preprocessor configuration. Submodules
were not expanded. Default Rust selection in lpc-rs does not classify LPC files as C.

Hardware was an Intel Core i7-13700H (14 cores / 20 logical CPUs), 65,508,036 KiB physical
RAM, AC connected, Linux Mint 22.3, x86-64 kernel `6.8.0-149-lowlatency`. CPU affinity was
0–19; CPU frequency and OS page caches were not controlled. The desktop remained running.
GPU rendering was not part of these observations.

The normal release host at Vizzi `d9fefdf` was built with Rust 1.100.0, LLVM 23.1.5:

```sh
CARGO_BUILD_JOBS=1 nice -n 10 cargo build --release --locked --offline -p vizzi
```

All measured real-repository processes used this same executable, SHA-256
`5eafc8bb5c78bba84b876e8f6b8fb81250909e48db57e2ceaa52f50e031dc3a3`, four analysis
workers and nice 10. Optional syntax/input/spool allowances remained at their default zero;
no language server was configured. The quota fixture used one worker. Exact native
arguments, expanded source policy, toolchain, runtime, timestamps and host memory metadata
are in each raw report. No inspected project was built, imported or executed.

Original repository heads, shallow boundaries and tracked status were checked before and
after and remained unchanged; see [preservation evidence](results/original-preservation.json.gz).
Measurements read disposable copies. Ordinary Git inputs had no missing reachable objects;
Linux's check visited 1,776,307 objects. Acquisition finished before the successful timing
runs, and no builds/tests or other qualification runs overlapped them.

## Workloads and measurement conditions

| Function | Current declaration lines | Current file bytes | Target syntax coverage | File revisions available / accepted earlier function versions |
| --- | ---: | ---: | --- | --- |
| Linux `mm/mmap.c::do_mmap` | 242 | 51,310 | Partial | 10 / 6 |
| Linux `kernel/sched/core.c::__schedule` | 184 | 302,556 | Partial | 48 / 10 |
| PyTorch `torch/_dynamo/convert_frame.py::_compile` | 681 | 107,930 | Syntax | 30 / 12 |
| PyTorch `torch/nn/modules/module.py::Module._call_impl` | 133 | 128,621 | Syntax | 15 / 3 |
| PyTorch `torch/csrc/autograd/engine.cpp::Engine::execute` | 115 | 74,197 | Syntax | 14 / 1 |
| PyTorch same file, `Engine::evaluate_function` | 178 | 74,197 | Syntax | 14 / 0 |
| lpc-rs `src/interpreter/task/mod.rs::Task::run_entry` | 200 | 73,875 | Syntax | At least 65 / 8 |
| lpc-rs `src/interpreter/program.rs::Program::lookup_function` | 9 | 18,318 | Syntax | At least 65 / 6 |

Function versions count the first returned earlier version plus successful deeper steps
in one session, not every file revision or every duplicate session. Counts were consistent
across processes. Linux's partial grammar recovery remains a coverage limitation in the
historical call results; it is not compiler-level analysis.

Each workload ran five times in each of three fresh native processes: three first passes
and twelve later passes. Each pass started history, revisited accepted index 0, requested
the first/next/final source sections, explored up to three captured-call hops, created an
adjacent comparison, paged both sides and explored one hop from each side. Candidate choice
was the first eligible native candidate, including explicit choices from ambiguous sets.
Calls were fully paged at eight per request; unresolved definitions were also probed.
The C++ changed-function supplement used three additional processes with the same full
PyTorch source policy. The unchanged C++ function remains a separate boundary control.

Deeper traversal continued to the native boundary. Later requests revisited accepted
captures and read retained windows after changing selection. For `Engine::execute`, only
index 0 existed: the scripted later index-1 request was another boundary attempt, not a
failed revisit of an accepted version. It is retained as a 503 refusal, outside successful
revisit statistics.

“First” means the first workload pass for that function in a process. OS caches were warm
from preparation. Comparisons followed history and call exploration; paging followed
initial framing. These are not independent cold-disk or cold-parser trials for each API.
The seeded/empty analysis-cache state is recorded separately from owner/process lifetime.

Timings include loopback HTTP, server work/serialization and reading the complete response.
They exclude parent JSON decoding, Git byte-oracle checks, initial indexing and browser
layout/painting. The source oracle checked exact snapshot/file/version pins and literal
UTF-8 fragments, byte columns and original line endings against separate Git reads.
It does not independently prove diff semantics or call resolution; correctness fixtures
remain separate evidence.

## Successful operation timings

Median milliseconds. History and comparison columns have **3 first / 12 later** samples;
accepted index-0 revisits have **15** samples. Only HTTP 200 responses enter these values.
Individual durations, counts, maxima, conditions and refusals are retained in
[operations.csv](results/operations.csv) and [summary.json](results/summary.json).

| Function | First history | Later history | First comparison | Later comparison | Accepted revisit |
| --- | ---: | ---: | ---: | ---: | ---: |
| Linux `do_mmap` | 168.40 | 158.15 | 184.19 | 176.81 | 10.35 |
| Linux `__schedule` | 425.03 | 508.44 | 565.81 | 523.20 | 23.61 |
| PyTorch `_compile` | 259.77 | 276.64 | 263.72 | 294.99 | 8.92 |
| PyTorch `_call_impl` | 271.68 | 267.88 | 277.80 | 272.12 | 8.44 |
| PyTorch `Engine::execute` | 337.40 | 388.72 | 377.86 | 397.78 | 10.04 |
| lpc-rs `run_entry` | 53.68 | 53.24 | 68.20 | 62.29 | 9.64 |
| lpc-rs `lookup_function` | 95.92 | 96.43 | 99.42 | 110.28 | 6.67 |

First passes were not uniformly slower. The largest successful first history observation
was 1,728.75 ms for `__schedule`; its later-pass maximum was 550.01 ms. Three first passes
are insufficient to estimate tail latency or attribute that outlier to a particular cause.

The following medians combine first and later passes. History windows have 45 samples per
function, except the short `lookup_function` with 30; comparison windows have 30. Call
metadata counts vary with declaration size and available destinations. First/later splits
for every stage remain available in the JSON summary.

| Function | History window | Pair window | Calls page | Definition | Destination open (sample count) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Linux `do_mmap` | 1.15 | 1.48 | 1.50 | 1.40 | 1.49 (45) |
| Linux `__schedule` | 1.36 | 2.13 | 7.68 | 7.55 | 8.00 (60) |
| PyTorch `_compile` | 0.99 | 1.48 | 1.99 | 1.98 | 2.02 (45) |
| PyTorch `_call_impl` | 1.11 | 1.54 | 1.99 | 1.88 | 2.47 (60) |
| PyTorch `Engine::execute` | 1.45 | 1.62 | 2.07 | 1.99 | 2.32 (75) |
| lpc-rs `run_entry` | 1.22 | 1.51 | 2.26 | 2.05 | 2.19 (60) |
| lpc-rs `lookup_function` | 1.36 | 1.39 | 1.28 | 0.99 | No same-file destination |

Destination-open timings cover that endpoint, not the entire multi-request trail or a
browser click-to-paint interval. Calls enumeration, definition queries and actions add work.
`Engine::execute` produced 45 ambiguous, 30 unique and 60 unresolved definition responses;
ambiguity retained explicit native choices. All 45 `lookup_function` definition probes
were unresolved, so there is no destination-open timing for that workload. Successful HTTP
definition responses do not imply that a destination was resolved. No cross-file historical
resolution or project type analysis was supplied.

## Initial indexing, memory and live coexistence

Initial indexing is outside every operation timing above. Values end at the first observed
ready status with approximately 100 ms polling. The lpc-rs status measurements had already
advanced to a no-op reconciliation; their reuse counters are not initial parse counts.

| Condition | Empty analysis cache, seconds | Seeded analysis cache, seconds | Peak sampled native RSS, MiB |
| --- | ---: | --- | ---: |
| Linux | 87.95, separate index-only observation | 7.19 / 7.99 / 7.89 | 4,436.2, measured operation processes |
| PyTorch main | 17.70 | 1.42 / 1.31 | 1,079.1 |
| PyTorch C++ supplement | 18.53 | 1.42 / 1.52 | 1,066.0 |
| lpc-rs | 0.93 | 0.10 / 0.10 | 162.9 |

The Linux cold index published successfully, then the collector stopped on an ambiguous
path-filter selection. It is retained as an **index-only** observation, not a history trial.
After fixing literal selection, three fresh operation processes reused its analysis cache
through the explicit `--cache` argument. Both exact collector sources are archived and
their SHA-256 values match the corresponding reports.
The separate Linux cold-index process peaked at 4,351.4 MiB sampled RSS.

During operation/capture admission, background requests read status and a pinned live
source window approximately every 100 ms. Every response was 200 with the same live
snapshot: 413 requests per endpoint for Linux, 471 for main PyTorch, 165 for the C++
supplement and 181 for lpc-rs. Median / maximum milliseconds:

| Condition | Status | Pinned live source window |
| --- | ---: | ---: |
| Linux | 0.70 / 2.58 | 4.04 / 8.87 |
| PyTorch main | 0.65 / 2.65 | 1.05 / 3.13 |
| PyTorch C++ supplement | 0.66 / 2.37 | 1.09 / 3.04 |
| lpc-rs | 0.69 / 4.18 | 0.74 / 2.04 |

The first function's first process admitted 25 extra history sessions and 13 extra pairs,
then checked expired source and call bindings. RSS at the last history / pair admission
was 4,350.1 / 4,387.3 MiB for Linux, 1,072.7 / 1,077.3 MiB for PyTorch and
158.9 / 161.2 MiB for lpc-rs. These small selected files reached count limits; the separate
fixture below exercises byte limits.

RSS was sampled every approximately 20 ms and at operation completion. Maxima are sampled
lower bounds; process `VmHWM` includes indexing/preparation and cannot identify a phase's
transient cost. Native RSS includes the live source corpus, facts, projections, parser and
allocation overhead and temporary captures. It excludes Git child-process memory, the
collector and browser. The independent **32 MiB history** and **32 MiB pair** source-byte
allowances do not bound total memory. These measurements do not isolate each overhead or
emulate a 16 GiB machine. The responsiveness probe adds load and measures native HTTP
service, not rendered frame rate.

## Boundaries, refusals and coverage

The [isolated quota fixture](results/limits.json.gz) passed 143 full-history operations
and one shallow-history operation with the release host and one worker:

- A current file of 2 MiB was admitted; 2 MiB plus one byte was refused with 503 by
  history and comparison.
- Sixteen retained 2 MiB history files remained readable at 32 MiB; the seventeenth
  expired the first binding with 409, before reaching the 24-session limit.
- Twenty-four small sessions remained admitted; the twenty-fifth expired the first.
- Eight pairs of two 2 MiB files remained readable at 32 MiB; the ninth expired the first.
- Twelve small pairs remained readable; the thirteenth expired the first.
- Both window and captured-call queries for evicted owners returned 409. No current-source
  bytes were substituted.
- A 68-revision fixture stopped at the 64-file-revision allowance with 503 and retained
  the earlier accepted source. A separate depth-one clone reported the shallow boundary.

Sampled RSS peaked at 63.11 MiB in the full fixture and 16.87 MiB in its shallow clone.
The large files use comments around tiny Rust declarations; this is quota evidence, not
a worst-case parser/memory model or a source of latency budgets.

Real-workload refusals remain outside successful timings:

- Linux: six shallow-boundary discoveries, plus five controlled expiry responses.
- Main PyTorch: 36 shallow-boundary responses, including 15 history and 15 comparison
  refusals for unchanged `Engine::evaluate_function`, plus five controlled expiries.
- C++ supplement: six shallow-boundary responses, including the three scripted index-1
  attempts after the only accepted earlier version.
- lpc-rs: three missing/ambiguous-declaration boundaries for `run_entry`, three
  64-file-revision boundaries for `lookup_function`, and five controlled expiries.

An earlier setup used partial/promisor Git inputs and produced 20 native 503 refusals;
prefetching selected blobs did not make that input supported. That incomplete attempt
also stopped on the unqualified C++ name. Its [raw report](results/partial-input.json.gz)
is retained separately and contributes no successful timing. Ordinary unfiltered history
replaced those inputs before qualification. No source-ownership or resolution scope was
expanded to accommodate the experiment.

The live inventory admitted only 4,096 files. In PyTorch it omitted 6,520 files and 14,907
symbols, including the selected target files. Kernel targets were also omitted. Captured
outline lookup bypasses this entry-point limitation for native measurements; it does not
repair the browser. [Issue #12](https://github.com/fixed-width/vizzi/issues/12) records the
specific defect. Missing type/configuration analysis, raw C recovery, unresolved calls,
finite first-parent history and omitted global inventory remain distinct coverage limits.
Concurrent owner saturation, physical GPU interaction and long-running edit/recovery
workloads were not performance-qualified here.

## Correctness checks and evidence integrity

These checks ran separately from successful performance collection:

- Formatting and Clippy with warnings denied passed.
- `cargo test --release --workspace --locked --offline` passed **774 tests/doctests**;
  **15 opt-in tests were ignored**, with no failures.
- Existing packaged Chromium history, comparison and captured-call workflows passed,
  including live coexistence, original copying, paging, cancellation and restoration.
  The complete browser suite was not rerun for this experiment.
- Six collector tests passed. Focused failures were observed before implementation and
  for the literal Linux-path regression. Seven manual Python mutations were all caught
  by failing assertions after a passing baseline; there were no timeouts, invalid builds
  or equivalent exclusions in that selected set. This is targeted collector evidence,
  not complete mutation coverage. Product Rust code was unchanged, so native mutation
  runs were not repeated; prior evidence remains in the history/comparison/call contracts.
- All **5,064 real-workload operations** were audited. Non-200 responses were the explicit
  boundaries and expiries above. All **2,460 live probe responses** were 200 with unchanged
  snapshot pins. Source-oracle checks passed throughout completed trials.

Browser checks used packaged Node 26.13.0 and Playwright 1.64.1, with these commands
(each prefixed by `CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10`):

```sh
node_modules/node/bin/node experiments/browser/function-history.test.mjs --no-build
node_modules/node/bin/node experiments/browser/function-comparison.test.mjs --no-build
node_modules/node/bin/node experiments/browser/captured-calls.test.mjs --no-build
```

[Validation logs](results/validation/) retain the baseline, browser checks, collector
regressions/mutations and acquisition/setup failures separately. `collector-initial.py.gz`
is the exact collector for main PyTorch and the Linux index-only observation; it contains
the subsequently fixed literal-filter-selection bug. `collector-final.py.gz` is the exact
collector for Linux operation trials, the C++ supplement and lpc-rs. The current procedure
uses the fixed collector. Artifact hashes are indexed in [manifest.json](results/manifest.json).
