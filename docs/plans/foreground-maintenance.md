# Measure foreground edits during cleanup

Pinned reads are isolated from allocation state, but capture still needs it. Measure
foreground source publication during real cleanup before selecting a scheduling change.
This stage adds qualification instrumentation and a probe, without changing production
maintenance, host scheduling, retention or quotas.

## Workload and overlap

Use the real Host/Engine/PendingRefresh/Application path over marked owned Linux/PyTorch
copies. Warm compatible fact caches before timing. Wait for initial verification, then
use an hour-long recovery interval and explicit path hints with native watching disabled.
This isolates cleanup from periodic recovery and watcher timing.

Prepare a generated streamed tail after host setup, outside project source. Retire it
and observe the actual truncate entry while allocation state is held. Compare edits sent
after completed cleanup with edits sent immediately after that entry. Record cleanup
start/end and reject ambiguous overlap claims; do not add a sleep to real truncation.
Alternate condition order across repeated samples and restore source between attempts.

Use one small ordinary source file, adding a unique declaration to require real analysis.
Verify published bytes and the expected declaration, compare facts with a fresh analyser,
preserve the old source pin and restore the baseline identity. The synthetic tail is a
controlled pressure fixture, not repository code or a claim about typical retirement size.

## Instrumentation

Qualification-only capture marks record entry and readiness after writer acquisition and
initial maintenance. Their interval includes writer/reclamation gating, not only disk IO.
Engine capture and total durations distinguish later analysis/merge work. Record write,
hint and observed publication times on the same monotonic clock. Request-to-entry includes
debounce, queueing and path preparation; publication time includes polling uncertainty.

A qualification-only truncate observer notifies the probe immediately before real set_len.
It must perform only short in-memory reporting. Deterministic tests may pause there to
validate interval recording. Default builds contain neither observer nor capture trace.
No unsafe code, dependency updates or new public host configuration are needed.

## Validation and limits

Use TDD for capture marks, blocked capture intervals, real host publication, expected facts,
old pins, source restoration and invalid ownership/configuration. Run workspace/doctest/
fmt/Clippy/browser checks and relevant mutation scopes before release measurements.

Measure three samples per condition/profile under an explicit child address-space limit.
Preserve raw intervals, code/binary hashes, source hashes, tail size and discarded/no-overlap
samples. Samples are finite native observations, not browser tails or baseline-laptop proof.
Source cache preparation, pressure setup and independent verification are outside the
write-to-publication interval; their allocations still contribute to process peaks.

## Evidence and next design

The [measurements](../../experiments/foreground-maintenance/RESULTS.md) observe all six
requested capture overlaps. Median publication latency is 464.8 / 95.5 ms during cleanup
versus 58.8 / 33.2 ms after it. Median gate waits are 409.5 / 64.1 ms; post-capture analysis
and merge stay broadly similar. Source/fact/pin/restoration checks pass and stores return
to zero file length. Full relevant mutation scopes have no survivors with timeouts and
compiler failures reported separately.

The result justifies a design for foreground capture that does not wait for an in-flight
physical truncate. Do not simply release allocation state: new ranges could cross the cut.
Compare bounded active backing independent of cleaning files and explicit foreground
admission, accounting disk and memory until actual cleanup completes. No production
scheduling/storage change is included in this measurement stage.
