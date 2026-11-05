# Discovery-first correction

This maintainer validation record follows `d647a6e`. The previous landing page asked for
known paths and a prewritten explanation, and exposed too many authoring controls. It
could retain an understanding but did not provide a useful starting point for an
unfamiliar reader. The [behaviour contract](../../docs/companion/PRODUCT.md) now starts
with discovery and makes saved interpretation optional.

## Observable behaviour

Opening `/` starts read-only indexing and presents actual source areas, counts,
languages and example names. Area clicks and breadcrumbs explore containment. One
optional input searches declaration names and paths. Results cap at 24 per page with
continuation. A name opens a 12-line/12-declaration preview; **Keep this location** saves
the exact anchor with a derived title. Questions, path entry and connection authoring
are disclosed on request. Existing saved JSON is unchanged.

The native projection reads immutable metadata, not source text. It reports selected
source scope and source-only/deferred coverage. Areas are real directories, not inferred
subsystems; name matches do not establish calls, dependencies or impact. Full reading
and editing stay in the editor. This corrects the starting interaction without adding
an editor plugin, new parser, renderer or dependency.

The pinned 552-file lpc-rs replay starts with one visible text input, finds six reference
declarations through search, keeps them without typing paths/lines/titles, and retains
notes and five reader assertions. Exact source paths/lines and three file SHA-256 values
are audited against the existing fixture. The editor is a recording executable only.
Artifacts are under `/tmp/vizzi-discovery-lpc-replay` (including `overview.png` and the
saved investigation/report). This reference-aware replay is functional evidence;
newcomer usefulness and comparison with an editor-only attempt remain open.

## Verification

Formatting, 407 workspace tests including doctests, Clippy across targets/features and
the complete headless browser suite pass. The new browser flow checks automatic
orientation, one initially visible text input, area/name selection, one-click capture,
recorded editor arguments, saved-state reload, progressive authoring and narrow layout.
The earlier advanced capture and review flows still pass. No real editor window was
opened during this correction.

Serial lower-priority cargo-mutants runs used one job/compiler task, serial tests and
passing unmodified baselines:

| Cohort | Generated | Caught by assertions | Survivors | Unviable | Timeouts |
| --- | ---: | ---: | ---: | ---: | ---: |
| Full portable discovery projection | 48 | 45 | 0 | 2 | 1 |
| Native bridge selection, including generated window-field mutations | 4 | 2 | 1 | 1 | 0 |

The first portable run missed ten mutations. Added tests check byte boundaries,
language/byte aggregates, unique example bounds and deferred counts. The final run has
no survivors. Its timeout changes UTF-8 boundary rounding from decrement to division,
creating a non-decreasing loop; the 20-second timeout is separate from assertion catches.
The two unviable mutants require nonexistent `Default` implementations.

The native survivor removes an unused outline-count hint during capture, already
documented in the [earlier report](RESULTS.md). The only generated bridge-body mutant
was unviable, so the two catches are existing window-field guards, not new bridge-body
coverage. HTTP success and invalid-path handling are tested directly. An initial native
baseline failed because its copied homepage test expected the previous page; it was
updated and the cohort rerun with a passing baseline. Browser controller code was not
mutation-tested. Artifacts: `/tmp/vizzi-discovery-core-mutants-final` and
`/tmp/vizzi-discovery-native-mutants-final`.

## Metadata request observation

The [raw record](discovery-measurement.json) measures optimized native HTTP requests on
the pinned lpc-rs scope, one analysis worker, after tests and mutations stopped. Thirty
warm requests per case follow five warmups. Timing includes request and complete body
delivery over loopback, with JSON parsing afterwards.

| Query | Entries / matches | Response bytes | Median | p95 |
| --- | ---: | ---: | ---: | ---: |
| Root | 15 / 15 | 2,860 | 0.46 ms | 0.67 ms |
| Compiler area | 18 / 18 | 5,154 | 0.32 ms | 0.42 ms |
| `reader` name/path search | 24 / 33 | 8,544 | 1.04 ms | 3.03 ms |

The projection traverses captured files and, for search, lexical facts. These small
repository observations do not qualify Linux/PyTorch scale, browser input latency or
GPU rendering. Response paging is bounded; search/index scale remains a separate
measurement requirement. The earlier capped saved-board measurements are historical,
not a performance claim for this new landing view.
