# Project design and scope

The project supports learning unfamiliar codebases and assisting code reviews through
coordinated views of the same source material. These notes capture the current design
direction and scope; they are not an implementation status report or a delivery plan.
Contributor standards belong in [AGENTS.md](AGENTS.md).

## Product role

The visual call workspace is Vizzi's main mode and opens at the launch URL. The source
browser, investigation and review prototypes are available through their direct routes.

Vizzi is a visual workspace beside an editor for understanding unfamiliar source and
reviewing changes across files and revisions. The maintainer's closest reference is
Code Bubbles: readable source fragments and other views form a task working set that
stays visible while the reader follows relationships. Walking a call chain should open
the next fragment beside its caller and retain earlier steps and branches in a stable
spatial arrangement. The [visual working-set rationale](docs/explanation/visual-working-sets.md)
records the literature and the intended interaction; [CONTEXT.md](CONTEXT.md) defines terms.

Multiple coordinated views are central: source fragments, focused relationships/overviews,
notes and revision/diff views share source identities and context. Layout/grouping,
collapse/expand and return to earlier reading positions support understanding; large
repositories must remain manageable through a focused task working set. Editing,
refactoring and debugging are outside Vizzi’s scope. Editor plugins remain an integration
path for source navigation and live context.

Search, directory browsing and declaration lookup help introduce a view into that working
set. They are supporting entry points rather than the organising product workflow. The
recent question-ranking and structural-list experiments exposed IDE duplication and do
not qualify the desired visual interaction. Keep their evidence and reusable interfaces;
remove question-search entry points and question-authoring controls from the browser.
Retain legacy saved metadata and offline evaluation interfaces. Develop additional views
around tasks that benefit from the spatial call trail. Caller/reference stacks, named task
groups and a task shelf support this workflow. See [working sets](docs/visual/WORKING_SETS_PRODUCT.md).

Optional model assistance may help choose or organise source views later. It is not a
prerequisite for the spatial workspace and is not the active experiment. Source identity,
analysis coverage, candidate relationships and reader-authored associations retain their
meaning in every view; placement or an arrow must not manufacture semantic certainty.


## Architecture

Keep source access, language analysis, indexing, investigations, views, and host
integration behind small interfaces. Keep platform facilities outside portable logic.

Visual-workspace state is independent of source/relationship facts. Persist the reader's
views, layout, groups and reading/camera state separately from analysis identities. A
fragment view references a particular source observation/version/range; it is not limited
to the short excerpt stored by the existing investigation model. This seam also permits
several representations of the same source and per-view revision context for reviews.
Working-tree views follow saved changes through explicit declaration tracking and coherent
source/relationship refreshes. Individual views can pin a captured version. Ambiguous or
missing declarations retain earlier context rather than adopting a guessed target; see
the [live view contract](docs/visual/LIVE_PRODUCT.md).

Source snapshots and their comparison are independent of source-control history.
Target Git initially without requiring Git-specific identifiers or operations in the
shared model. Preserve an integration path for other source-control systems.

Support mixed-language workspaces and differing analysis capabilities. Preserve
source locations, provenance, configuration, and analysis coverage. Language-specific
findings should retain their meaning when presented through shared views.
Local dependency source uses independent, bounded captures admitted by language providers.
Discovery stays language-specific; repository snapshots and watchers retain their scope.
See [dependency source](docs/visual/DEPENDENCIES_TECH.md).
Native module/package identity uses captured project configuration and language declarations.
Its labels, accents and optional contours are independent of reader-authored groups and
call relationships; unknown membership stays neutral. See [module identity](docs/visual/MODULES_TECH.md).

Source selection policy and language analysis have distinct responsibilities. Preserve
raw syntax for build-free exploration and keep configured compilation results tied to
their actual environments. The [C/C++ policy](docs/prototype/C_FAMILY_POLICY.md) defines
this extension; its [rationale](docs/explanation/c-family-analysis.md) records the evidence.

Initial delivery uses a local Rust engine and browser interface, with offline
operation. Use editor-neutral source-location interfaces. The first companion slice brings a minimal
VS Code handoff forward; richer plugin integration remains a later capability.

Rust with wgpu is the leading rendering candidate. Representative performance
experiments should establish the rendering backend before it is treated as settled.

## Initial scope

Begin with read-only static exploration, disk changes, and comparisons of local
changes or revisions. Basic exploration should work without a successful project build.
Within the product, invoking repository-dependent build tooling requires an explicit
user action; this is a product behavior requirement, not an approval rule for building
or testing this project's own implementation.

Use lpc-rs, the Linux kernel, and PyTorch as reference repositories with staged analysis
depth. Use lpc-rs for Rust analysis and complete learning and review workflows. Use the
kernel for scale and configuration testing, and PyTorch for mixed-language testing.

LPC analysis is out of scope for now. Explicitly exclude LPC source in lpc-rs from
C/C++ analysis rather than classifying it by extension.

## Incremental indexing

Reuse unchanged inputs, track analysis dependencies, recover from missed filesystem
events, cancel superseded work, and publish consistent results. File metadata alone
is not proof of unchanged contents. Analysis validity depends on source contents,
configuration, dependencies, and analyzer version.

Prioritize correctness checks for invalidation, cache reuse, source identity, revision
comparison, cancellation, and consistent publication. Compare incremental results
with clean rebuilds, alongside independently specified expected results.

## Performance validation

The baseline is a developer laptop with an integrated GPU and 16 GB RAM. Measure
initial indexing, warm opening, change detection, reindexing, memory, and interaction
separately. Rendering experiments should include representative labels, relationships,
updates, and multiple views.

Use small deterministic fixtures for focused correctness checks and reference
repositories for broader integration and performance measurements.

## Open implementation choices

The rendering backend, storage engine, supported compiler version, and concrete host
protocol remain to be selected. Record their rationale here as the design develops;
keep task sequencing and completion evidence in a separate implementation plan when
one is needed.

The first end-to-end slice is specified in [the workspace behaviour contract](docs/prototype/PRODUCT.md)
and [the technical contract](docs/prototype/TECH.md).

The [product milestone plan](docs/plans/product-milestones.md) sets the next delivery
order. Its [research rationale](docs/explanation/task-driven-views.md) connects proposed
views to concrete learning/review tasks and distinguishes hypotheses from evidence.
