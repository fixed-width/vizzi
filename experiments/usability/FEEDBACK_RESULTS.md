# Maintainer feedback validation

This record is for contributors assessing the response to [the first maintainer
findings](FINDINGS.md), after viewer `66ebde8` and trial kit `a59890e`. The
[behaviour](../../docs/prototype/TRIAL_PRODUCT.md) and
[implementation](../../docs/prototype/TRIAL_TECH.md) contracts describe the resulting
interface. These checks do not establish unfamiliar-user comprehension or productivity.

## Verified behaviour

- M1/M9/M10: coverage labels no longer duplicate syntax; ordinary debug footers are
  removed; equal file counts collapse, while capped maps and paged lists expose their
  actual visible/total counts. Debug measurements remain available to the harness.
- M2/M3: Rust, Python, C and C++ use grammar queries over the existing captured parse.
  Immutable styles clip to UTF-8 fragments without reparsing on scroll. Types/callables
  have labelled badges; Line N and the accessible reveal label explain the destination.
- M4/M5: directory cards pack across both dimensions, including mixed directory sizes.
  Overview jumping, explicit zoom and retained wheel zoom complement panning. Pointer
  and keyboard separators resize exploration/review and outline/source sections;
  bounded numeric preferences persist, with usable narrow layouts.
- M6/M7/M8: repository path means a file/directory scope. A separate Two commits mode
  pins both requested revisions, ignores dirty working files, labels Target, and hides
  working refresh. Invalid revisions and empty non-root scopes report beside the form
  and retain the active pair. Deleted-only and metadata-only paths remain valid scopes.

Native tests include moved refs, equal commits, invalid targets/lengths, source identity,
cache encoding, all style classes, Unicode/multiline fragments, shared style admission
across rows, and a full 1-MiB index. Adversarial escaped source/names plus 1,024 style
spans remain below the 512-KiB document-window bound. Browser checks exercise actual
native capture, literal styled DOM, errors, both comparison modes, resize persistence,
overview navigation, selection/history/refresh and 600-pixel layouts.

Formatting, workspace tests including doctests, and Clippy with all targets/features
passed. The workspace has **422** passing tests. The complete browser runner has
**188** passing Node/Chromium cases, plus startup, review, refresh and feedback flows.
The guided lpc-rs learning replay and both frozen review replays completed with captured
source/bounds checks. Learning verified one coherent snapshot and literal visible rows;
one cancelled/navigation-time response body could not be archived by Playwright.
That audit loss is separate from page errors (none) and successful rendered-row checks.
The replays used known paths/answers; they are functional regressions, not usability trials.

## Mutation evidence and limits

[Compact mutation outcomes](results/feedback-mutations.json) retain raw-report hashes,
source hashes and individual outcomes. All executed cohorts passed unmodified baselines.
Native runs used `CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo mutants`,
`-j 1 --jobserver-tasks 1`, offline Cargo, and serial tests. JavaScript used serial
Node/Chromium commands with the Stryker instrumenter, unmodified/instrumented baselines
and an activation control. No mutation exclusions were applied.

Overlapping runs are reconciled by mutant identity, rather than adding reruns:

| Scope | Caught by failing tests | Survived | Build failures | Timeouts |
| --- | ---: | ---: | ---: | ---: |
| Core colouring/projection/layout/traversal | 232 | 11 | 17 | 2 |
| Comparison application | 41 | 0 | 18 | 6 |
| Camera/presentation JavaScript | 149 | 2 | 0 | 0 |

Stronger tests caught missing style categories, UTF-8 boundary validation, malformed
cache ordering/adjacency, exact index admission, aggregate window admission, an empty
range, mixed-size layout overflow, retained parser state and contextual HTTP recovery
status. The remaining core survivors were inspected:

- Packing OR→XOR is equivalent because checked offsets reserve four zero low bits.
  Removing the fragment-boundary guard is redundant for validated immutable documents
  and the character-aligned source reader.
- A binary-search shift mutation still returns the expected clipped styles for the
  tested workloads but scans more preceding data. This is a performance-test gap,
  not a caught mutation or a claim of equivalence.
- Six shelf/area heuristic variants still satisfy the tested balance, containment and
  non-overlap requirements. They can change geometry; tests do not pin incidental
  coordinates or prove equivalent layouts on every repository.
- Two cancellation-guard variants survive the monotonic-predicate and publication
  checks. Checkpoint timing is not exhaustively covered; no claim of equivalence for
  every cancellation interleaving is made.

The two JavaScript survivors remove/change the fatal-decoder option. All inputs reach
that renderer after UTF-8 range validation; valid admitted fragments behave identically.
Timeouts include non-terminating traversal/UTF-8 rounding and disabled lifecycle workers.
Build failures are unviable generated variants. Neither is counted as caught by a test.
An initial mutation setup named a nonexistent `python` test target; no mutants ran in
that attempt. The corrected target was `languages`. This evidence is not full coverage
or a proof of correctness.

## Performance and artifacts

Optimized measurements ran after correctness and mutation work finished, with three
sessions per established full-checkout fixture and one parser worker:

| Observation | Linux `kernel/sched` (54 paths) | PyTorch `torch/_inductor/codegen` (109 paths) |
| --- | ---: | ---: |
| Median first comparison | 865 ms | 931 ms |
| Median edited source visible after explicit refresh | 347 ms | 215 ms |
| Observed rAF p99 | 16.8 ms | 16.8 ms |
| Maximum sampled native high-water RSS | 94.4 MiB | 68.8 MiB |
| Maximum sampled projection payload | 64,365 bytes | 86,472 bytes |
| Maximum sampled rendered syntax spans per pane | 88 | 60 |

All **30 refreshes** retained the edited file. Source, outline and style bounds passed;
proof windows reported ready colouring for C/C++/Python. The original checkouts were
unchanged and live workspace indexing stayed dormant. The preceding refresh cohort
observed 366 / 199 ms for edited source, the same 16.8-ms rAF p99, and about 83.5 / 53.6
MiB native peaks. These are separate small cohorts with uncontrolled external host
activity; they do not demonstrate a causal speedup or a tail guarantee. Styling adds
retained metadata and query work. RSS excludes Git children; renderer heap samples are
not whole-browser memory. Sampling rAF does not measure physical presentation.

[The validation record](results/feedback-validation.json) contains timing distributions,
raw-report and binary hashes, source fingerprints, bounds and replay summaries.
Full screenshots, traces, videos and raw measurement reports remain in local
`/tmp/vizzi-trial-{learning-check,review-a-check,review-b-check,linux-measure,pytorch-measure}`
directories. Their raw hashes are retained for correlation; those temporary artifacts
are not a durable publication.

The baseline laptop, other browsers/operating systems, full-repository review and
independent unfamiliar-user acceptance remain separate qualifications. A maintainer
should retry the reported interactions with the updated viewer before closing usability
acceptance.
