# Allocate independently of a file undergoing cleanup

The [foreground measurements](../../experiments/foreground-maintenance/RESULTS.md) show
403–442 ms Linux gate waits during physical truncation. Separate active allocation from
the file being cleaned without moving live bytes or releasing disk charges prematurely.

## Choice and alternatives

Use two fixed anonymous backing files and one staging file. A single serialized maintenance
operation marks one backing file unavailable, snapshots its safe cut, then performs physical
IO outside global allocation state. Captures reserve in the other file. Existing reads and
already-reserved writes stay valid within the cut. No source identity or portable reader
change is required; each descriptor retains its file index and owned extent.

Merely releasing the old mutex is insufficient: extending a file above an in-flight cut
can lose new data. A cut fence on one file cannot supply append space when the live prefix
has no usable holes. A resident bypass would require a separate memory-admission/lifetime
policy and accounting changes. Two files address one serialized cleanup with a fixed five
handles per spool (two cursor/maintenance pairs plus staging), rather than per-source files
or an unbounded generation list. This remains an ephemeral adapter, not a durable engine.

## Capacity and maintenance

The aggregate charged spans of both backing files must never exceed the configured quota.
Staging retains its separate equal quota. A cleaning file keeps its full old charge until
successful physical cleanup; failure retains that charge and permits retry. Headroom is
required for progress elsewhere. With no headroom or no contiguous fit, capture refuses
capacity promptly while old publications remain readable.

Before capture/commit, drain descriptor retirements into extent indexes without physical
IO. Released tail bytes remain charged and can be overwritten in a non-cleaning file.
Explicit maintenance serializes physical cleanup, drains retirements, makes a file
unavailable and releases global state for metadata/truncation. New reservations cannot
enter that file until completion. On completion, update its charge and availability
under state. A guard restores availability without releasing charge on failure/unwind.
Maintenance visits at most both files once per call; later retirements wait for another
pass. Automatic capture cleanup never invokes physical truncation.

Keep cumulative store/read/write accounting. Report aggregate charged/allocated/free spans,
actual total file length and cleaning status separately. Free charged bytes may be in a
cleaning file and unavailable. Interior fragmentation, retained pins and pending physical
cleanup can still exhaust admission. No hard filesystem/capture deadline is promised.

## Verification and evidence

Start with a paused-truncation regression showing capture completes before cleanup is
released. Test quota refusal without premature credit, both file indexes, live/reserved
ranges, retirement routing, errors/retry/unwind, empty inputs, repeated rotation and
final zero-length backing. Retain existing independent extent-model and source integrity
tests. Run workspace/doctest/fmt/Clippy/browser checks plus complete affected mutation scopes.

Adapt the existing qualification harness to allow readiness before cleanup ends; keep
independent timing/byte/fact/pin validation. Compare preceding/current implementations with
the identical updated harness, separate build outputs and verified binary/source hashes.
Repeat the controlled Linux/PyTorch edit workload and source-history capacity measurements
without competing jobs. Distinguish finite warm observations from broader hardware/UI
qualification. No unsafe code, new dependencies or production background thread is needed.

## Delivery evidence

The [comparison and replay](../../experiments/independent-capture/RESULTS.md) show median
during-cleanup publication of 62.4 / 33.3 ms versus 418.6 / 101.5 ms with the preceding
backend. All six edits publish before observed truncation ends; post-cut source/fact/pin
checks pass. Full history remains within shared quotas and cleans both files to zero.
Full relevant mutation scopes have no survivors, with timeouts/build failures separated.

The implementation retains charge on failed/unwound cleanup and refuses capacity instead
of waiting when there is no safe fit. Automatic capture never truncates; explicit idle
maintenance and broader interaction/hardware qualification remain follow-up work.
