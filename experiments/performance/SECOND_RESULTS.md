# Repository parsing and filesystem event measurements

This round compares content inventory with syntax parsing and measures the path from
a native filesystem notification to a verified manifest update. All nine cases
completed, with incremental parse results checked against full parses and deliberate
event loss recovered by a scan. These are native Linux experiments rather than
application or browser benchmarks.

[run.json](results/round-2/run.json) records source revisions, dependency versions,
environment, verification, and scope. [executions.json](results/round-2/executions.json)
records commands and exit codes. The [experiment guide](README.md) explains reproduction.

## Sources and conditions

The inputs were committed Git archives of the local reference checkouts. Linux uses
`cbbddb6b08cc9900a87cccaa9b22bbe06aa4f50a`; PyTorch uses the available local revision
`1f4d2d5e820dd870626dca4f9603e3a49879dbde`, replacing the earlier unmeasured target.
lpc-rs retains `8d3b1f54b36eae33f689bae3e42c716647000e4d` from the initial round.
Submodules were not expanded, and working-tree changes were not incorporated.

The machine is the same Intel Core i7-13700H system reporting about 62 GiB RAM. Jobs
ran sequentially after builds and mutation tests finished. OS cache state, unrelated
load, and power settings were uncontrolled. The results do not qualify the intended
integrated-GPU laptop with 16 GB RAM.

## Complete content inventory

These are medians over seven observations, in milliseconds. A verified rescan reads
and hashes every included regular file, then compares manifests. Metadata reads alone
do not verify unchanged contents.

| Corpus | Files | Content MiB | Discover paths | Read metadata | Verified rescan and comparison | Decode JSON manifest |
|---|---:|---:|---:|---:|---:|---:|
| lpc-rs | 1032 | 10.50 | 2.29 | 1.37 | 16.53 | 2.12 |
| Linux | 97239 | 1598.00 | 171.12 | 115.23 | 1810.62 | 238.48 |
| PyTorch | 22720 | 238.89 | 48.49 | 25.95 | 369.92 | 69.15 |

The three inventory processes peaked at approximately 3.1, 83.5, and 24.3 MiB RSS,
respectively. This covers fingerprint manifests and their benchmark copies, excluding
syntax trees and semantic indexes. JSON decoding uses an in-memory buffer and does
not measure persistent-cache disk loading.

The gap between a repository-wide scan and an individual notification remains
substantial on real source trees. Reconciliation should remain background work;
source providers can later be evaluated for additional ways to identify unchanged
committed inputs.

## Sequential syntax parsing

Each grammar parsed all matching files three times, with a reusable parser and trees
dropped between files. Source reads, parsing, and node inspection were timed separately.
The table reports median whole-pass parsing and inspection times in seconds. Recovery
means a file contained at least one error or missing node in the returned tree.

| Corpus and grammar | Files | Parsed MiB | Full parsing | Node inspection | Files with recovery |
|---|---:|---:|---:|---:|---:|
| lpc-rs Rust | 551 | 7.21 | 0.928 | 0.148 | 0 |
| Linux C and headers | 65034 | 1414.39 | 108.580 | 12.466 | 32896 |
| PyTorch Python | 5115 | 114.91 | 12.981 | 1.888 | 4 |
| PyTorch C++ and headers | 4876 | 56.93 | 7.961 | 0.823 | 2079 |

These inputs were not preprocessed or selected by a compiler build configuration.
Headers were assigned to the explicitly requested grammar; build-derived language
contexts were not reconstructed.
The recovery counts describe grammar behaviour on raw source, including tests and
generated or third-party code. They are not measurements of compilation validity,
resolved references, or overall analysis accuracy. A file with recovery may still
contain usable syntax information; the experiment does not measure that proportion.

The Linux pass traversed about 232 million syntax nodes. Whole-project syntax-tree
retention was not measured. These results support retaining compact reusable facts
and evaluating bounded syntax-tree caches before selecting a production index design.

## Retained-tree edits and preparation costs

The probes appended a comment, prepended a comment, and replaced an ASCII letter near
the middle of each selected source buffer. Every resulting tree matched a full parse
of the same edited buffer in the compared kinds, fields, flags, depths, and byte ranges.
The original trees remained unchanged. Validation ran outside timed phases.

