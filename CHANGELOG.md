# Changelog

Notable user-visible changes to Vizzi. The initial entries summarize the current
development prototype as of 2026-11-11; earlier implementation and experiment history
remains in Git and the project documentation.

## Unreleased

### Added

- A [visual call workspace](docs/how-to/follow-a-call-trail.md) with source bubbles,
  retained caller context, branches and direct navigation from highlighted call names.
  Function lookup accepts qualified names and ranks exact matches first.
- Caller and reference result stacks with source previews. Configured rust-analyzer
  supplies semantic Rust results; other languages expose labelled written-call name
  candidates.
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

- Retired the question-driven browser workflow. Offline evaluation tooling and experiment
  records remain available.
