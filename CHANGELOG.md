# Changelog

Notable user-visible changes to Vizzi. The initial entries summarize the current
development prototype as of 2026-11-11; earlier implementation and experiment history
remains in Git and the project documentation.

## Unreleased

### Added

- Search file paths and declaration names from **Find source**. The repository map
  narrows to matching files, and the Matches list opens functions, modules and other
  declarations at their source lines.
- Reopen a saved location at its line in the Source browser or its bubble in the
  Call workspace, based on where it was saved.
- Report the installed build with `vizzi --version`.
- Open highlighted function definitions and written calls in the Call workspace from
  the Source browser.
- Browse every captured file in large repositories through directory cards and pages.
  Open captured READMEs and project manifests from the map's landmarks.
- Open Ruby gem specifications and files with recognized execution entry points from
  the Source browser's landmarks.
- Color file tiles by language or type in the Source browser repository map
  and overview navigator.
- Select multiple file types in the Source browser map legend to show files matching
  any selected type. Use **Clear filters** beside the legend to reset type and path filters.
- Browse TOML and Markdown files with syntax coloring, including project manifests and READMEs.
- Browse JSON files, including test fixtures, with syntax coloring in the Source browser.
- Browse Java files with syntax colors, outlines, and written-call candidates. Use a
  shared VS Code Java provider to resolve definitions and references in workspace source.
- Run Eclipse JDT LS directly from Vizzi with `--jdtls` for Java definitions and references
  across configured source roots.
- Open JDK and local Java library source from attached archives in dependency bubbles.
- Import Maven and Gradle Java classpaths from the Workspace menu without listing individual JARs.
- Reuse cached Gradle Wrapper distributions and resume Java archive navigation after JDT LS restarts.
- Show Java package and named module cues in live, historical, and comparison bubbles.
- Follow Java dependency and JDK source through VS Code sharing and reopen archive bubbles in VS Code.
- Open Vizzi from Neovim and jump back to verified saved source in the launching split.
- Find and confirm an earlier declaration across a simple rename or file move in Git
  function history, with exact before and after source evidence.
- Follow named calls across repository files at the caller's Git revision. The same
  file chooser and name-candidate flow work across the supported written-call languages.
- Choose an exact local Git commit, branch or tag for a function bubble, including
  commits where the function was unchanged.
- Browse JavaScript, JSX, TypeScript, and TSX with syntax colors, declaration
  outlines, and written-call candidates. Capture skips nested `node_modules` trees.
- Use VS Code's shared JavaScript and TypeScript service for project-aware definitions
  and references. Bubbles show captured package names, including in function history.
- Enable standalone JavaScript and TypeScript definitions and references with
  `--typescript-server` and the native language server in TypeScript 7 or newer.
- Follow definitions into installed JavaScript and TypeScript packages with managed
  TypeScript or VS Code sharing. Dependency bubbles retain package names and versions.
- JavaScript and TypeScript outlines show variable-bound object methods, anonymous
  argument callbacks, and named TypeScript signatures.

### Changed

- Open Saved locations directly and use the Source browser to find new locations.
- Resize the Saved locations inspector on wider screens and close it with the standard X.
- Open Settings from the cog in the shared header.
- Choose **Add notes** on a saved location to open its inspector and focus the **Notes** field.
- Open Vizzi in the Source browser by default. The header lists Source browser, Call
  workspace, Review changes, and Saved locations in that order.

### Fixed

- Keep Source browser error banners visible above both pane separators.
- Keep the repository map cursor steady during a press to avoid delayed file clicks in Brave.
- Open files from the repository map without fetching the same map again.
- Resolve language-server locations in files with LF, CRLF, and bare CR line endings.
- Use the callee name's line in the Calls list and selection highlight for multiline calls.
- Refresh installed JavaScript and TypeScript resolutions after package manifest edits.
  Restore package references after restart and preserve VS Code package aliases.
