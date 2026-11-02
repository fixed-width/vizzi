# Bounded document delivery results

The browser now uses metadata summaries and pinned source/outline windows. In same-build
diagnostic comparisons, Linux's 24 MB header changes from a **24.2 MB bulk response to
a largest tested 20.6 KB window**. PyTorch's 8.4 MB header changes from **15.2 MB to
59.7 KB**. Complete source remains reachable through line fragments and logical outline
navigation. Existing bulk diagnostics remain available explicitly.

The [window contract](../../docs/prototype/WINDOWS.md) defines snapshot/file/version
binding, bounded UTF-8 data, truncation flags, selection, and focus/scroll recovery.
Initial windows are prepared before applying a new linked view; later page requests
use independent abort/tokens so stale replies cannot mutate another document view.

## Native payload comparison

The [driver](README.md) reads unchanged pinned Linux/PyTorch profiles with four workers,
warm OS caches, and separately prepared current-context analysis caches. Preparation is
excluded. The i7-13700H host exposes 20 logical CPUs and about 62.5 GiB RAM; Rust is
1.99.1. Source archives are neither edited nor built. No tests, builds, mutation jobs,
or other benchmarks compete with measurement.

Each file has three bulk requests, repeated summary requests, and start/middle/end
windows bound to the same snapshot and document version. Every returned fragment,
identity, and line range matches the explicit bulk response. Raw header assignments
remain language inputs rather than compiler/configuration coverage.

| File | Bulk bytes | Initial summary bytes | Largest tested window bytes |
| --- | --- | --- | --- |
| Linux `kernel/locking/mutex.c` | 46,387 | 1,456 | 24,112 |
| Linux 24 MB AMD generated header | 24,234,100 | 1,490 | 20,569 |
| PyTorch `c10/core/Device.cpp` | 7,694 | 1,379 | 14,130 |
| PyTorch 8.4 MB Perfetto header | 15,248,720 | 1,509 | 59,676 |

Windows do not always make tiny files smaller: fragment metadata can exceed an entire
small source response. The goal is finite delivery as files and outlines grow. Summary
and window are separate requests; the table does not hide that extra round trip.

Median bulk stdio response times are 13.48, 148.61, 2.72, and 129.16 ms in table order.
Median window times are 2.53, 0.75, 0.27, and 1.10 ms. These include diagnostic envelopes
and Python JSON decoding. They are not HTTP/browser presentation times, and the three
window positions are not identical work to three full-file requests.

Source windows have at most 96 rows and 32 KiB unescaped text, with explicit continuation.
Outline windows have at most 200 entries plus a cursor; names/kinds are at most 128/32
UTF-8 bytes. Adversarial control-character/name tests exercise JSON escaping below
512 KiB serialized document-window bodies. This bound excludes the overview and bulk
diagnostic operations. Identity lookup can construct a lazy whole-document index once
per immutable version; bounded output does not make all query preparation constant time.

[Payload observations](results/payloads.json), [summary](results/payload-summary.json),
and [CSV](results/payload-summary.csv) retain pins, inputs, raw sizes, timings, and checks.
The host's normal watcher/recovery remains active, so scheduling is not isolated.

## Browser interaction and response audit

Both full profiles run with the existing 512 MiB syntax-retention option, prepared
caches, software-rendered headless Chromium 156, and a 1440×900 viewport. Each profile
has three four-second steady and three refresh-motion runs, five ordinary edits, and
one large-header open/edit. Edits affect disposable copies and use unique probe inputs.

| Observation | Linux | PyTorch |
| --- | --- | --- |
| Largest-header opening | 112.0 ms | 88.7 ms |
| Header edit to matching DOM plus two rAF callbacks | 892.0 ms | 744.3 ms |
| Five ordinary edits, median / maximum | 123.2 / 173.9 ms | 104.7 / 301.3 ms |
| Largest recorded browser window body | 17,920 bytes | 58,113 bytes |
| Rendered source rows / outline controls for header | 49 / 0 | 47 / 21 |

Completed browser view responses use `projection=summary`, contain null source text,
and empty full declaration lists. Every observed document-window reply stays below
512 KiB. The driver's own explicit bulk calls for input selection and independent
verification are separate from browser page requests. Aborted requests may have no
body observation. The PyTorch 2,000-tile overview reaches 687,437 bytes; it is a distinct
delivery cost, not a violation of the document-window bound.

