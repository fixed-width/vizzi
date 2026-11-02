# Bounded outlines and cheaper compatible edits

The later [cancellation follow-up](CANCELLATION_RESULTS.md) implements interruption
within parsing/extraction and records the remaining hot-tree ownership work.

This follow-up addresses the two interactive bottlenecks found in the
[initial qualification](RESULTS.md): declaration DOM growth and repeated validation of
unchanged snapshot metadata. On the same full Linux/PyTorch profiles, the large C++
outline opens about eight times faster and ordinary edits are faster. Large-file
reparsing, cancellation, and resource admission still need work.

## Changes and measurement scope

The browser [outline](../../docs/prototype/OUTLINE.md) renders visible rows with
overscan, capped at 200 controls including a focus pin. Logical keyboard navigation
reaches all declarations. Stable file/document-version keys avoid rebuilding its
index for selection/filter requests; changed versions preserve surviving scroll and
focus identities. Selected source responses now include that additive version field.

A portable [capture plan](../../docs/prototype/CAPTURE_PLAN.md) freezes policy and full
analyzer descriptors. Only validated snapshots receive its private compatibility
marker. The native host checks that marker before retaining unread files, validates
changed documents, and uses binary path lookup. Generic, incompatible, or invalid
baselines still force full discovery. Snapshot composition is now v4 and includes the
marker; document composition remains v2. Analyzer fingerprints invalidate old cache
contexts once, so these runs use separately prepared current-context caches.

Profiles, revisions, raw-language assignments, worker count (four), and the
i7-13700H / 62.5 GiB host match the original report. Originals remain untouched;
edits use disposable copies and final snapshots agree with complete captures. Timings
have warm OS caches and no concurrent tests, builds, mutation jobs, or benchmarks.
Periodic full recovery remains enabled in live native and browser runs.

Native edits have three samples per profile. Browser PyTorch repeats three four-second
steady runs and three refresh runs; Linux has one four-second run per condition. Each
browser profile has five ordinary edits and one large-file open/edit observation.
Headless Chromium 156 uses software graphics at 1440×900. Collection occurred on
2026-11-02 UTC (November 1 locally). Reports record the parent commit plus binary/app
hashes because measurements precede the implementation commit; the
[source manifest](results/responsiveness-source-sha256.json) identifies measured sources.

The browser driver now records a fresh UUID in every run's edit probes. This prevents
seeded caches from satisfying a supposedly fresh parse with an earlier run's identical
comment. `--probe-id` permits deliberate replay. An intermediate frontend-only run
reused one such cache entry; its edit latency is excluded here. Later status metadata
can still describe a no-op recovery rather than the parse that caused visible change.

## Ordinary edits

Native times below use `kernel/locking/mutex.c` and `c10/core/Device.cpp`, respectively.
All six follow-up edits read and parse exactly one file, have zero persisted-cache
hits, and match a separate complete capture.

| Case | Initial qualification | Follow-up |
| --- | --- | --- |
| Linux input capture/validation | 56.4–71.9 ms | 0.23–0.52 ms |
| Linux total engine attempt | 127.5–152.6 ms | 42.2–45.5 ms |
| Linux write to coherent stdio response | 210.4–261.9 ms | 94.3–201.7 ms |
| PyTorch total engine attempt | 22.3–25.8 ms | 6.3–8.7 ms |
| PyTorch write to coherent stdio response | 63.5–80.3 ms | 46.9–77.4 ms |
| Linux browser edit median / maximum | 268.8 / 2,154.6 ms | 147.6 / 192.1 ms |
| PyTorch browser edit median / maximum | 130.4 / 151.9 ms | 102.6 / 121.4 ms |

The browser uses its picked source file, rather than the two native files above.
Browser timing ends after matching source DOM and two rAF callbacks. Stdio timing
includes serialization and Python decoding. Neither is an engine-only metric.
The initial Linux maximum coincided with recovery; five improved follow-up samples
do not establish a tail or prove recovery can no longer delay edits.

The marker removes repeated classification/context scans, and binary lookup removes
the full baseline path map. Unchanged documents share their existing `Arc` values.
Updates still merge document references, validate workspace/path invariants, and hash
versions across the resulting snapshot. The remaining total CPU work is linear in
file count; this change does not make all update work proportional to changed files.
See [native observations](results/responsiveness-native.json).

## Large files and motion

| Browser observation | Initial qualification | Follow-up |
| --- | --- | --- |
| PyTorch 8.4 MB Perfetto header open | 1,848.8 ms | 229.3 ms |
| Its rendered declaration buttons | 42,443 | 21 |
| Its opening maximum rAF interval | 1,599.9 ms | 33.3 ms |
| Its opening long animation frames | 1,556.8 ms frame | None observed |
| Its edit to DOM plus two rAF callbacks | 3,326.8 ms | 2,109.9 ms |
| Its edit maximum rAF interval | 1,333.2 ms | 83.4 ms |
| Linux 24.0 MB generated header open | 223.5 ms | 190.8 ms |
| Its edit to DOM plus two rAF callbacks | 1,263.3 ms | 1,117.4 ms |

