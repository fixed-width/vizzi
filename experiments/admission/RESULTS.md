# Source availability under analysis input admission

Opt-in admission now bounds **active analysis input volume** across foreground and
periodic recovery work. Refused jobs publish exact captured source with explicit
`deferred` coverage and no facts. They remain distinct from unsupported encoding,
source-only language selection and completed grammar recovery. Eligible deferrals retry
through recovery without another edit; oversized inputs stay deferred until input or
configuration changes.

All four Linux/PyTorch conditions completed under a child-only **8 GiB virtual
address-space limit**. The large paste published source in 122 ms Linux and 55 ms
PyTorch with admission, while analysis remained explicitly deferred. These are source
availability observations, not completed-analysis speedups or browser-visible timings.

The [reference](../../docs/prototype/ADMISSION.md) defines exact capacity, foreground
headroom, cache/retry rules and exclusions. The [plan](../../docs/plans/indexing-admission.md)
records the design. This first stage does not cap in-memory capture/snapshot storage,
parser scratch/facts, retained parser allocations or total physical memory. Retention
and input admission remain disabled by default, with independent opt-in settings.

## Workload and constrained conditions

The [Linux procedure](README.md) uses the pinned qualification scopes, four shared parser
workers and 512 MiB syntax retention. Each repository has separate admission-disabled
and admission-enabled processes, marked disposable copies and unique UUID comments.
No analysed project is built or executed. Current-context fact caches are prepared
before timing because the new portable coverage model conservatively changes analyser
fingerprints. Preparation is separate from the bounded workflow timings.

Each condition makes four changes:

1. Append 1,000 declarations to `kernel/locking/mutex.c` or `c10/core/Device.cpp`.
2. Paste the primary header plus 2 MiB whitespace into that ordinary source file.
3. Add a new root source file containing a unique copy of the primary header.
4. Change the three large files used by the preceding working-set experiments together.

Every write follows a sample of `recovery.running: true`. Recovery can finish between
that status response and IO; this observes activity, not exact parse/IO overlap. A held
recovery parser in correctness tests establishes actual foreground overlap and verifies
that a 75-byte recovery permit leaves 25 bytes for a held foreground job in a 100-byte
budget. Shutdown releases both charges.

Linux uses 32 MiB total active input, a 24 MiB per-file/recovery limit and 8 MiB foreground
headroom. PyTorch uses 12/9/3 MiB. These deliberately admit every original file while
making the paste exceed the per-file limit and the batch exceed aggregate capacity.
They are diagnostic conditions rather than memory recommendations. Linux starts with
66,066 analysed files; PyTorch with 10,578 files, 10,147 analysed and 431 CUDA source-only.
The unseen step adds one source file per owned copy, later removed.

The i7-13700H host exposes 20 logical CPUs and about 62.5 GiB RAM, with Rust 1.99.1.
No builds, tests, mutation jobs or competing benchmarks run during workflow timing.
Native watches are disabled for controlled hints; periodic recovery remains active.
A Python wrapper sets `RLIMIT_AS` before exec for each probe child. This limits virtual
address space, not RSS, physical RAM or the combined editor/browser/OS envelope. It does
not qualify the 16 GB laptop or a cold-indexing workflow under that physical limit.

## Published source and deferred completion

| Operation | Linux disabled / admitted | PyTorch disabled / admitted |
| --- | --- | --- |
| Structural edit to published source | 70.7 / 76.9 ms | 42.2 / 39.3 ms |
| Large paste to published source | 938.5 / 121.9 ms | 1,979.3 / 54.6 ms |
| Unseen file to published source | 757.5 / 790.7 ms | 1,455.9 / 1,541.4 ms |
| Three-file batch to published source | 1,035.3 / 866.9 ms | 1,871.6 / 1,683.1 ms |

The admitted paste exceeds the per-file limit: about 26.14 MB Linux and 10.52 MB PyTorch.
It publishes source with empty deferred analysis and remains deferred during settlement.
The disabled condition completes a raw syntax attempt before publication; those attempts
report partial grammar coverage. Comparing these paste times as analysis speedups would
hide the changed availability outcome.

The admitted Linux batch initially publishes two deferred changed files; PyTorch initially
publishes one. Their eligible analysis then completes through recovery without another
edit. The unseen files fit per-file limits and complete raw syntax analysis in both
modes. Structural edits likewise complete, with Linux reporting partial and PyTorch
syntax coverage. Completed facts exactly match independent fresh parsing.

