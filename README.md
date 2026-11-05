# Vizzi

Coordinated views for learning source code and reviewing changes alongside your editor.
The primary page automatically maps captured source into areas and searchable names.
Click through the overview and keep useful locations without entering paths or lines.
Saved context offers optional questions, notes and reader-authored connections.
VS Code handles normal reading and editing; an explicit
handoff checks working-file content before opening a saved location. A source map,
captured preview, declaration outline and comparison remain supporting views.
A native Rust engine supplies immutable observations to a local browser host.
Rust/Python are selected by default; explicit scopes enable raw C/C++ analysis.

The production renderer, compiler-backed language integrations and storage engine
remain open decisions. Both guided learning and local-change review tasks now pass
on the frozen reference repository. [Focused review measurements](experiments/review-workflow/SCALE_RESULTS.md)
are recorded; independent usability and whole-repository qualification remain open.
The Canvas2D overview is a reference view for
validating coordination; it does not qualify large-codebase GPU performance.

The current [product checkpoint](docs/plans/product-milestones.md) evaluates the
investigation's contribution alongside an editor. Guided functional checks do not
establish that contribution; the editor-only comparison and maintainer trial remain open.

The [CPU repository-question pilot](experiments/qa/RESULTS.md) now supplies a reusable
question library and compares small local models with an external index. Both candidates
fit the observed memory budget, but this retrieval/prompt baseline is not yet a reliable,
fast answer feature. [Reproduce it or extend the questions](experiments/qa/README.md).

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
[Source-only Git acquisition](docs/prototype/REVIEW_ACQUISITION.md) now exercises the
same model, now with [bounded review ownership](docs/prototype/REVIEW_OWNERSHIP.md).
Review analysis now [shares the existing engine](docs/prototype/REVIEW_ANALYSIS.md).
[Comparison transport](docs/prototype/COMPARISON_API.md) now exposes bounded queries and
separate review status. The [review interface](docs/how-to/review-local-changes.md)
now passes [both rendered guided variants](experiments/review-workflow/WALKTHROUGH_RESULTS.md),
with screenshots, recordings and a repeated learning regression.
The [task-driven startup contract](docs/prototype/STARTUP_PRODUCT.md) lets review
open before live indexing; exploration activates the same host engine explicitly.
The [startup measurements](experiments/review-workflow/STARTUP_RESULTS.md) record the
first usable review around 0.8–1.0 seconds from launch in the focused reference scopes.
Working refresh now [preserves reading context](docs/prototype/REVIEW_REFRESH.md);
the [repeated measurements](experiments/review-workflow/REFRESH_RESULTS.md) record
zero edited-file resets across 30 scoped refreshes.
The [human-trial kit](experiments/usability/README.md) prepares maintainer feedback and
later unfamiliar-user evaluation with neutral tasks and separate scoring material.

- [Keep an investigation beside VS Code](docs/how-to/investigate-with-vscode.md)
- [Run the source and review views](docs/how-to/run-workspace.md)
- [Companion behaviour](docs/companion/PRODUCT.md), [architecture](docs/companion/TECH.md), [API](docs/companion/API.md) and [trial procedure](docs/companion/TRIAL.md)
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
