# Companion validation observations

Observed 2026-11-05 after `0968fe4`. This record is for maintainers assessing the first
companion slice, not a usability acceptance claim. The [reproduction procedure](README.md)
and [human comparison protocol](../../docs/companion/TRIAL.md) serve different purposes.

## Functional evidence

The pinned lpc-rs learning workflow now constructs a question, six source anchors,
reader notes and five explicit connections, then reloads that state with the evidence
drawer closed. The reader's interpretation remains separate from captured bytes.
Three source-file SHA-256 values match fixture commit
`1a4f5b65dd2a269298748df2dfd68092a169a8d9` before and after replay.
The recording editor receives `--reuse-window`, `--goto` and the absolute compiler
path at line 140, column 1. No lpc-rs build or script is run.

The artifact is `/tmp/vizzi-companion-lpc-replay-final`: investigation JSON, source-bound
report and screenshot. It contains expected answers and stays separate from the empty
participant investigation. This guided replay knows the answer; no editor-only human
baseline, newcomer comprehension improvement or added product value has been established.

The focused browser flow also passes source chooser → investigation, notes, connection
removal, persistence, an invalid-path error visible inside its dialog, narrow layout and
changed-working-source refusal without a second editor invocation. Launch rejects an
investigation path inside the disposable analysis cache. Source previews mark omitted
lines and respect the 4,096-byte UTF-8 budget, including exact-boundary input.

An attempted native-editor verification opened VS Code on the main desktop: a separate
profile did not isolate the display. GUI verification was stopped. Subsequent checks use
only recording executables/adapters. Actual editor file/caret display is unverified;
the CLI cannot confirm existing unsaved buffers or eliminate changes after verification.
No macOS or Windows editor handoff was validated.

## Verification and mutation evidence

Formatting, 401 workspace tests including doctests, and Clippy over all targets/features
pass. The full browser suite passes; the final focused flow and real-repository replay
pass after the storage and accessible-label fixes. No dependencies or unsafe code were
added. Mutation work used passing baselines, one job/compiler task, serial tests and
`nice -n 10`; no mutation cohorts overlapped.

| Full file cohort and follow-up | Caught by failing tests | Remaining survivors | Build failures | Timeouts |
| --- | ---: | ---: | ---: | ---: |
| Portable investigation, 109 generated | 106 | 0 | 3 | 0 |
| Native companion/editor, 65 generated | 52 | 2 | 10 | 1 |

Totals combine the last full cohorts with focused reruns of strengthened tests without
double-counting mutants. The original core cohort missed four mutations; the initial
native cohort missed 25. Added tests cover unaffected/incoming connections, decoder
identities and byte boundaries, current/changed/missing/unchecked freshness, line bounds,
imported anchor checks, complete large-file verification and literal process success/failure.

Two native survivors remain documented, with no broad exclusions configured:

- Changing the early invalid-line `||` to `&&` still fails validated core admission:
  zero lines are invalid, and clamped out-of-range windows have an end before their start.
- Removing `outline_count: Some(1)` leaves the captured excerpt unchanged. It performs
  extra unused outline work under the existing finite window cap; the explicit budget
  remains in the implementation.

The timeout replaces the decrement in UTF-8 boundary rounding with division, leaving a
non-decreasing loop for the multibyte fixture. It hit the 20-second mutation timeout;
it is not counted as caught by an assertion. Build failures are invalid generated
constructors/defaults and unsupported `Instant` arithmetic, not behavioral detections.
Artifacts: `/tmp/vizzi-companion-{core,native}-mutants-final` and matching `-followup`
directories. HTTP adaptation, CLI configuration and browser controller code are exercised
by tests but were not mutation-tested in this slice. Coverage is evidence, not proof.

## Bounded interaction observation

The [raw measurement](measurement.json) uses the same optimized native binary as the
successful replay, SHA-256
`8532d9d19382337f31122e895296559714df876922e1011ef258420e660b3705`.
Linux x86_64, Intel Core i7-13700H, about 62.5 GiB total host RAM, Chromium
156.0.8078.4, 1600 × 1000 viewport, one analysis worker. No test or mutation cohort ran
during this observation. This machine exceeds the project's 16 GB reference budget.

At 32 cards and 64 synthetic reader links, the investigation response was 31,328 bytes.
Thirty warm detail-open → two-animation-frame observations after five warmups measured
32.1 ms median, 32.4 ms p95 and 32.5 ms maximum, with no page errors.
These include frame scheduling and use repeated source anchors. They do not establish
end-to-end input latency, GPU frame times, large repository performance, independent
human benefit or the cost of the full 512 KiB storage bound. Larger graph scopes remain
outside this deliberately bounded slice.