- Managed TypeScript preserves alias precedence, scopes config errors to affected
  projects, and handles BOM-marked configs and lookups in large source files.

- Structural capabilities report written-call support from the host's registered analyzers.
- Follow VS Code definitions to quoted JavaScript and TypeScript methods. Past package
  labels in nested projects use the nearest manifest from the selected commit.
- Bound long computed method labels and lexical keys during JavaScript and TypeScript indexing.

## 0.2.0 - 2026-11-19

### Changed

- Caller results distinguish opening live source from showing an existing comparison
  at its retained revisions and reading position.

- Function comparisons identify the jump to the first changed block and label the
  controls for moving between blocks or exiting the diff to read an older revision.

- Scroll through long functions inside a bubble. Adjacent source loads as you read,
  with line numbers, call actions and selection across loaded sections.

- Editor navigation waits for confirmed browser application. Newer editor requests or
  manual navigation cancel waiting actions silently, with source rechecked before UI
  application. Unconfirmed results report an error without automatic replay.

- Stop Workspace cancels older pending commands; fresh actions wait for cleanup and
  start normally. The latest selected project wins during concurrent startup, while
  Stop Sharing leaves native startup and return navigation available.
  Same-project concurrent opens wait for shared startup and preserve its original failure.

- Plugin-launched Editor actions return to the exact originating VS Code window,
  including from external browsers. Dirty or changed target files are refused without
  saving buffers; unavailable origins report an error without opening another window.

- Editor call and source-browser actions share native captured-source handoff logic.
  Both wait for matching saved source for up to 30 seconds; newer requests in the same
  editor window silently replace waiting requests without stopping shared capture.
- Successful editor handoffs are silent in the call workspace and saved locations;
  failures still show an error.
- VS Code setup starts from the status bar or **Vizzi: Open Workspace**, with optional
  project-local `vizzi.toml` settings and no manual connection-file setup.
- Plugin activation opens Vizzi in a reusable editor tab. **Vizzi: Open Workspace in
  Browser** remains available for an external browser.

### Added

- Follow calls from historical and comparison bubbles within their exact captured file,
  with revision-labelled destinations, explicit name choices and retained call trails.

- Compare adjacent function versions in pinned Before/After bubbles, with full-declaration
  change highlights, coordinated edit navigation and original source line numbers/copying.

- Function bubbles can step backward and forward through local Git changes while
  retaining their layout, then return to live source. Unchanged function versions are
  skipped; historical code is labelled and keeps current-source actions disabled.

- **Vizzi: Open in Source Browser** in VS Code's editor and file-tab context menus
  opens saved project files and reveals the editor's cursor line when available.
- VS Code Source Control and Command Palette actions compare two selected local Git
  revisions, or saved working changes with HEAD, in the embedded review workspace.
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

- Rust and Python function history and comparisons retain verified module labels and outlines,
  including when following calls within the captured file.

- **Add bubble** can search an exact source file even when repository-wide function
  search omits that file or its declarations.

- At the newest function-history revision, **Return to live** appears in the history
  controls. The bubble shows progress during a lookup and keeps the captured revision
  readable. Readers can step through history or retry if the live refresh fails.

- **Add bubble** finds Rust methods by qualified names such as `Task::run_entry`
  with the impl's generic arguments omitted. Results retain their full names.

- Superseded function comparisons and edit navigation stay silent after newer editor
  navigation, Return to live or closing a bubble; delayed responses cannot replace newer
  source, change focus or display obsolete errors.

- Historical bubbles recover from temporary busy source reads when restoring later
  sections, keeping their captured revision and reading position.

- Newer editor requests survive an ongoing workspace-mode switch. Manual navigation
  cancels routed requests before they resume, and Review waits for comparison status
  to catch up before rejecting a requested capture.

- Editor handoffs revalidate saved source after the Vizzi tab becomes ready, preventing
  navigation after edits made while the tab loads.
- Superseded source actions no longer display startup errors. Queued newer actions
  can retry startup; current failures remain visible.
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