Latest status measurements can belong to a later accepted recovery operation. Per-file
`documents` records describe the captured publication; `settled_documents` records verify
later eligible completion. Whole-snapshot deferred counts can still include the earlier
oversized paste when another changed file is complete. Settlement is checked after
independent source verification, so its sampling is not a separate causal latency metric.

All **16 source/classification/scope comparisons** match independent directory capture.
All **22 completed changed-file analyses** match fresh parsing independently of persisted
facts. After restoring targets and removing the new files, all **four final observations**
match both their complete initial baseline and a clean full capture. Independently copied
old source bytes remain unchanged. Original archive/target hashes are preserved.

These are single-process observations per condition, with different UUID comments.
They do not establish tails, randomized paired speedups, frame pacing, GPU/physical
presentation, or a universal latency improvement. Per-file deferral is deliberately
visible rather than presented as successful empty analysis.

## Resource observations and limits

| Observation | Linux disabled / admitted | PyTorch disabled / admitted |
| --- | --- | --- |
| Sampled peak RSS including verification | 4.776 / 4.720 GiB | 1.667 / 1.336 GiB |
| Sampled peak virtual memory | 5.086 / 4.939 GiB | 1.830 / 1.460 GiB |
| Peak admitted input | Disabled / 31.609 MiB | Disabled / 11.986 MiB |
| Final active permits / charged bytes | N/A / 0 | N/A / 0 |

Linux's disabled lifetime RSS high water is slightly higher than the sampled peak,
4.797 GiB. Probe peaks include source verification, independent fresh parses, temporary
probe input construction, old observations and allocator retention. The shared sampler's
phase labels do not isolate verification. Differences are not a controlled allocation
study and do not prove monotonic RSS savings from input capacity.

Cumulative refusal counters include bootstrap, recovery and repeated attempts, not unique
files: Linux records six busy/five oversize refusals, PyTorch 478/seven. Deferral is
nonblocking; a small active-input quota can initially refuse many cache-load jobs while
other jobs hold permits. Quiet recovery completes eligible work. Counters are distinct
from current deferred-file totals and completed-analysis counts.

Input permits precede cache loading and parser creation and release on every exit path.
Complete immutable in-memory analysis can be reused without a new permit. Deferred
records are rejected by cache storage/loading and skipped by complete-document reuse.
Source is still reopened and hashed normally. Captured input is already observation
storage before admission, so this work does not bound capture buffers or retained source
history. A future capture/storage budget needs its own design; input volume also cannot
predict or enforce parser scratch allocation. No hard cancellation deadline is introduced.

## Validation evidence

The admission interface and source-preserving retry tests first failed before their
methods/coverage existed. Tests exercise exact limits, empty inputs, positive configuration,
foreground reserve, shared concurrent permits, unwind/drop release, oversize/busy coverage,
source and old-pin preservation, cached-work gating, complete-document reuse, cache rejection,
and retry without edits. A real browser test reads a deferred file, sees its resource
reason, then regains an outline after shrinking the input while its old source pin stays
unchanged. Description tests distinguish deferred assignment from source-only selection.

Formatting, **211 Rust tests**, doctests, Clippy and **58 Node/browser checks** pass.

| Mutation scope | Tested | Caught | Unviable | Timeouts | Survived |
| --- | --- | --- | --- | --- | --- |
| Complete native admission/engine/cache/host/main and core model/query modules | 300 | 234 | 60 | 6 | 0 |
| Complete JavaScript source-description module | 59 | 59 | 0 | 0 | 0 |

Passing unmodified baselines precede interpretation. Development survivors exposed
missing refusal-counter assertions and the accepted side of MiB conversion boundaries.
The final aggregate replaces admission/main outcomes with their complete reruns, without
duplicate scopes. The existing equivalent directory-prefix exclusion is unchanged;
no new exclusions were added. Timeouts break bootstrap/worker/recovery progress and are
separate from assertion catches. Unviable substitutions are compiler/build failures.

Grammar/dependency internals, serialization/hash macro bodies, optional probe orchestration,
Python driver, and OS delivery are outside native mutation scopes. App canvas/count/tooltip
rendering and CSS are outside description mutations; browser integration checks run
separately. No dependencies or unsafe-code exceptions were introduced. Zero survivors
is evidence rather than a proof of correctness.

[Raw observations](results/observations.json), [summary](results/summary.json),
[CSV](results/summary.csv), [native mutation details](results/native-mutation.json),
[description mutations](results/description-mutation.json), and
[source hashes](results/source-sha256.json) retain configuration, source/binary identities,
original inputs, coverage, limit/refusal observations, and independent checks.
