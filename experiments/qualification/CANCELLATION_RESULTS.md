# Cancellation during parsing and extraction

The later [retention report](../retention/RESULTS.md) implements bounded owned-tree
reuse and records the remaining capture/transport work.

Built-in analyzers now stop cooperatively inside a file, using safe Tree-sitter
progress callbacks and bounded polling during fact extraction. On the full Linux and
PyTorch profiles, request-to-engine-return time was **41–46 ms** and **18–23 ms**,
respectively. Selected pinned source remained consistent and obsolete results did not
publish. These observations do not impose a hard deadline or make a necessary complete
large-file edit instantaneous.

## Implementation and scope

The [analyzer contract](../../docs/prototype/ANALYSIS_CANCELLATION.md) adds
`analyze_cancellable`: complete facts or an abandoned attempt. Existing adapters keep
their complete operation with default checks before and after it; all four built-ins
also interrupt parsing and extraction. Interrupted parsers reset before another input.
Any worker's abandoned result discards the entire refresh, preventing publication of
a snapshot with a missing file. Partial facts never reach validation or persistence.
Completed facts from other files can still warm the disposable cache.

The optional qualification wrapper forwards cancellation instead of reverting to
whole-file analysis. It now records the cancellation request and engine return using
one monotonic clock across threads. That direct interval includes worker joining and
disposal of abandoned captured inputs; it excludes the query loop's delay before
noticing completion. Earlier reports retain their observed completion interval.

The same [profiles](profiles.json) cover 66,066 Linux files / 1,495,484,792 bytes and
10,578 PyTorch files / 186,092,681 bytes. PyTorch retains 431 source-only CUDA files.
Header assignments remain explicit raw-language inputs, not compiler configuration.
Archives are read-only inputs; no analyzed project is built or executed.

Runs use four workers on the same i7-13700H, 20 logical CPUs, about 62.5 GiB RAM,
warm OS caches, and Rust 1.99.1. Current-context caches were prepared before timing;
preparation is excluded. Each timed run loads a baseline from that cache, retains its
snapshot, then uses a separate empty phase cache and revised analysis context to force
actual parsing. There are three cancellation and three complete reparse samples per
profile, with no concurrent builds, tests, mutations, or other benchmark jobs.

Cancellation is scheduled 100 ms after the first observed analyzer entry, at the query
loop's cadence. Actual request delays are 136–175 ms for Linux and 133–137 ms for
PyTorch. During work, the query loop serialises a bounded 2,000-tile view and checks
selected old source. The controlled probe omits watcher scheduling and browser work.

## Cancellation and complete reparse

| Observation, four workers | Linux | PyTorch |
| --- | --- | --- |
| Earlier observed request-to-completion range | 714–845 ms | 2,027–2,265 ms |
| Follow-up observed request-to-completion range | 86–96 ms | 60–62 ms |
| Follow-up direct request-to-engine-return range | 40.9–45.9 ms | 17.8–23.2 ms |
| Earlier median complete reparse engine time | 60.385 s | 11.293 s |
| Follow-up complete reparse engine range | 53.999–56.583 s | 9.603–9.763 s |
| Follow-up median complete reparse engine time | 55.198 s | 9.644 s |
| Maximum sampled reparse process RSS | 6.479 GiB | 1.135 GiB |
| Largest per-run query p95 during reparse | 35.25 ms | 9.34 ms |

The observed interval remains available for comparison with the
[initial report](RESULTS.md); it includes query-loop observation delay and parent
event collection. The direct interval is a different endpoint and must not be mixed
with that interval to compute a speedup. Full reparse measurements include capture,
analysis, cache writes, and snapshot construction while retaining the old observation.
They show no observed throughput regression; previous compatible-snapshot changes and
uncontrolled system activity prevent isolating cancellation overhead from this comparison.

All six cancelled attempts return no refresh, leave the baseline snapshot published,
and preserve the selected pinned source. All six complete reparses publish generation
2. Per-profile baseline identities agree across all runs; reparse identities agree
with each other and differ from the baseline because their analysis context changes.
Existing fixture tests check independently expected declarations and complete/incremental
equivalence; identity agreement alone is not evidence of compiler semantics.

The [compact JSON](results/cancellation-summary.json),
[CSV](results/cancellation-summary.csv), and
[full observations](results/cancellation-observations.json) retain source arguments,
revisions, counters, query samples, events, binary hashes, and host/resource limits.
The [source manifest](results/cancellation-source-sha256.json) identifies the working
tree measured before commit. RSS is sampled around every 50 ms, so peaks are lower
bounds. These runs impose no memory cap and do not qualify the 16 GB laptop.

## Correctness and mutation evidence

The first new interface tests failed before `analyze_cancellable` existed. New tests
cover complete-result equivalence for all four languages, Unicode, recovery, empty
and unsupported input, interruption inside parsing, parser reuse for different source,
abandoned extraction traversal, amortised polling, and legacy adapter/error behaviour.
Native tests verify retry parses the cancelled file afresh, old pins remain coherent,
and an adapter returning no result cannot publish a snapshot with missing files.
Probe tests cover cancellation forwarding, revised contexts, timestamp order, missing
timestamps, units, and separation from query-loop observation time.

Formatting, **149 Rust tests**, doctests, Clippy, and **42 Node/browser checks** pass.
Export checks preserve direct intervals in JSON/CSV and export null for legacy reports
without the new field. No frontend behaviour or rendering backend changes in this step.

The complete affected-module cargo-mutants run covers core analysis/Rust/shared syntax
and native engine/qualification: **157 tested, 128 caught, 24 unviable, five timeouts,
zero survivors**. It uses the existing equivalent directory-prefix comparison exclusion;
there are no new exclusions. The first passing-baseline run had one survivor reversing
the polling cadence, causing almost every operation to poll. A new behavioral test
checks amortisation across many node visits; the final complete run catches it.

Five mutants that prevent complete results leave startup fixtures waiting and time out.
Those are separate from assertion catches. Two additional manual statement deletions,
which cargo-mutants does not generate here, remove parser reset and callback wiring;
both fail their targeted behavioral tests. The exact source is restored after each
and the unmodified workspace suite passes again. The
[mutation report](results/cancellation-mutation.json) records commands, hashes,
non-caught outcomes, manual checks, and scope limits. Dependencies, hash macro expansion,
filesystem delivery, and browser internals are outside this scope.

## Remaining work

Cancellation is cooperative. Tree-sitter callbacks, a collector operation, hashing,
decoding, validation, I/O, allocation, and cleanup do not have a general hard latency
bound. An [upstream report](https://github.com/tree-sitter/tree-sitter/issues/6067)
describes long callback gaps during pathological TypeScript recovery; it is a limit on
general claims, not a measured failure in these four grammars. Timed cancellation
samples during reparsing do not qualify every possible cancellation phase or input.

Persistent syntax-tree reuse remains the next architectural step. Current analyzers
are worker-local and dropped after every refresh, and the trait does not require
transfer between threads. The [delivery plan](../../docs/plans/parser-responsiveness.md)
records session ownership, exact-source/context validation, admission including active
leases, eviction, interruption, and clean-capture requirements. Retaining arbitrary
trees without those contracts would add memory and concurrency risks. Necessary
large-file parsing, complete source/outline transport, and full-capture memory overlap
remain open performance work.
