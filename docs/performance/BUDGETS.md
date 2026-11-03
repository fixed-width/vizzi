# Provisional interactive resource budgets

These are proposed engineering targets informed by
[full-repository measurements](../../experiments/qualification/RESULTS.md). They are
not predeclared acceptance criteria, enforced limits, or proof of the 16 GB laptop
baseline. Update the targets with hardware and workflow evidence rather than silently
claiming the current implementation passes them.
The [responsiveness follow-up](../../experiments/qualification/RESPONSIVENESS_RESULTS.md)
updates ordinary edits and outline rendering; it does not repeat retained-reparse or
cancellation qualification.
The [cancellation follow-up](../../experiments/qualification/CANCELLATION_RESULTS.md)
repeats four-worker retained reparse and measures cooperative interruption within files.

| Area | Proposed target / policy | Current evidence |
| --- | --- | --- |
| Interactive parsing | Four shared workers by default | Preserves throughput without separate per-language pools |
| Background throughput | Eight only with available resources and measured query impact | Retained reparse improves about 1.6×; Linux core-query p95 grows from at most 44.4 to 52.6 ms |
| Native workspace memory | Aim below 8 GiB peak while retaining the previous observation | Latest four-worker shared-source reparse: Linux 2.526 GiB, PyTorch 0.757 GiB on the 62.5 GiB host. Earlier eight-worker conditions are separate; no enforced cap |
| Bounded core view | Aim for p95 below 50 ms for the current 2,000-tile query | Four-worker observations fit on this host; eight-worker Linux has misses |
| Ordinary edit visibility | Aim for ordinary probe edits below 200 ms before collecting meaningful tails | Ten writes following observed active recovery per profile: Linux p50/max 120/139 ms; PyTorch 96/106 ms. All twenty are one-file attempts; finite samples on the reference host |
| Cancellation response | Aim below 250 ms after a request during parsing | Follow-up direct engine-return ranges: Linux 41–46 ms; PyTorch 18–23 ms. Three samples each; cooperative callbacks do not impose a hard deadline |
| Retained syntax | Opt-in shared admission quota, initially 512 MiB and eight entries | Charges old/new state together; Perfetto peaks at 500.3 MiB weight. Normal parsing/capture allocations remain outside it |
| Source and outline DOM | Bound each pane by visible rows plus overscan; initial ceiling 200 rendered entries | Latest headers render 47–49 source rows and 0/21 outline controls. Browser delivery uses <=96 source rows/32 KiB text and <=200 outline entries; lazy identity indexing remains input-dependent |
| Overview motion | Maintain rAF p99 below 33.3 ms in the software reference workload | 16.8 ms in all recorded steady/refresh runs; physical presentation unqualified |

Cold indexing is a separate throughput/startup measure: about 60 s Linux and 11 s
PyTorch at four workers. The host currently exposes its browser URL only after initial
publication. Progress/partial exploration during cold indexing is a separate design
decision, not something the existing loading behavior already provides.

The native memory target leaves room for an editor, browser, OS, and filesystem cache,
but the combined envelope has not been validated. Point-in-time renderer heap is not
peak browser RSS; this host has neither a 16 GiB cgroup cap nor baseline-laptop hardware.
Per-file size/admission and process-wide scheduling remain open. The separate
cooperative cancellation implementation does not impose a hard deadline on a long parse.

## Work before compiler-context expansion

Bounded outline DOM and verified plan compatibility are implemented. Linux small-source
engine attempts fell from 128–153 ms to 42–45 ms. Merging/hashing still scales with
snapshot size, and full source/outline responses remain large. Keep the follow-up's
measurements separate from the initial cold/reparse matrix.

Cooperative built-in parser/extraction cancellation is implemented and measured. It
preserves old publications and resets interrupted parsers; necessary complete edits
still require analysis and transfer.

Bounded syntax retention is implemented as an opt-in [shared cache](../prototype/SYNTAX_RETENTION.md).
The [repeated-edit report](../../experiments/retention/RESULTS.md) measures faster large
headers and an ordinary-edit recovery outlier. Continue working-set and hardware
validation before choosing a default.

The [shared-source measurements](../../experiments/shared-source/RESULTS.md) reduce
retained memory while preserving reads and hashes. PyTorch cached loading is slower
in that paired run; capture-phase timing now includes hashing/index construction.

1. Qualify admission policy with multiple active large files and baseline hardware.
   Keep cache weight separate from parser scratch space and whole-process memory.
2. Continue capture/admission design with bounded working inputs and consider the
   serial preparation cost of new source. [Verified source sharing](../prototype/CAPTURED_SOURCE.md)
   now removes duplicate byte/line-index storage; different old/new contents can still
   coexist. Full recovery still reads/hashes the entire selected scope.
3. Continue overview and first-use identity-index work after
   [bounded document delivery](../prototype/WINDOWS.md). The
   [windowed measurements](../../experiments/windows/RESULTS.md) remove bulk browser
   source/list transfer; necessary indexing and full source verification remain.

[Private periodic recovery](../prototype/RECOVERY.md) now reserves one parser worker
while foreground hints continue. Its [follow-up](../../experiments/recovery/RESULTS.md)
meets the ordinary-edit probe target on this host and retains independent clean-capture
agreement. Full verification still takes about 1.1–1.3 s Linux and 0.19–0.25 s PyTorch
in the sampled accepted passes. Perfetto's large edit takes 1.34 s without tree reuse;
large-file admission and working sets remain unresolved. One-worker hosts remain serial.

The [working-set investigation](../../experiments/working-set/RESULTS.md) reproduces
Perfetto state loss under the 512 MiB policy and shows batch bypasses even at 1 GiB.
Four idle states weigh about 436 MiB, but old/new overlap and maximum replacement
reservations require additional headroom. Investigate demand-based allowances while
preserving accounting; increasing the default has not been justified. This direct
engine probe includes correctness verification in its process peaks and does not
repeat browser or baseline-laptop qualification.

The [replacement-allowance follow-up](../../experiments/allowances/RESULTS.md) now
selects known-state headroom from observed weight. Both 1 GiB three-file batches reuse
all three trees without bypass, with observed Linux/PyTorch times of 180/675 ms.
At 512 MiB, bypasses fall from two to one but Perfetto's final edit remains cold at
1.50 s. The growth margin is provisional; first-use admission, real old/new overlap,
whole-process limits and baseline hardware remain unresolved. Retention stays disabled
by default and its quota/maxima are unchanged.

Do not replace semantic correctness checks with these performance targets. Language
assignments, partial grammar coverage, source-only cases, cancellation, cache validity,
and immutable publication must remain explicit and independently tested.
