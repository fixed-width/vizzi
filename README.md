# Vizzi

Coordinated views for exploring source code and, eventually, reviewing changes.
The current prototype links a source file map, captured source, and a lexical declaration
outline. A native Rust engine supplies immutable observations to a local browser host.
Rust/Python are selected by default; explicit scopes enable raw C/C++ analysis.

The production renderer, compiler-backed language integrations and storage engine
remain open decisions. The review workflow is specified and under development.
The Canvas2D overview is a reference view for
validating coordination; it does not qualify large-codebase GPU performance.

Next delivery follows the [product milestones](docs/plans/product-milestones.md):
explain a real subsystem, then review a local change with surrounding context.
The [view-selection rationale](docs/explanation/task-driven-views.md) connects these
tasks to the literature. Further performance work is conditional on a workflow blocker.

The [baseline learning walkthrough](experiments/learning-workflow/RESULTS.md) records
the task and its return-context gap. [Investigation navigation](docs/prototype/NAVIGATION.md)
now restores that context; the [updated recording](experiments/learning-workflow/NAVIGATION_RESULTS.md)
demonstrates the same task with Back/Forward controls.

The [review fixture](experiments/review-workflow/README.md) and
[comparison contracts](docs/prototype/COMPARISON_PRODUCT.md) now fix the next workflow:
compare a pinned Git baseline with observed working files, including unstaged edits
and untracked text. Portable inventory comparison and side-specific source binding are
implemented, along with [bounded textual hunks](docs/prototype/TEXT_DIFF.md).
Git acquisition, admission and the guided review interface remain next.

- [Run the workspace](docs/how-to/run-workspace.md)
- [Next product milestones](docs/plans/product-milestones.md)
- [Why these coordinated views](docs/explanation/task-driven-views.md)
- [Baseline learning walkthrough and recording](experiments/learning-workflow/RESULTS.md)
- [Investigation navigation and updated walkthrough](experiments/learning-workflow/NAVIGATION_RESULTS.md)
- [Contextual review behaviour](docs/prototype/COMPARISON_PRODUCT.md), [technical contract](docs/prototype/COMPARISON_TECH.md) and [frozen fixture](experiments/review-workflow/README.md)
- [Architecture and scope](DESIGN.md)
- [First-slice behaviour](docs/prototype/PRODUCT.md) and [technical contract](docs/prototype/TECH.md)
- [Host API reference](docs/prototype/API.md)
- [Bounded document windows](docs/prototype/WINDOWS.md)
- [Change-notification contract](docs/prototype/CHANGES.md)
- [Incremental-capture contract](docs/prototype/INCREMENTAL_CAPTURE.md) and [periodic recovery](docs/prototype/RECOVERY.md)
- [Captured document and snapshot identities](docs/prototype/IDENTITIES.md)
- [Shared captured-source ownership](docs/prototype/CAPTURED_SOURCE.md)
- [Fallible source reads and private backing](docs/prototype/SOURCE_READS.md)
- [Language adapters and shared worker budget](docs/prototype/LANGUAGES.md)
- [Cooperative analysis cancellation](docs/prototype/ANALYSIS_CANCELLATION.md)
- [Bounded syntax retention](docs/prototype/SYNTAX_RETENTION.md)
- [Analysis input admission and deferred coverage](docs/prototype/ADMISSION.md)
- [Source selection and policy identity](docs/prototype/SOURCE_POLICY.md)
- [Verified capture plans](docs/prototype/CAPTURE_PLAN.md) and [bounded outlines](docs/prototype/OUTLINE.md)
- [Raw C/C++ extraction](docs/prototype/C_FAMILY_SYNTAX.md)
- [C/C++ policy](docs/prototype/C_FAMILY_POLICY.md), [rationale](docs/explanation/c-family-analysis.md), and [delivery plan](docs/plans/c-family.md)
- [Application measurement procedure](experiments/prototype/README.md)
- [Browser validation and measurements](experiments/browser/README.md)
- [Verified incremental-capture measurements](experiments/incremental/RESULTS.md)
- [Mixed-language adapter measurements](experiments/languages/RESULTS.md)
- [Source-policy validation and measurements](experiments/languages/SOURCE_POLICY_RESULTS.md)
- [Raw C/C++ validation and scoped measurements](experiments/languages/C_FAMILY_RESULTS.md)
- [Full-repository qualification](experiments/qualification/RESULTS.md) and [provisional budgets](docs/performance/BUDGETS.md)
- [Bounded-outline and incremental-edit measurements](experiments/qualification/RESPONSIVENESS_RESULTS.md)
- [Parser cancellation measurements](experiments/qualification/CANCELLATION_RESULTS.md)
- [Repeated-edit syntax-retention measurements](experiments/retention/RESULTS.md)
- [Shared-source capture memory measurements](experiments/shared-source/RESULTS.md)
- [Windowed source/outline delivery measurements](experiments/windows/RESULTS.md)
- [Foreground edits during periodic recovery](experiments/recovery/RESULTS.md)
- [Retained syntax across mixed working sets](experiments/working-set/RESULTS.md)
- [Replacement-allowance validation and measurements](experiments/allowances/RESULTS.md)
- [Source availability under input admission](experiments/admission/RESULTS.md)
- [Captured-source storage and ownership measurements](experiments/source-storage/RESULTS.md)
- [Verified reads and immutable-backing measurements](experiments/source-reads/RESULTS.md)
- [Streaming source capture measurements](experiments/stream-capture/RESULTS.md)
- [Verified unchanged-source reuse measurements](experiments/verified-reuse/RESULTS.md)
- [Retired-source reclamation measurements](experiments/reclamation/RESULTS.md)
- [Pinned reads during maintenance](experiments/readable-maintenance/RESULTS.md)
- [Foreground edits during cleanup](experiments/foreground-maintenance/RESULTS.md)
- [Independent capture during cleanup](experiments/independent-capture/RESULTS.md)
- [Automatic idle maintenance](experiments/idle-maintenance/RESULTS.md)
- [Watching, recovery and cleanup together](experiments/interactive-maintenance/RESULTS.md)
- [Combined engine/browser memory observations](experiments/memory-envelope/RESULTS.md)
- [Earlier performance experiments](experiments/performance/README.md)
- [Contributor guidance](AGENTS.md)

Implementation uses Rust 2024 with unsafe code denied. The browser adapter uses plain
JavaScript, Canvas2D, and DOM controls, with no external assets or runtime dependencies.
