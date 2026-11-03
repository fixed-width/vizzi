# Streaming private source capture

Opt-in spooled capture now reads project files through EOF in bounded requests instead
of allocating a whole-file byte vector. It preserves content identities, UTF-8/line/CRLF
metadata, immutable old pins and verified unchanged-source sharing. Resident capture
remains the default. No dependencies or unsafe-code exceptions were added.

The [source reference](../../docs/prototype/SOURCE_READS.md) defines the interface and
limits; the [architecture plan](../../docs/plans/streaming-source-capture.md) explains the
staging/commit boundary. The [procedure](README.md) reproduces these observations.

## Large-file capture

A generated 512 MiB single-line UTF-8 file isolates transient capture bytes. Separate
sequential processes compare current-code whole-vector capture plus persistence against
streaming capture, using the same metadata builder and a 2 GiB child virtual address-space
limit. Both capture twice and independently verify complete bytes, namespace/length-prefixed
content identity, encoding, line count and unchanged-source allocation sharing. Verification
uses bounded reads rather than another whole-file load.

| Observation | Whole-vector capture | Streaming capture |
| --- | --- | --- |
| Sampled process peak RSS | 515.0 MiB | 3.25 MiB |
| First capture | 1,033 ms | 1,108 ms |
| Unchanged capture | 399 ms | 795 ms |
| Committed bytes after both captures | 512 MiB | 512 MiB |
| Peak staging-file length | 0 | 512 MiB |

The memory improvement is visible for an individual large file. First capture is slightly
slower in this observation, and unchanged capture costs about twice as much. Streaming
must stage input before hashing with the verified actual length, and currently constructs
candidate metadata before discovering unchanged identity. Staging, metadata scanning and
changed-content copying add IO. These single conditions do not establish timing tails or
randomised speedups. The sampled RSS peak includes verification; per-capture high-water
checkpoints and raw measurements are retained separately.

## Representative source history

The same pinned Linux/PyTorch scopes contain 66,066 / 10,578 selected files and
1,495,484,792 / 186,092,681 source bytes. Each marked owned copy runs the existing
source-only storage history: baseline, unchanged verification, four whole-scope changed
generations, an expired external baseline pin, release and restoration. Facts, parsing
and UI work are omitted. Original archives are unchanged and all source/pin/sharing/
restoration/release checks pass. Committed and verified-read byte totals match the
preceding whole-vector backing workload.

| Observation | Linux streamed | PyTorch streamed |
| --- | --- | --- |
| Sampled process peak RSS | 2.263 GiB | 0.307 GiB |
| Baseline capture/validation | 4,447 ms | 665 ms |
| Unchanged capture | 4,010 ms | 611 ms |
| Fourth changed capture/validation | 8,504 ms | 1,105 ms |
| Baseline owned source/index/integrity buffers | 0.420 GiB | 0.054 GiB |
| Fourth version plus external pin, owned buffers | 2.108 GiB | 0.271 GiB |
| Peak staging-file length | 22.90 MiB | 8.03 MiB |
| Committed session bytes | 9.018 GB | 1.124 GB |

The [preceding backing report](../source-reads/RESULTS.md) recorded peaks of 2.339 / 0.318
GiB, baseline times of 3,905 / 535 ms, unchanged times of 1,463 / 217 ms and fourth-change
times of 7,611 / 974 ms. These are historical observations, not paired current-code
speedups. The metadata builder also changes allocation capacities slightly: current
baseline buffers are 0.420 / 0.054 GiB versus 0.414 / 0.053 GiB previously. Small files
now use growing integrity-digest vectors. Metadata remains the dominant retained source
cost; streaming does not remove it. All backed source-body capacities remain zero.

The sizeable unchanged-pass regression deserves follow-up. This implementation verifies
actual staged contents and constructs metadata on every pass before deciding to reuse;
the preceding resident capture hashes first and avoids rebuilding unchanged metadata.
Optimising that boundary must preserve actual-content verification and immutable pins.
No claim is made that streaming improves throughput.

