# Language adapter measurements

Rust and Python now use one source policy and one bounded analysis queue. Native
indexing of PyTorch's selected Python scope took **6.09–6.34 seconds** with four
workers; fresh-process analysis-cache loads took **339–389 ms**. Python edits required
one read and one parse, and every incremental result matched a complete capture.

## What changed

`LanguageAdapter` defines extension selection, a frozen context, and a worker-local
analyzer factory. The native engine dispatches both adapters through one largest-file-first
queue; it creates parsers lazily and reuses them within an attempt. Full discovery,
dirty capture, event classification, cache validity, and dispatch share the registry.
Output is validated before persistence. The [language reference](../../docs/prototype/LANGUAGES.md)
defines capabilities and failure behavior, including the requirement for full capture
after replacing a source policy in native callers.

Python classes, methods, nested/async functions, byte spans, and lexical identities
appear through the existing source/outline interface. The browser identifies the
selected language and no longer calls every discovered file Rust. This validates a
second adapter without settling C/C++ preprocessing, header contexts, or CUDA dialects.

## Native evidence

Measurements use Rust release builds, four workers, warm OS caches, disposable source
copies, and the same [native diagnostic procedure](README.md). The machine is the
i7-13700H host with approximately 62.5 GiB RAM used in the preceding experiments.
Each condition has three samples; timings are ranges, not tail percentiles.

| Selected source scope | Files / bytes | Empty analysis cache | Fresh-process persisted cache | One-file engine attempt | Write to coherent diagnostic response |
| --- | --- | --- | --- | --- | --- |
| lpc-rs Rust + Python | 552 / 7,563,695 | 354.84–360.83 ms | 23.28–26.54 ms | 1.34–1.43 ms | 30.86–32.02 ms |
| PyTorch Python | 5,077 / 120,363,731 | 6,086.67–6,339.03 ms | 338.92–389.01 ms | 6.92–12.78 ms | 42.57–57.16 ms |

lpc-rs is pinned at `8d3b1f54b36eae33f689bae3e42c716647000e4d`: 551 Rust
files and one Python file, with LPC excluded. PyTorch is pinned at
`1f4d2d5e820dd870626dca4f9603e3a49879dbde`: its C++, CUDA, and header files
are excluded. Source inventories, their hashes, binary hashes, full status measurements,
and clean-capture checks are in [the native report](results/native-2026-11-01.json).
The measured tree was based on `746962d` with the implementation uncommitted.

PyTorch's settled in-memory full verification took 165.01 ms and read all 5,077 files;
targeted edits read one file and retained 5,076 documents. Initial cold runs parse
4,907–4,914 files and reuse 163–170 content-identical analyses written earlier in the
same attempt. Those counts vary with worker scheduling; snapshot identity is unchanged.
The live PyTorch host peaked at 402.1 MiB RSS; release one-shot cold processes peaked
at approximately 341.8–342.1 MiB. These are process measurements, not a memory limit.

The lpc-rs edit times remain close to the [cached-version baseline](../incremental/FINGERPRINT_RESULTS.md).
Its source scope now includes the extra Python file. The small cold-load difference
does not establish a regression or attribute a cost to dynamic dispatch.

## Correctness and mutation evidence

Formatting, 114 workspace Rust tests, doctests, Clippy, and 29 Node/browser checks pass.
Tests cover mixed-language cache reuse, targeted edits, rename/removal, configuration
invalidation, factory/parser failure, malformed output, and invalid cache context metadata.
Instrumented mixed adapters demonstrate peak active analysis of exactly 1, 2, and 4
for those worker settings, identical snapshots, and no parser creation on warm hits.

The complete seven-module mutation run passed its unmodified baseline and tested 171
mutations: **143 caught, 25 unviable, three timeouts, zero survivors**. It includes both
crates' integration tests and the native live CLI tests. The timeouts suppress initial
publication (`refresh_scoped` returning `None`, or cancellation always true/inverted),
leaving host-startup tests waiting. They are separate from assertion-detected mutations.
One existing equivalent directory-prefix comparison is excluded; no new exclusions
were added. [The mutation report](results/mutation-2026-11-01.json) records the command,
scope, exclusions, and all outcomes that were not caught by assertions.

## Limits

This is a staged syntax integration. It does not analyze PyTorch's complete architecture,
cross-language calls, imports, types, generated bindings, or build configurations.
The Linux kernel's C integration remains future work. Python raw grammar acceptance
does not establish runtime correctness; non-UTF-8 sources remain unavailable analysis.

These native response measurements exclude browser rendering, GPU work, and physical
presentation. The successful browser run checks coordination and live updates on a
small mixed fixture. No new headed/GPU timing run or 16 GB laptop qualification was
performed. The worker count caps active analysis per refresh; captured input bytes,
parser memory, and direct concurrent callers do not have a process-wide memory budget.
An individual long parse is still not interruptible. Hash macro expansion, dependency
internals, and actual filesystem delivery are outside this mutation scope.
