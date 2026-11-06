# Fixed-weight physical batch resource screen

For contributors assessing whether smaller prompt-processing batches bring the 4B
candidate inside the local QA budget. **None of the three profiles meets the declared
4 GiB observed PSS target.** Physical batch 8 saves about 13.2 MiB relative to 128 while
the response takes 64.5% longer. No profile is selected and the conditional fifteen-question
validation is not run. This completes the bounded diagnostic with a budget miss.

## Three serial probes

Each profile answers `decomp.commit-checks.full` once with the same complete source and
answer-first schema. Logical batch stays 512; only physical batch changes. All three
requests finish normally with source binding and requested field order intact.

| Physical batch | Peak process-tree PSS | First token | Full response | Prefill tokens/s | Output tokens |
| --- | ---: | ---: | ---: | ---: | ---: |
| 128, control | 4.658 GiB | 130.9 s | 185.7 s | 25.44 | 329 |
| 32 | 4.651 GiB | 151.8 s | 212.9 s | 21.94 | 356 |
| 8 | 4.645 GiB | 250.8 s | 305.5 s | 13.28 | 326 |

The smallest physical batch remains about 661 MiB above the target. Startup is 2.50,
2.15 and 2.21 seconds respectively; observed whole-run time, including indexing/startup,
is 200.9, 229.7 and 321.5 seconds. Full-response latency also reflects different output
lengths. Prefill handles the same 3,329 uncached prompt tokens in each fresh server.

These are single observations in fixed profile order, with no variance estimate or
counterbalancing. A fresh server/context does not imply a cold filesystem cache. The
prior full-suite peak of 4.76 GiB and this one-question control's 4.658 GiB cover different
workloads; their difference is not evidence of an optimisation.

## Fixed inputs and selection

The [contract](../../docs/qa/PREFILL.md) and [profiles](library/prefill/profiles.json)
predeclare resource-only selection: fastest successful full response with at most
4 GiB sampled process-tree PSS. `selectResourceProfile` returns `null` for these records.
The [screen data](results/prefill/screen.json) explicitly records that stage two has zero
responses because no profile qualified. No answer score influences selection.

The [preparation](results/prefill/prepared.json.gz) freezes code/data/native binary hashes,
source archive hash, selected case IDs and permitted runtime profiles. Post-run checks
verify all frozen files, the source archive, exact model/runtime hashes, actual child
arguments, and matching context/messages/limits against the prior 4B commit question.
Arguments differ only in physical batch and the ephemeral loopback port among these runs.

- Source: lpc-rs `1a4f5b65dd2a269298748df2dfd68092a169a8d9`, same five source ranges,
  12,993 evidence bytes. No gold fact prose or grading decisions enter model messages.
- Weights: unchanged Qwen3.5-4B Q4_K_M, 3,013,027,808 bytes, SHA-256
  `13c16f426047e2de38cd075bdade4a7bcbc8c774384876f677740cda65f8a983`.
- Runtime: llama.cpp b12110, executable SHA-256
  `eaf9c1c664cc0ae1f4dea4b171bb91aac8409fcfea0574c9b6bcc93792ce2400`.
- CPU-only, two inference/prefill threads, one slot, 8,192 context tokens, unchanged
  cache types, temperature 0, seed 42, thinking off, `nice -n 10`.
- Same 300-word/1,024-token/eight-ID output allowance and 2,000-character schema limit;
  uniform 600-second timeout for these resource probes. Earlier full-suite timeout was
  360 seconds. The manifest's generic 384-token default is overridden by the recorded
  arm and response limit of 1,024, as in the prior comparison.

Only one measured model is resident at a time. Builds and correctness checks finish
before inference. Each server closes before the next; recorded PIDs are absent afterward.
No viewer, editor, training, weight conversion or inspected-project build is invoked.

## Output and measurement limits

The 128 control reproduces the prior 4B commit answer byte-for-byte. Both smaller-batch
answers change. They are retained without repair and **not newly semantically graded**:
this resource screen does not establish improved, preserved or degraded task accuracy.
Native reference validity and valid JSON order do not establish factual completeness.
If these answers are graded later, the [dispatch oracle correction](DISPATCH_ERRATUM.md)
still applies: `Committer::dispatch` is real.

PSS is a simultaneous sampled sum of the measured process tree, including the observer,
Node driver, native index and model, rather than the weight file size or sum of individual
high-water marks. Sampling is non-atomic and can miss short peaks. The observer interval
is 250 ms, with no sampling errors reported. It excludes unmapped filesystem cache and
other kernel charges. RSS counts shared mappings repeatedly.

This is observe-only on an i7-13700H host with about 64 GB RAM. No cap is enforced and
no 16-GB laptop/editor workload is emulated. The memory files' inactive
`requested_cap_bytes` value of 12 GiB is the observer default, **not** the QA selection
target. Even an observed fit would not qualify the device or bound a longer workload.

## Verification and artifacts

The [validation record](results/prefill/validation.json) and compressed logs retain:

- 453 passing Rust workspace tests including doctests, formatting and all-target Clippy.
- 13 passing serial Node test files. The new focused profile test first fails without
  its implementation, then tests defaults, input rejection, filtering and resource-only
  selection. A stub executable verifies the actual spawned CPU-only batch arguments.
- Post-run code/data/binary, source, model and runtime identity checks; unchanged checksums
  for every artifact in the prior model comparison.

No new Rust/dependency changes or native mutation cohort are included. The prior relevant
provenance run remains 52 caught and three unviable; that is not mutation coverage of
JavaScript or this performance diagnostic.

Each profile directory in [the archive](results/prefill/) retains manifest, actual runtime
arguments/startup, raw messages/source/answers, complete runtime/run logs and process-tree
memory samples. [Checksums](results/prefill/checksums.json) cover the archived payloads.
The [procedure](PREFILL.md) describes reproduction. Older runs, grades and checksums stay
unchanged.

## Implications

Reducing physical batch alone does not recover the required memory in these observations.
This does not test changing logical batch, cache precision, context allocation or weights,
and the memory samples do not identify which backend allocation dominates. Further runtime
work should first inspect allocation sizes and isolate a different setting in a new cohort.

The maintainer also proposed targeted behavioural weight edits. The
[weight-space explanation](../../docs/explanation/model-weight-optimisation.md) reviews
the linked report, released patch and related editing literature. That is a separate
quality hypothesis: better evidence use could make a smaller model sufficient, while a
shape-preserving edit itself does not shrink dense allocations. Neither a weight patch
nor a new model comparison is performed here. QA remains outside the product UI.