Perfetto's edit still produces one 98.7 ms long animation frame (15.4 ms blocking).
Linux opening/editing produce 77.0/78.2 ms long frames with zero reported blocking;
its largest rAF intervals are 50.0/49.9 ms. Source DOM stays at 56–58 lines. Linux's
selected generated header has no extracted declarations, before or after this change.
These observations separate waiting for native parsing from browser main-thread work.
Perfetto's sampled final metadata is a later no-op full recovery, so it is not used
to attribute the edit's parse duration.

All eight follow-up overview runs have a 16.8 ms rAF p99 and no rAF intervals over
50 ms. Draw CPU p99 is at most 0.8 ms. Linux's steady run does have one 66.6 ms
interval between actual draws; smooth rAF scheduling does not mean every input causes
a draw on the next frame. The map still paints only the bounded 2,000-tile view.

At the final observation, PyTorch renderer heap is 51.6 MiB with 861 DOM nodes,
versus 58.1 MiB and 229,789 nodes in the original run. Linux renderer heap is
63.8 MiB with 693 nodes. Native lifetime high water is 1.198 GiB for PyTorch and
4.458 GiB for Linux. These are descriptive observations, not a repeat of the full
retained-reparse memory matrix or a memory cap. Heap samples are not browser-process
RSS. GPU execution, physical presentation, and the 16 GB laptop remain unqualified.

Both browser reports have empty error lists and complete-capture agreement:
[PyTorch observations](results/responsiveness-browser-pytorch.json),
[Linux observations](results/responsiveness-browser-linux.json),
[PyTorch screenshot](results/responsiveness-pytorch-workspace.png), and
[Linux screenshot](results/responsiveness-linux-workspace.png).

## Correctness and mutation evidence

New core tests first failed before the capture-plan module existed. Behavioral tests
cover full/incremental equivalence, retained pointer sharing and pins, incompatible
and generic baselines, changed descriptor/policy identities including empty snapshots,
and rejection of invalid paths, classifications, contexts, duplicates, and conflicts.
Existing engine tests continue checking conservative discovery and clean-capture
agreement. Formatting, **139 Rust tests**, doctests, and Clippy pass with all features.

The **42 Node/browser checks** include eight real-Chromium outline tests and a native
HTTP integration regression with 50,000 declarations. They cover logical navigation,
native button activation, prevented navigation defaults, actual row/reveal geometry,
roving focus, scroll anchors, disappearing declarations, version reuse, resizing,
empty/unversioned data, literal labels, and selected source after live edits.

The final complete affected-module mutation runs have passing unmodified baselines:

| Scope | Tested | Caught by failing tests | Timeouts | Unviable | Survived / uncovered |
| --- | --- | --- | --- | --- | --- |
| Core capture/model/query; native engine/source | 205 | 160 | 1 | 44 | 0 |
| Complete browser outline module | 222 | 188 | 34 | 0 | 0 |

The native run uses the existing equivalent directory-prefix comparison exclusion;
there are no new exclusions. Its timeout replaces refresh with no result and blocks
startup. UI timeouts include runaway loops and browser hangs, and are reported
separately from assertion catches. The browser harness explicitly propagates Stryker's
active mutant into Chromium; the initial run without that propagation is discarded.
Subsequent survivors strengthened behavioral assertions, including key defaults and
physical geometry. One intermediate attempt failed its unmutated visibility test;
the empty-note margin bug was fixed before later runs were interpreted.

[Native mutation details](results/responsiveness-native-mutation.json) and
[outline mutation details](results/responsiveness-outline-mutation.json) retain scopes,
commands, source hashes, timeouts/unviable results, and limits. These runs do not mutate
CSS, app orchestration, parser dependencies, hash macro expansion, or OS/browser internals.
Earlier controller/notification evidence remains separate. Zero survivors is evidence,
not a proof of correctness or an assertion that timeouts were caught by tests.

## Remaining work

The outline DOM ceiling is now enforced by the component and exercised at 50,000
declarations. Full selected-source/outline responses and changed-version index
construction remain unbounded in data size. Large-file source splitting and transfer
are still material; pagination needs its own coherent snapshot/selection contract.

Next investigate parser cancellation during a file and a bounded cache of hot-file
syntax trees, retaining immutable published facts and clean-capture equivalence.
Then reduce capture memory overlap and define resource admission. The
[provisional budgets](../../docs/performance/BUDGETS.md) remain engineering proposals;
five small edits below 200 ms do not complete hardware or latency qualification.
