# Initial native performance observations

These experiments establish input-processing costs and native rendering availability.
They do not select a production renderer or measure semantic indexing. Run conditions,
source revisions, and verification evidence are recorded in [run.json](results/run.json).
Use the [experiment guide](README.md) to repeat the measurements.

Completed follow-up measurements using the local Linux and PyTorch checkouts are in
[the repository parsing and filesystem event report](SECOND_RESULTS.md).

## File inventory observations

The runs used an Intel Core i7-13700H system reporting approximately 62 GiB of RAM.
The filesystem cache, power settings, and unrelated system load were uncontrolled.
The project's own builds and mutation tests finished before timing began. File
discovery, hashing, and manifest processing were sequential.

Each scan operation has seven samples. Medians below are exploratory observations;
seven samples are insufficient to characterise a reliable latency tail.

| Operation | lpc-rs | Synthetic 100000 files |
|---|---:|---:|
| Regular files captured | 1032 | 100000 |
| Contents fingerprinted | 10.50 MiB | 97.66 MiB |
| Discover paths, median | 2.28 ms | 125.81 ms |
| Read metadata for known files, median | 1.55 ms | 95.54 ms |
| Verify all contents and compare manifests, median | 16.33 ms | 960.02 ms |
| Decode JSON manifest from memory, median | 2.13 ms | 248.44 ms |
| Process peak RSS across the complete run | 2.97 MiB | 82.07 MiB |

The lpc-rs source came from a Git archive of revision
`8d3b1f54b36eae33f689bae3e42c716647000e4d`. The synthetic fixture has distinct
1024-byte files grouped into directories of 100 files. Neither case runs language
analysis, including LPC analysis. Raw observations and process memory reports are in
[lpc-rs.json](results/lpc-rs.json), [synthetic-100k.json](results/synthetic-100k.json),
[lpc-rs-memory.txt](results/lpc-rs-memory.txt), and
[synthetic-100k-memory.txt](results/synthetic-100k-memory.txt).

Applying a supplied buffer change, including hashing its contents, had a median of
4.21 microseconds for the selected 4069-byte lpc-rs file and 1.52 microseconds for a
1024-byte synthetic file, over 700 samples each. These measurements exclude event
detection, file reads, the initial working-manifest clone, semantic recomputation,
immutable snapshot publication, and view updates.

Full reconciliation belongs in background work: even metadata reads over 100000 known
files exceeded a 60 Hz frame budget. Supplied changes can avoid that repository-wide
work, while reconciliation remains necessary for correctness. The experiment has not
implemented or measured a filesystem watcher.

JSON decoding was a material cost at 100000 files. This provides a baseline for later
storage experiments, without committing the project to JSON as its persistent format.

## Rendering observations

The environment exposed llvmpipe through Vulkan and no physical GPU device. The
executable correctly declined timing until software rendering was explicitly enabled.
The opted-in run exercised 10000 rectangles and 40000 straight edges at 1920 by 1080.
Its median synchronous frame wall time was 98.82 ms; this is a software-renderer result.
Adapter information and all summaries are in
[software-rendering.json](results/software-rendering.json).

This run only exercises the native offscreen rendering path. Browser presentation,
text, picking, multiple simultaneous views, and background analysis remain unmeasured.
The integrated-GPU laptop with 16 GB RAM remains the hardware qualification target.

## Verification observations

Tests were first run against incomplete implementations and failed on the expected
behaviour. The completed library passes 21 unit tests, with three additional CLI
integration tests checking report contents, source preservation, invalid sample counts,
and refusal to overwrite a generated fixture. Formatting and Clippy pass with all native
targets and features checked and warnings denied. No project-authored unsafe code was
introduced.

The first mutation run found 36 survivors, including incorrect geometry and an
unchecked manifest accessor. Stronger assertions addressed those gaps. The final run
examined 85 mutations: 77 were caught by failing tests, eight did not compile, and none
survived or timed out. [mutation.json](results/mutation.json) records the scope and
unviable cases. CLI orchestration, GPU/shader code, and browser behaviour are outside
that mutation result.

## Remaining measurements

- Acquire and measure the pinned Linux and PyTorch snapshots; archive downloads failed
  in this environment. Their selected revisions are recorded in `run.json`.
- Measure event delivery and recovery, actual parsing, dependency invalidation, and
  publication of consistent index updates.
- Run representative labelled, interactive, coordinated views on the target GPU and
  browser combinations, including frame-time tails and peak memory.

These are follow-up experiments, not evidence that those capabilities already exist.
