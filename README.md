# Vizzi

Coordinated visual working sets for learning code and reviewing changes beside your editor.
The current direction is a [Code Bubbles-style workspace](docs/explanation/visual-working-sets.md):
follow a call into a new source fragment while its caller, earlier steps and branches
remain available in a stable spatial layout. Multiple source/relationship/diff views can
share context without replacing each other. Search is a supporting entry point.

The native Rust engine, immutable source observations, bounded previews, source/version
checks, captured call candidates and explicit VS Code handoff are implemented foundations.
The first [visual call-trail prototype](docs/how-to/follow-a-call-trail.md) is available
at `/workspace`: source fragments stay open while callees and branches are added; their
layout, camera and reading state restore locally in the browser. Candidate arrows retain
the analysis limitations. Underlined call names open an action menu; Ctrl/Cmd-click follows
directly ([validation](experiments/visual/INLINE_CALL_RESULTS.md)). Optional [type-aware navigation](docs/visual/SEMANTIC_PRODUCT.md)
uses rust-analyzer definitions for field/variable calls while keeping captured-source checks.
It can also follow calls into [local dependency source](docs/visual/DEPENDENCIES_PRODUCT.md),
with package labels and independent captured versions; downloads/builds remain disabled.
Smart [source reflow](docs/explanation/source-reflow.md) fits code to each pane while
preserving original line numbers, copying and call controls; Original lines is available
in Settings → Code layout ([validation](experiments/visual/REFLOW_RESULTS.md)).
Working-tree fragments follow disk saves while retaining their layout. **Pin version**
holds an earlier view; **Follow live** resumes updates. Missing declarations and historical
arrows retain explicit earlier context ([validation](experiments/visual/LIVE_RESULTS.md)).
Module labels and shared header/minimap accents distinguish native Rust/Python scopes.
Settings → Module outlines adds optional Bubble Sets contours without moving bubbles
([scope](docs/visual/MODULES_PRODUCT.md)).
Rust/Python are selected by default; explicit scopes enable
raw C/C++ analysis. Normal editing stays in the editor, with richer plugin integration
remaining a supported direction.

The question-search browser trial is retired; its [evaluation records](experiments/companion/QUESTION_SEARCH_RESULTS.md)
remain available. [Structural exploration](docs/how-to/structural-exploration.md) remains a
supporting prototype. The [Tract](experiments/companion/TRACT_PLUGIN_PACK_FRESH_RESULTS.md)
and [Sketchlayer](experiments/structure/SKETCHLAYER_RESULTS.md) results document relevance,
retention and IDE-duplication limits.

The production renderer, compiler-backed language integrations and storage engine
remain open decisions. Both guided learning and local-change review tasks now pass
on the frozen reference repository. [Focused review measurements](experiments/review-workflow/SCALE_RESULTS.md)
are recorded; independent usability and whole-repository qualification remain open.
The Canvas2D overview is a reference view for
validating coordination; it does not qualify large-codebase GPU performance.

The current [product checkpoint](docs/plans/product-milestones.md) evaluates whether a
visual call trail preserves useful context alongside an editor. The
[Minitosis replay](experiments/visual/RESULTS.md) checks functionality; direct maintainer
feedback on the spatial interaction remains open.

The [CPU repository-question experiment](experiments/qa/MODEL_COMPARISON_RESULTS.md) supplies a
versioned question library and compares small local models with an external index.
Smaller evidence and host-resolved references cut fixed-model latency, but none of the
three exact maintainer workflow probes is completed correctly, even when complete source
is supplied. A 4B candidate improves guided lookups but misses the 4 GiB target and costs
more latency; earlier receiver-method grading is corrected in an explicit source-audit
erratum. This remains an isolated experiment, with no chat interface.
The [physical batch follow-up](experiments/qa/PREFILL_RESULTS.md) still exceeds 4 GiB;
smaller physical batches save little memory and increase latency. A separate
[weight-space investigation](docs/explanation/model-weight-optimisation.md) considers
targeted behavioural edits. A [paired test of the published 0.8B patch](experiments/qa/WEIGHT_EDIT_RESULTS.md)
finds no complete-task improvement under the current CPU Q4 non-thinking conditions.
The [evidence-use diagnostic](experiments/qa/EVIDENCE_USE_RESULTS.md) identifies repeatable
errors on small source contrasts; real-code evaluation questions remain ungenerated.
The [output-contract comparisons](experiments/qa/JSON_OBJECT_RESULTS.md) favour the full
schema under the current prompts; relaxed outputs are less usable. Schema remains the default.
[Reproduce the model comparison](experiments/qa/MODEL_COMPARISON.md) or inspect the
[compact comparison](experiments/qa/COMPACT_RESULTS.md) and [original pilot](experiments/qa/RESULTS.md).

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