All twelve overview runs have a 16.8 ms rAF p99 and no intervals above 50 ms. Draw CPU
p99 is at most 1.0 ms. Neither header opening/editing records a long animation frame;
opening maximum rAF is 16.7 ms and editing maximum is about 16.8 ms. These observations
cover this reference renderer, not GPU execution or physical presentation.

The Linux header edit reports a targeted 723.6 ms engine attempt without tree reuse.
PyTorch's sampled header metadata is a later no-op full recovery, so it is not used
to attribute the causal parse duration. Its 301.3 ms ordinary outlier records full
reconciliation; necessary recovery still dominates some updates. The proposed
all-five-edits-below-200-ms gate therefore remains unmet for PyTorch.

Earlier bulk-browser PyTorch header opening/update were 184/858 ms in the
[shared-source run](../shared-source/RESULTS.md); current values are 89/744 ms. These are
historical observations with changed protocol and analyzer fingerprints, not a randomized
paired latency experiment. The native payload comparison above uses one binary/pin.

Point renderer heap samples are 13.4 MiB Linux and 10.8 MiB PyTorch. Native lifetime
high water is about 2.657 GiB and 0.920 GiB. Heap/DOM samples are not browser-process
peak memory; allocator/garbage collection state and earlier work influence them.
Both reports have empty browser errors and complete-capture agreement:
[Linux](results/browser-linux.json), [PyTorch](results/browser-pytorch.json),
[Linux screenshot](results/browser-linux.png), [PyTorch screenshot](results/browser-pytorch.png).
This host does not qualify the 16 GB baseline laptop. Five edits and one header operation
per profile do not establish tails or a general stutter-free guarantee.

## Correctness and mutation evidence

New native/reader/browser tests first failed before their interfaces or handlers existed.
They cover UTF-8/CRLF/empty/binary source, huge lines/names, exact clipping limits,
multiple contexts, selection outside loaded windows, old source pins, wrong versions,
focus/scroll anchors, removed-focus recovery, logical Home/End navigation, activation
while a page is pending, full long-line access, and literal source rendering.

Reader/controller tests independently reject malformed bindings, types, coordinates,
budgets, ordinals, and stale successes/failures. They exercise transports that ignore
abort, completed-fetch cleanup, and side-effect-free initial preparation. Browser tests
exercise actual native HTTP with 50,000 declarations and a two-megabyte line.
Formatting, **177 Rust tests**, doctests, Clippy, and **56 Node/browser checks** pass.
Eight of those are retained compatibility tests for the old bulk outline adapter;
the new inspector also has real-host integration coverage.

| Complete mutation scope | Tested | Caught | Unviable | Timeouts | Survived |
| --- | --- | --- | --- | --- | --- |
| Core projection/query/source/model and native HTTP handler | 205 | 141 | 63 | 1 | 0 |
| JavaScript window reader and main controller | 543 | 528 | 0 | 15 | 0 |

Passing unmodified baselines precede interpretation. No exclusions were added.
Development survivors exposed missing exact-budget, label, multi-context, focus, and
coordinate assertions. They also exposed a test flaw: a callback assertion was caught
as an expected reader error. Those assertions now run outside callbacks. Redundant
guards/recovery arithmetic were removed instead of adding exclusions.

The Rust timeout breaks UTF-8 boundary progress into a nonterminating loop. JS controller
timeouts are separate from assertion catches. [Native mutation details](results/native-mutation.json)
and [JS details](results/window-js-mutation.json) preserve scopes, commands, source hashes,
non-caught outcomes, and development notes. Inspector DOM painting/CSS, dependency
internals, hash/JSON macro expansion, OS delivery, and benchmark logic are outside these
mutation scopes. Zero survivors is evidence, not a proof of correctness.

The [source/binary manifest](results/source-sha256.json) identifies the working tree
measured before commit. No dependencies or unsafe-code exceptions were introduced.

## Remaining work

Windowing removes repeated bulk text/list transfer and full client outline indexing.
Full analysis, lazy identity indexing, snapshot merging, overview delivery, and periodic
source verification still scale with their input. Shared source storage and private
syntax retention keep their separate limits; there is no new whole-process memory cap.
Window ranges preserve logical inspection and source access, while broader working-set,
hardware, recovery scheduling, and configured-analysis work remain separate decisions.
