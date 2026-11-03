# Verified unchanged-source metadata reuse

Opt-in streamed capture now verifies actual incoming bytes against the previous capture
before constructing candidate metadata. An unchanged input retains its original source,
line index, encoding, integrity digests and backing without a staged metadata read or
committed append. Resident capture remains default. The implementation adds no dependencies
or unsafe-code exceptions.

The [reference](../../docs/prototype/SOURCE_READS.md) defines the interface; the
[design](../../docs/plans/verified-source-reuse.md) records the verification seam and
staging trade-off. The [procedure](README.md) reproduces the measurements.
The subsequent [reclamation report](../reclamation/RESULTS.md) adds owned range reuse;
the measurements below describe the preceding cumulative append adapter.

## Capture observations

Preserved previous-stage executables match the hashes in the committed
[streaming observations](../stream-capture/results/observations.json). They run first,
followed by the new implementation, on marked owned copies of the same pinned Linux
and PyTorch scopes. Both sides verify complete source manifests, unchanged sharing,
expired external pins, restoration and release. Original source manifests, cumulative
spool counts/bytes/reads and every phase's owned buffer inventory match exactly.

| Observation | Linux before / after | PyTorch before / after |
| --- | --- | --- |
| Baseline capture/validation | 4,677 / 4,682 ms | 630 / 550 ms |
| Unchanged capture | **3,674 / 2,172 ms** | **569 / 263 ms** |
| Fourth changed capture/validation | 8,655 / 7,503 ms | 1,008 / 1,075 ms |
| Sampled process peak RSS | 2.263 / 2.263 GiB | 0.307 / 0.307 GiB |
| Baseline owned source/index/integrity buffers | 0.420 / 0.420 GiB | 0.054 / 0.054 GiB |

Observed unchanged times decrease by about 41% / 54%. Retained memory does not change:
the optimisation skips temporary metadata work and a staged read. New/changed inputs
still build their full metadata. PyTorch's fourth changed phase is slower in this sample;
extra speculative hashing and environmental variation remain relevant. These are single
source-only history conditions with different UUID prefixes, not general speedup or tail
estimates. Phase times include independent prefix/source validation and verified loads.
Parsers, facts, watcher scheduling and UI work are omitted.

For the 512 MiB single-line fixture, unchanged streamed capture changes from 880 to
428 ms; sampled peak RSS stays near 3 MiB (3.22 / 3.07 MiB). Both implementations verify
complete bytes, identity, encoding, line count and unchanged-source allocation sharing
using bounded reads. The whole-vector fixture reference also changes from 473 to 345 ms,
showing timing variation across the sequential runs; its peak remains about 515 MiB.

Two additional streamed fixture pairs use the same owned input in opposite orders:

| Implementation order | Before unchanged | After unchanged |
| --- | --- | --- |
| Before then after | 829 ms | 396 ms |
| After then before | 722 ms | 367 ms |

The direction is consistent in both orders. Four additional processes remain a small
sample and do not establish latency tails. First-capture times also vary; no isolated
throughput claim is made for initial or changed capture.

The i7-13700H host exposes 20 logical CPUs and about 62.5 GiB RAM, with Rust 1.99.1.
No builds, tests, mutation jobs or other benchmarks compete during timing. OS caches
are warm. The child virtual address-space limits are 2 GiB for the fixture and 12 GiB
for corpus histories. They do not enforce RSS, physical RAM, combined editor/browser/OS
memory or the baseline laptop envelope. Memory peaks include probe rewriting, inventories
and verification. Admission/browser latency workflows were not remeasured; integrated
correctness checks ran separately.

## Why verification remains complete

`SourceVerifier` retains fixed hash state, a borrowed previous capture and a remaining-byte
count. It uses the previous captured length as a candidate prefix for the existing
namespaced identity algorithm. Native capture still reads through actual EOF, feeds
every input slice and stages every byte. Reuse requires both exact observed length and
matching digest, followed by cancellation checks. Merely reaching the expected length
does not finish capture. Missing bytes, extra bytes, same-length changes and errors after
matching data all reject reuse.

The verifier never loads previous backing or builds line/encoding metadata. On mismatch,
the existing staged path builds identity and complete metadata from actual staged length,
then commits as before. Changed inputs with a previous capture incur extra input-pass
hashing; longer inputs stop speculative hashing after their length already rejects reuse.
This avoids a second read of a mutable project file. All immutable pin, disk capacity,
rollback and cleanup rules remain unchanged.

## Correctness and mutation evidence

New interface tests first failed before the verifier existed. They compare verification
with independent `ContentId::derive` over empty, binary, Unicode and multi-block inputs,
using one-byte, three-byte and 64 KiB partitions. They cover truncation, suffixes, same-length
changes, empty updates and irreversible excess length. A failing backing fixture proves
verification needs no backing reads. Native tests check changed text/binary/short/long/empty
metadata and preserved old bytes, plus error and cancellation after matching input data.
Existing full-quota reuse, concurrent readers, commit rollback and source/pin tests pass.

Formatting, **246 Rust tests**, doctests, Clippy and **58 Node/browser checks** pass.
Complete core capture/read and native spool mutation scopes run both packages' tests
for every mutant with a passing unmodified baseline: **258 outcomes, 207 caught by tests,
39 unviable, 12 timeouts, zero survivors**. No new exclusions or survivor reruns were needed.
All generated mutations in the new verifier/hasher methods are caught except the
unviable default-return verifier constructor. The model change only re-exports the type.

Timeouts break capture/read-loop progress and are separate from assertion catches;
unviable substitutions fail compilation/build. The existing incremental exclusion
configuration is unchanged and concerns an unrelated engine directory-prefix equivalence.
Dependency/hash macro internals, optional benchmark probes/Python, allocator/filesystem
and browser internals are outside scope. Coverage and zero survivors are evidence,
not proof of correctness.

## Remaining costs

Staging still writes all input and truncates after reuse; unchanged verification is not
metadata-only change detection. Changed capture still builds metadata and copies into
committed backing. Committed and staging files each have the configured capacity, so
their combined lengths may reach twice it. Session reclamation/compaction, resident
metadata admission, parser/fact/response allocations and a calibrated process/physical-RAM
envelope remain separate work. Cooperative cancellation has no hard IO deadline.

The [before observations](results/before.json), [after observations](results/after.json),
[summary](results/summary.json), [mutation details](results/native-mutation.json), and
[source hashes](results/source-sha256.json) preserve workload, code and binary identity.
The [opposite-order fixture observations](results/fixture-pairs.json) and
[corpus CSV](results/corpus-summary.csv) retain the additional comparison data.
