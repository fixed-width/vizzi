# Project design and scope

The project supports learning unfamiliar codebases and assisting code reviews through
coordinated views of the same source material. These notes capture the current design
direction and scope; they are not an implementation status report or a delivery plan.
Contributor standards belong in [AGENTS.md](AGENTS.md).

## Architecture

Keep source access, language analysis, indexing, investigations, views, and host
integration behind small interfaces. Keep platform facilities outside portable logic.

Source snapshots and their comparison are independent of source-control history.
Target Git initially without requiring Git-specific identifiers or operations in the
shared model. Preserve an integration path for other source-control systems.

Support mixed-language workspaces and differing analysis capabilities. Preserve
source locations, provenance, configuration, and analysis coverage. Language-specific
findings should retain their meaning when presented through shared views.

Source selection policy and language analysis have distinct responsibilities. Preserve
raw syntax for build-free exploration and keep configured compilation results tied to
their actual environments. The [C/C++ policy](docs/prototype/C_FAMILY_POLICY.md) defines
this extension; its [rationale](docs/explanation/c-family-analysis.md) records the evidence.

Initial delivery uses a local Rust engine and browser interface, with offline
operation. Preserve an integration path for editor plugins through host interfaces.
Editor integration is a future capability.

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
