# Vizzi

Coordinated views for exploring source code and, eventually, reviewing changes.
The current prototype links a Rust/Python file map, captured source, and a lexical declaration
outline. A native Rust engine supplies immutable observations to a local browser host.

The production renderer, compiler-backed language integrations, review workflow, and
storage engine remain open decisions. The Canvas2D overview is a reference view for
validating coordination; it does not qualify large-codebase GPU performance.

- [Run the workspace](docs/how-to/run-workspace.md)
- [Architecture and scope](DESIGN.md)
- [First-slice behaviour](docs/prototype/PRODUCT.md) and [technical contract](docs/prototype/TECH.md)
- [Host API reference](docs/prototype/API.md)
- [Change-notification contract](docs/prototype/CHANGES.md)
- [Incremental-capture contract](docs/prototype/INCREMENTAL_CAPTURE.md)
- [Captured document and snapshot identities](docs/prototype/IDENTITIES.md)
- [Language adapters and shared worker budget](docs/prototype/LANGUAGES.md)
- [C/C++ policy](docs/prototype/C_FAMILY_POLICY.md), [rationale](docs/explanation/c-family-analysis.md), and [delivery plan](docs/plans/c-family.md)
- [Application measurement procedure](experiments/prototype/README.md)
- [Browser validation and measurements](experiments/browser/README.md)
- [Verified incremental-capture measurements](experiments/incremental/RESULTS.md)
- [Mixed-language adapter measurements](experiments/languages/RESULTS.md)
- [Earlier performance experiments](experiments/performance/README.md)
- [Contributor guidance](AGENTS.md)

Implementation uses Rust 2024 with unsafe code denied. The browser adapter uses plain
JavaScript, Canvas2D, and DOM controls, with no external assets or runtime dependencies.
