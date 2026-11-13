# Changelog

Notable user-visible changes to Vizzi. The initial entries summarize the current
development prototype as of 2026-11-11; earlier implementation and experiment history
remains in Git and the project documentation.

## Unreleased

### Changed

- Successful editor handoffs are silent in the call workspace and saved locations;
  failures still show an error.
- VS Code setup starts from the status bar or **Vizzi: Open Workspace**, with optional
  project-local `vizzi.toml` settings and no manual connection-file setup.
- Plugin activation opens Vizzi in a reusable editor tab. **Vizzi: Open Workspace in
  Browser** remains available for an external browser.

### Added

- **Vizzi: Explore Call** in the VS Code editor context menu opens the saved call's
  caller and definition in the current visual task, with ordinary ambiguity handling.
- Shared VS Code analysis can open local Rust dependency and standard-library functions,
  including `ok_or_else`. Checked editor function ranges support newer Rust syntax without
  rewriting captured source or launching another language server.
- Platform-specific VSIX packages include the native Vizzi executable. **Open Workspace**
  starts it and opens an editor tab; **Stop Workspace** closes the process it owns.
- TOML launch settings through `--config FILE`, with command-line overrides, explicit
  path rules and strict validation.
- A VS Code plugin and `--vscode-connection` option to share editor definition
  and reference providers without launching duplicate Vizzi-managed language servers.
  Shared analysis checks saved/captured source and reports unavailable analysis on
  dirty buffers or disconnect; managed-server fallback is never automatic.

### Fixed

- Automatic VS Code launches no longer print obsolete instructions to copy connection
  paths into TOML. Optional external-launch settings appear only in advanced sharing.
- VS Code sharing opens Output before folder selection, uses a sole open folder directly,
  and reports missing folders, cancellation and startup errors instead of returning silently.

## 0.1.1 - 2026-11-11

### Fixed

- Include the project's `LICENSE-APACHE` and other `LICENSE*` files in binary archives.
  Packaging requires a project license, and the publication gate checks each archive's
  license contents against the tagged source before publishing.

## 0.1.0 - 2026-11-11

### Added

- CI workflows for native/browser checks and immutable Linux/macOS release archives
  on x86-64 and ARM64, with checksums and locked GitHub Actions dependencies.

- A Ruby call-trail tutorial with an included greeting example, module cues and
  saved-task walkthrough.
- Ruby source browsing, colouring, reflow, module cues and call navigation, including
  bare calls and punctuation method names. Optional `--ruby-lsp` integration supplies
  captured-source definitions, checked references and local dependency source without
  installing project dependencies or loading project add-ons.
- A C/C++ call-trail tutorial with a mixed-language example. The Rust tutorial and its
  example now have explicit Rust names alongside the Python lesson.
- Language-aware LSP navigation for Python/Pyright and C/C++/clangd alongside Rust.
  Typed receivers, imports and references resolve before name fallback; local dependency
  source uses bounded captures. C/C++ written calls and header declarations are navigable.
- A [visual call workspace](docs/how-to/follow-a-call-trail.md) with source bubbles,
  retained caller context, branches and direct navigation from highlighted call names.
  Function lookup accepts qualified names and ranks exact matches first.
- Caller and reference result stacks with source previews. Configured rust-analyzer
  supplies semantic Rust results; configured Pyright/clangd supply Python/C-family results.
  Unconfigured analysis retains labelled written-call name candidates.
- Named task groups that move and collapse related bubbles together, plus a browser-local
  task shelf that retains layouts, zoom and reading positions between tasks.
- Syntax colouring, smart source reflow, content-fit controls and resizable bubbles.
  Hovering call lines, bubbles or connecting arrows highlights the related context.
- Live updates from saved files, with per-bubble Pin version and Follow live controls.
  Language module labels, shared accents and optional module outlines identify related
  bubbles independently of task groups.
- Rust navigation into locally available dependency source, with package provenance and
  independent captured versions. Dependency source is not downloaded automatically.
- Source browsing for Rust and Python, with raw C/C++ analysis available through explicit
  source scopes. [Git reviews](docs/how-to/review-local-changes.md) compare two commits or
  a baseline with staged, unstaged and non-ignored untracked working files.
- Saved source locations and an explicit VS Code handoff that works without first saving
  a location. Source exploration remains read-only.

### Changed

- Expanded `--help` with grouped option descriptions, defaults, requirements and
  launch examples for Python, Rust and C/C++.
- `--root` now defaults to the current directory. `--cache` defaults to the platform
  user cache with a separate directory for each canonical source root; both options
  remain available as overrides.
- Refreshed user documentation with Python and Rust tutorials, focused task guides,
  command/language/storage references and troubleshooting. The README is a first-run
  introduction; development history and experiment links have a separate contributor index.
- The call workspace opens at the launch URL. A shared header connects Call workspace,
  Review changes, Source browser and Saved locations, with consistent Settings and
  separate mode-specific controls. Theme selection follows the system by default and
  persists across modes.
- Reduced first-navigation delays by preparing Rust type analysis while source fragments
  are read. Proven unique destinations open directly; ambiguous candidates retain a
  chooser.
- Reference-result warnings say “Repository updated” because changes elsewhere in the
  repository can add or remove callers without changing the displayed function.

### Fixed

- Fixed a reload failure with module outlines enabled when a task contains reference
  stacks or collapsed groups alongside source bubbles.

### Removed

- Retired the question-driven browser workflow and archived its native experiment tool.
  Experiment records and historical source remain available.
