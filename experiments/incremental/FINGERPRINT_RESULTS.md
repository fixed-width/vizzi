# Cached document-version measurements

Immutable document versions remove repeated fact serialisation from snapshot assembly.
Four targeted edits in the generated workload took **4.46–4.78 ms per engine attempt**,
compared with **29.39–33.38 ms** in the [dirty-capture baseline](RESULTS.md).
The software browser's median source-update observation fell from **109 ms to 87 ms**.
Fresh full captures agreed with the incremental results.

## Change and identity rules

`Document::capture` validates the captured bytes and analysis records, then computes a
cached `DocumentVersion` from file identity, content identity, and the ordered analysis
digest. Its private fields remain immutable. Reused `Arc<Document>` values bring their
versions into the next snapshot without serialising their facts again.

Snapshot composition version 2 hashes the workspace and path-sorted document versions.
Duplicate-path and workspace validation remains. The [identity reference](../../docs/prototype/IDENTITIES.md)
defines the inputs and the one-time change to opaque snapshot digest values. Entity
identity rules and the HTTP response schema are unchanged. Model-source fingerprinting
conservatively invalidates prior analyzer/cache contexts; cached document versions are
not persisted or trusted in place of source/fact validation.

## Method

This repeats the [capture procedure](README.md) using Rust release builds, four workers,
warm OS caches, and disposable source copies. The reference revision is lpc-rs
`8d3b1f54b36eae33f689bae3e42c716647000e4d`, with 551 Rust files / 7,562,179 bytes.
The generated workload has 10,008 files / 4,670,156 bytes, including eight files with
16,001 declarations each and 10,000 small files. LPC and analysed-project builds remain
excluded. The original sources are unchanged.

The i7-13700H host has approximately 62.5 GiB RAM. Browser measurements use headless
Chromium 156.0.8078.4 with SwiftShader, Playwright 1.64.1, Node 26.13.0, and a
1440 × 900 viewport. The generated overview is bounded to 2,000 tiles. There is no
new headed/GPU run: the preceding headed frame-count failures remain unresolved.

Browser conditions have three four-second pan/zoom runs and five source edits. The
endpoint is matching visible source DOM plus two rAF callbacks, not GPU execution or
physical presentation. Before/after runs were collected at different times; these
small samples do not establish tail latency or statistically general speedups.

## Engine and browser observations

| Workload | Previous targeted engine observations | New targeted engine observations | Previous browser median | New browser median/range |
| --- | ---: | ---: | ---: | ---: |
| lpc-rs | 4.69–7.72 ms | 0.84–1.50 ms, five edits | 77 ms | 69 ms / 66–79 ms |
| Generated | 29.39–33.38 ms | 4.46–4.78 ms, four edits | 109 ms | 87 ms / 70–92 ms |

The fifth generated-workload edit coincided with full reconciliation: it read all
10,008 files and took **49.71 ms**, with a **40.32 ms** capture phase. It remains in
the five-edit browser distribution. The other four edits read one file and retained
10,007 documents, with approximately **0.92–1.04 ms** input/capture phases. Full
reconciliation is intentionally preserved, not filtered out as a failed observation.

Three separate native diagnostic edits on lpc-rs took **1.25–1.46 ms** per engine
attempt and **30.67–32.45 ms** from write through coherent response. Each read one file
and matched a fresh full capture from a second process. Three empty-analysis-cache
loads took **310–344 ms**; three fresh-process cache loads took **21–30 ms**. The
one-time fingerprint work is included in document capture and those startup timings.

During background refresh, CPU Canvas submission p99 was **1.0 ms** for the generated
view and **0.4 ms** for lpc-rs; independent rAF p99 remained **16.8 ms**. Native process
high-water RSS reached approximately **300 MiB** for the generated workload and
**72 MiB** for lpc-rs. Heap/allocation, cache state, and fingerprint storage all affect
these process peaks; they are not isolated measurements of the added version field.
No baseline-laptop, dense-edge, large-outline, or new renderer qualification is implied.

Raw [native](results/document-version-native-2026-11-01.json) and
[browser](../browser/results/document-version-software-2026-11-01.json) records include
source and executable fingerprints. Source manifests match the baseline. Each final
browser index agrees with a fresh full capture and has zero browser status polls.
The reports identify base commit `5b7c9d6` and this implementation's measured working
tree. Screenshots accompany the browser JSON.

## Correctness and mutation evidence

Tests first demonstrated that a placeholder document version failed field sensitivity.
They now cover content bytes, unsupported encoding bytes, path/workspace identity,
all context fields, diagnostics, coverage, every fact field, ordered contexts, empty
workspaces, invalid inputs, and fresh-capture versus document-reuse equivalence.
The native/browser checks retain pinned reads and selection/reconnection behaviour.

The complete core model, publication, query, and Rust-analyzer modules generated
**118 mutations**: **92 caught by failing tests**, **23 unviable**, **three timeouts**,
and **zero survivors**, with a passing unmodified baseline and no exclusions.
See the [mutation record](results/document-version-mutation-2026-11-01.json).
Macro-generated identity hashing is outside cargo-mutants' generated mutations;
field-sensitivity tests exercise it through captured documents. Timeouts and compile
failures are distinct from assertion failures. No native/browser implementation was
mutated in this run.

Formatting, 107 Rust tests, doctest checks, Clippy with warnings denied, and 28
Node/browser checks pass. No new dependencies or unsafe code were added.

## Next architectural work

Snapshot assembly still sorts/checks all documents and hashes their version identifiers;
it is not a persistent-tree or constant-time update algorithm. At the current scope,
the next substantial step is to introduce a language-analysis adapter interface and
shared resource budget so the application can extend beyond Rust into the Linux and
PyTorch reference workloads. Renderer selection and hardware qualification remain
separate work; these results do not select a production GPU backend.
