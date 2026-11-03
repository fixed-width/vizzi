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

[Input admission](../prototype/ADMISSION.md) now bounds opted-in active analysis input
across foreground/recovery work and reports deferred analysis without hiding source.
The [workflow report](../../experiments/admission/RESULTS.md) covers structural edits,
a paste, an unseen file and a batch under an 8 GiB child virtual address-space limit.
Eligible deferrals recover without another edit. This input-volume bound and OS address
limit do not enforce parser heap, capture/snapshot storage, physical RAM or a 16 GB
laptop envelope. Keep input admission disabled by default pending broader calibration.

The [source-storage investigation](../../experiments/source-storage/RESULTS.md) isolates
buffers without analysis: one Linux source/index set is about 1.805 GiB; whole-source
version churn retains 7.268 GiB in publication and 9.073 GiB with an expired external
query handle. Churn across three selected files is much smaller. The next
[bounded-read interface](../plans/bounded-source-reads.md) must preserve immutable backing
and explicit read/load errors; history eviction alone cannot cap current data or active
query handles. Buffer capacities and post-release RSS remain distinct, and no new
production source-memory limit is enforced.

The [reader/backing follow-up](../../experiments/source-reads/RESULTS.md) implements
fallible verified loads/windows and an opt-in anonymous session spool. Current-code
whole-source history peaks change from 9.156 to 2.339 GiB Linux and 1.163 to 0.318 GiB
PyTorch, with higher capture/validation IO cost. Source pins and admitted workflows pass.
Those observations predate streaming; default capture remains resident.

The [streaming-capture follow-up](../../experiments/stream-capture/RESULTS.md) bounds
transient input/scan/copy byte buffers for opt-in spooling. A 512 MiB generated file
peaks at 515.0 MiB RSS with whole-vector capture versus 3.25 MiB streamed. Source-only
Linux/PyTorch history peaks are 2.263 / 0.307 GiB, with preserved old pins and restored
bytes. Unchanged capture regresses to 4,010 / 611 ms versus historical backed observations
of 1,463 / 217 ms; investigate avoiding unnecessary candidate metadata construction while
still verifying actual contents. Committed and staging files each have the configured
allowance, so combined lengths may reach twice it. Resident indexes, parser heap,
response memory, cumulative disk reclamation and baseline hardware remain unresolved.

The [verified reuse follow-up](../../experiments/verified-reuse/RESULTS.md) now verifies
incoming bytes and actual EOF before rebuilding metadata. In the repeated before/after
history, unchanged capture falls from 3,674 to 2,172 ms Linux and 569 to 263 ms PyTorch,
with unchanged owned buffers and process peaks near 2.263 / 0.307 GiB. Opposite-order
512 MiB fixture pairs also improve unchanged capture. Full staged IO remains and changed
inputs add speculative hashing; these finite samples do not qualify timing tails or
baseline hardware. Reclamation and metadata/process budgets remain separate work.

The [reclamation follow-up](../../experiments/reclamation/RESULTS.md) completes source
history within 8 GiB Linux / 1 GiB PyTorch reserved backing spans despite cumulative writes
of 9.018 / 1.124 GB. Peak spans are 7.006 / 0.873 GiB and final file lengths return to
zero with the spools still open. External pins preserve their ranges; gaps are reused
and free tails truncated. Maintenance is deferred outside publication locks, but the
final large cleanup takes 801 / 122 ms and currently holds the backing IO mutex. Isolate
or subdivide maintenance before claiming interactive read latency. Fragmentation,
compaction, metadata/queue memory and the full process envelope remain open.

The [maintenance-read follow-up](../../experiments/readable-maintenance/RESULTS.md) removes
the allocation lock from pinned backing reads. During Linux/PyTorch real cleanup, 319 / 57
bounded reads complete versus zero with the preceding locks. Maximum observed overlapping
read latency changes from 737 / 108 ms to 0.120 / 0.131 ms. Cleanup still takes 682 / 124 ms
and can block allocation/captures/diagnostics. These warm-prefix, finite source-only samples
do not qualify browser latency, filesystem deadlines, other platforms or baseline hardware.

The [foreground-cleanup measurement](../../experiments/foreground-maintenance/RESULTS.md)
keeps scheduling unchanged and isolates ordinary explicit-hint edits. Median native
publication latency rises from 58.8 / 33.2 ms after cleanup to 464.8 / 95.5 ms during
generated-tail cleanup. Median writer/initial-maintenance gates take 409.5 / 64.1 ms,
while parsing/merge and debounce remain similar. All six requested overlaps are observed.
Design foreground capture independently of in-flight truncation while preserving reservation,
pin and aggregate capacity safety. These three-sample conditions are not browser tails
or a representative distribution of retired tails.

The [independent capture implementation](../../experiments/independent-capture/RESULTS.md)
routes allocations away from a cleaning file while retaining its full aggregate charge
until IO succeeds. In the identical-harness comparison, median during-cleanup publication
changes from 418.6 / 101.5 ms to 62.4 / 33.3 ms, and all six new-backend edits publish
before truncation ends. History still completes within 8 / 1 GiB shared backing allowances
and returns both files to zero. Progress requires headroom/contiguous fit; capture performs
only logical retirement, with physical cleanup explicit. These finite warm native samples
do not qualify full-capacity pressure, browser tails, cold storage or baseline hardware.

The [idle-maintenance implementation](../../experiments/idle-maintenance/RESULTS.md) admits
one cleanup worker after a 250 ms quiet interval, using metadata hints and a 100 ms wake
ceiling. Each observed host completes eight operations without failure; all six requested
overlap edits publish during real truncation. During-cleanup medians are 67.5 / 33.6 ms
with gates below 0.01 ms. Retry backoff and joined shutdown are tested; the 1–97 ms
retirement-to-entry observations follow already-idle pressure setup and do not calibrate
the quiet timeout. Generated tails, warm native samples and nonblocked shutdown do not
qualify browser/recovery/cold-storage tails, physical RAM or baseline defaults.

The [combined interactive qualification](../../experiments/interactive-maintenance/RESULTS.md)
enables native watches and two-second recovery with automatic cleanup. All 18 edits pass
source/fact/pin checks, and all six cleanup-overlap edits publish before truncation ends.
During-cleanup medians are 56.0 / 31.2 ms; recovery-observed burst medians are 57.6 / 31.0 ms.
Each burst shares one recovery cycle. Cleanup admission takes 2.49–2.70 s Linux and
68–621 ms PyTorch; these are not cleanup deadlines. Warm finite native samples preserve
the physical-RAM, sustained-pressure and browser/baseline qualification gaps.

Do not replace semantic correctness checks with these performance targets. Language
assignments, partial grammar coverage, source-only cases, cancellation, cache validity,
and immutable publication must remain explicit and independently tested.
