# Parallel syntax parsing measurements

File-level parsing scales well on these inputs, with diminishing returns and different
memory costs between languages. Eight workers delivered roughly five times the batch
throughput of one worker. Sixteen workers improved Rust, Python, and C further, while
the observed C++ throughput plateaued around eight workers.

The [scaling figure](results/parallel/scaling.svg) compares speedup and memory.
[summary.json](results/parallel/summary.json), [executions.json](results/parallel/executions.json),
and [run.json](results/parallel/run.json) preserve measurements, commands, and provenance.
Use the [experiment guide](README.md) to reproduce the comparison.

## Experiment and validation

The inputs and grammars match the [previous round](SECOND_RESULTS.md): lpc-rs Rust,
PyTorch Python and C++/headers, and Linux C/headers. Source revisions remain pinned to
the same archived commits. These are raw syntax scopes, without compiler preprocessing
or semantic analysis.

Each worker owns a parser and reuses it across files. A dynamic queue assigns
indivisible files in descending byte size. Every run accounts for all input paths and
matches every file's content fingerprint, byte length, and syntax counts against a
one-worker reference. Aggregate counts also match the previous round. Full AST shapes
are not compared in this experiment.

The machine reports 20 logical CPUs across 14 physical cores, with an i7-13700H and
approximately 62 GiB RAM. Affinity permitted 20 logical CPUs. A cgroup CPU quota was
not available through the exposed filesystem. Cache state, power settings, and other
system load were uncontrolled. Worker counts ran in ascending order, in separate
processes, with no simultaneous experiment or build jobs.

Rust, Python, and C++ have three passes per worker count; the tables use their medians.
Linux has one full-corpus observation per worker count and a final one-worker repeat.
These are exploratory batch measurements, without interactive contention or reliable
latency-tail estimates.

## End-to-end batch wall time

All values are seconds. The timer includes worker creation, parser setup, reads,
parsing, inspection, hashing, tree disposal, joins, and result sorting. Discovery,
baseline loading, output comparison, and result-digest serialization are outside it.
Summed worker phase durations are reported separately and are not elapsed time or
process CPU time. These wall times should not be compared directly with the previous
round's parse-only phase durations.

| Workers | lpc-rs Rust | PyTorch Python | PyTorch C++/headers | Linux C/headers |
|---|---:|---:|---:|---:|
| 1 | 0.913 | 14.994 | 9.269 | 128.981 |
| 2 | 0.506 | 8.390 | 5.082 | 70.538 |
| 4 | 0.288 | 4.833 | 2.759 | 40.368 |
| 8 | 0.184 | 3.049 | 1.730 | 25.004 |
| 16 | 0.141 | 2.050 | 1.739 | 18.571 |

Relative to the initial one-worker cases, sixteen workers achieved 6.47x for Rust,
7.31x for Python, 5.33x for C++, and 6.95x for C. C++ achieved 5.36x at eight workers;
the difference between its eight- and sixteen-worker observations is too small to
establish an improvement in either direction from these samples.

The final one-worker Linux run took 119.685 seconds, 7.2% less than the initial
128.981 seconds. Using those two baselines, the sixteen-worker gain is approximately
6.4–6.9x, and the eight-worker gain is approximately 4.8–5.2x. The experiment does not
isolate the cause of baseline drift.

Files remain indivisible in this pool. Large files can therefore limit scaling even
when other workers are available. The precise cause of the C++ plateau was not isolated.
Throughput gains across files do not reduce the latency of a single expensive edit.

## Process memory

Peak RSS is in MiB for each worker-count process. It includes the parser pools,
reference/result vectors, and their serialization. Pools are recreated each pass;
whole-project syntax trees and semantic indexes are not retained. The initial
one-worker process creates its reference, while subsequent cases load it outside the
timed passes, so memory figures include some differences in validation setup.

| Workers | lpc-rs Rust | PyTorch Python | PyTorch C++/headers | Linux C/headers |
|---|---:|---:|---:|---:|
| 1 | 14.7 | 44.3 | 177.0 | 109.3 |
| 2 | 20.1 | 71.3 | 345.7 | 162.1 |
| 4 | 34.8 | 133.3 | 432.1 | 262.4 |
| 8 | 54.9 | 251.6 | 486.2 | 443.2 |
| 16 | 78.4 | 411.1 | 619.3 | 650.4 |

For C++, doubling the worker limit from eight to sixteen increased peak RSS by about
27%, with no meaningful observed throughput gain. Memory therefore needs its own
budget rather than being inferred from the logical CPU count. These process peaks do
not qualify the full application on the intended 16 GB target.

## Architectural implications

The measurements support a bounded pool with parser ownership local to each worker
and dynamic file assignment. A production scheduler should share a CPU and memory
budget across languages and distinguish interactive edits from background discovery
and full indexing. Separate unrestricted pools per language would compound resource use.

Four to eight workers are useful candidate budgets for the next interactive experiment:
they provided approximately three to five times batch throughput here. The default
should be chosen after measuring renderer/source-view responsiveness and contention on
the target hardware. Sixteen workers remain a useful explicit batch option on this host.

Long single-file work still needs asynchronous execution and cancellation. Streaming
versioned results, semantic dependency scheduling, worker reuse across a live session,
and bounded tree-cache retention remain separate measurements. This benchmark waits
for its complete batch before returning results.

## Verification

The project passes 41 unit tests and six CLI integration tests with all features.
Tests first failed against incomplete pool and measurement implementations. They check
per-file results against independent sequential parsing, worker limits, complete work
accounting, error propagation, phase aggregation, baseline protection, and detection
of changed source contents. Formatting and Clippy pass with warnings denied.

The new pool module's mutation run tested 19 mutations: 15 were caught, four did not
compile, and none survived or timed out. [mutation.json](results/parallel/mutation.json)
records that scope; CLI orchestration and third-party implementations are outside it.
Project-authored code remains safe Rust, with no additional dependencies introduced.