The i7-13700H host exposes 20 logical CPUs and about 62.5 GiB RAM, with Rust 1.99.1.
Children use a 12 GiB virtual address-space limit. No builds, correctness tests, mutation
jobs or other benchmarks compete during timing. OS caches are warm. Single sequential
conditions with different UUID prefixes do not establish timing tails. Storage phase
times include byte-for-byte prefix validation through verified backing loads; probe
rewrites, inventories and final checks contribute to process peaks. These are not
end-to-end indexing or baseline-laptop measurements. Address-space limits do not enforce
RSS, physical RAM or combined editor/browser/OS use. Admission/browser latency workflows
were not remeasured here; integrated correctness checks ran separately.

## Resource contract

Input and append buffers have at most 64 KiB of bytes. Metadata scanning uses a 64 KiB
buffer, an incomplete UTF-8 tail of at most three bytes, and an optional join of at most
64 KiB plus three bytes. Buffers for separate capture phases do not coexist. Resident
line starts, CRLF bits and integrity digests still scale with input. This is a transient
byte-buffer bound, not a resident-index or whole-process memory cap.

A writer gate serialises staging and commits without holding the committed-file mutex
during project IO. Existing readers can access old ranges between backing IO operations.
Actual EOF determines length; no metadata-only equality shortcut is introduced. Changed
captures become visible only after complete append. Unchanged captures retain the old
source even when committed quota is full.

The committed spool and reusable staging file each have the configured capacity as a
length bound; together they may reach twice that capacity. Staging is truncated after
success, reuse, observed cancellation or failure. Incomplete append tails are rolled back
without changing committed ranges. Truncation failures make cleanup best effort. A late
cancellation after commit can still consume session quota without publishing that capture.
Disk reclamation/compaction remains separate work.
Metadata admission, parser/fact/response allocations, allocator retention and a calibrated
whole-process envelope remain unresolved. The default remains resident while these
costs and the unchanged-capture regression are addressed.

## Correctness and mutation evidence

The first streaming tests failed before the new interfaces existed. Tests cover fragmented
and interrupted reads, empty/binary/truncated UTF-8, code points and CRLF across 64 KiB
boundaries, independent content/line checks, exact and exceeded staging/commit limits,
unchanged reuse at full quota, old readers during blocked project IO, cancellation during
staging/scanning/append, rollback, short backing and preserved old pins. Invalid UTF-8 is
also checked for bounded validation scratch, independently of its final metadata.

Formatting, **242 Rust tests**, doctests, Clippy and **58 Node/browser checks** pass.
Default-feature workspace tests also pass. A small pre-existing test crate documentation
ordering issue was corrected so its disabled qualification feature satisfies missing-docs
linting. Production code remains safe Rust.

Complete affected capture/source/spool mutation scopes plus the new digest helper yield
**303 unique outcomes: 232 caught by tests, 54 unviable, 17 timeouts, zero survivors**.
All unmodified baselines pass. Four initial survivors prompted tests for the chosen 64 KiB
request size, full committed quota, short staging IO classification and invalid-UTF-8
scratch growth. Exact-mutant reruns catch all four and replace their earlier outcomes
without double-counting. No new exclusions were added; the existing incremental config
exclusion concerns an unrelated engine directory-prefix equivalence.

Timeouts include broken capture/bootstrap and read-loop progress and are reported separately
from assertion catches. Unviable substitutions fail compilation/build; the digest helper's
generated default-return substitution fails because content identities have no default.
Dependency/hash macro internals, optional measurement probes/Python, allocator/filesystem
and browser internals are outside this mutation scope. Coverage and zero survivors are
evidence, not proof of correctness.

The [raw observations](results/observations.json), [summary](results/summary.json),
[corpus CSV](results/corpus-summary.csv),
[mutation details](results/native-mutation.json) and
[source hashes](results/source-sha256.json) preserve workload, code and binary identity.