The following rows cover the largest selected file for each grammar. Values are the
range of the three edit-pattern medians, in milliseconds, rather than latency tails.
Large-file repetitions were bounded and are shown explicitly.

| Grammar | Largest file MiB | Edit preparation | Incremental parsing | Full parsing of edited buffer | Repetitions per pattern |
|---|---:|---:|---:|---:|---:|
| Rust | 0.31 | 0.03–0.13 | 0.08–0.29 | 32.37–40.03 | 60 |
| C | 22.90 | 2.38–15.14 | 0.02–76.42 | 485.45–534.60 | 3 |
| Python | 1.26 | 0.13–0.68 | 4.59–5.21 | 110.15–138.61 | 25 |
| C++ | 8.03 | 0.69–4.91 | 3.56–6.88 | 759.16–865.64 | 3 |

The largest C file is a generated AMD register header. Appending a comment had a
median incremental parse time of 17.1 microseconds, while the middle-byte replacement
took 76.4 milliseconds. Preparation for the append took 15.1 milliseconds. None of
these three edited C trees reported syntax recovery. Incremental analysis therefore
still needs cancellation and execution outside the interaction thread.

Preparation includes building a new byte buffer and calculating positions. Optimising
the parser alone leaves this cost. Line-position indexing and source-buffer strategies
are useful follow-up experiments. The C++ outlier is the vendored Perfetto SDK header;
explicit analysis scopes will need to account for generated and third-party source.
Recovery was present in that C++ file and was included in the tree comparisons.

Peak RSS for the complete syntax experiments was approximately 54.9 MiB for Rust,
500.9 MiB for C, 170.0 MiB for Python, and 997.9 MiB for C++. These peaks include the
temporary full-tree descriptions used for validation. They do not estimate the memory
needed to retain an entire project's syntax or semantic index.

## Native notifications and recovery

Each fixture used distinct paths for 50 same-length writes and 50 atomic replacements.
The main thread verified the contents after notification and updated a mutable manifest.
All operations succeeded. Times below are microseconds measured from the start of
the write. A callback may arrive before the write call finishes. Writes used ordinary
buffered I/O without fsync; these timings do not measure disk durability.

| Fixture files | Operation | Median callback arrival | Median verified update | Observed p95 verified update |
|---|---|---:|---:|---:|
| 1000 | Same-length write | 10.25 | 29.30 | 41.54 |
| 1000 | Atomic replacement | 37.89 | 49.48 | 105.55 |
| 100000 | Same-length write | 8.95 | 26.47 | 133.17 |
| 100000 | Atomic replacement | 30.86 | 42.03 | 75.21 |

Watcher registration took 2.56 ms for the 1000-file fixture and 163.98 ms for the
100000-file fixture. Recovering a deliberately ignored edit through a complete scan
and comparison took 9.26 ms and 1229.14 ms, respectively.

These are controlled, isolated edits on owned local temporary fixtures. They exclude
parsing, dependency invalidation, immutable index publication, and view updates. They
do not test concurrent writers during initial registration, rapid checkout storms,
remote filesystems, or real operating-system queue overflow. Production initialisation
will need to close the gap between its initial capture and watch registration.

## Verification and next experiments

The library has 37 passing unit tests and five passing CLI integration tests. New
behaviour was first tested against incomplete implementations, producing the expected
failures. Formatting and Clippy pass with all native targets and features checked.
Project-authored Rust remains safe under the inherited unsafe-code lint.

The first new-module mutation run exposed two untested missing-token counter mutations.
A targeted recovery fixture addressed them. One other mutation changes only an initial
allocation-capacity hint and is narrowly excluded with its rationale in
[mutants.toml](mutants.toml). Mutation details and scope are recorded alongside the data.
The combined library run tested 123 mutations: 112 were caught and 11 did not compile,
with no survivors or timeouts, in addition to that one explicit exclusion.
CLI orchestration, GPU/shader code, and third-party parser/watcher implementations are
outside the library mutation scope.

The next measurements should cover incremental fact extraction, dependency
invalidation, consistent publication, and bounded cache retention. Browser rendering
with labels, picking, coordinated views, and the target GPU remains a separate experiment.
