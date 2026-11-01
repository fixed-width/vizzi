# First codebase workspace behaviour

The first slice lets a developer explore Rust source through a repository overview,
a source inspector, and a declaration outline sharing one selection. It uses the local
engine and works without a project build or external account.

## Required behaviour

- The root directory and cache directory are chosen when launching the native host.
- Rust files are discovered without following symlinks or reading root `.git`,
  `target`, or `node_modules` directories. LPC analysis remains out of scope.
- The overview groups files by directory, supports filtering, pan, zoom, and selection,
  and provides readable file labels and hover details.
  Its initial camera reveals readable labels; fitting the whole map can hide labels
  until the user zooms in again.
- Selecting a file updates its source and declaration outline together. Selecting a
  declaration reveals its lines. Source inspection is read only.
- Each displayed source and outline belongs to the displayed immutable snapshot.
  Requests and responses from older browser selections cannot replace newer selections.
- File selection survives content edits. Unique lexical declaration identities survive
  position changes; ambiguous duplicate identities fall back to file selection.
  Background publication preserves the reader's scroll position in the same file.
  Explicitly selecting a declaration, including the current one, reveals its lines.
- A source edit triggers analysis and publishes a complete replacement snapshot.
  Queries continue to use the previous snapshot while work is in progress.
- Versioned change notifications replace browser polling. Reconnection starts with
  current state, including after a host restart; slow clients need no event replay.
  A response for a superseded snapshot cannot change the selection or displayed source.
- Unchanged contents reuse analysis. A fresh process can reuse persisted analysis.
  Cache corruption or an unavailable cache must not produce incorrect source views.
- File-event hints avoid reading unhinted documents. Hinted bytes are verified even
  when size and timestamps match. Full reconciliation recovers missed changes and
  remains scheduled during continuous edits. Obsolete/failed work cannot lose hints.
- Syntax recovery and unavailable analysis are visible. The outline does not claim
  compiler-level name resolution, call relationships, or build-configuration coverage.
- Empty workspaces, removed selections, analysis failures, and connection failures have
  clear states. Source text and file names are displayed as text, not interpreted HTML.
  A recovered connection clears its error, and a failed selection request is retried
  even when the published snapshot has not changed.

## Verification

Tests cover cache reuse and invalidation, pinned source identity, stale publication,
selection reconciliation, bounded queries, syntax extraction, and corrupt cache recovery.
An end-to-end diagnostic run exercises cold load, warm load, and one-file updates on
the pinned lpc-rs snapshot. Rendering and interaction budgets require actual browser
measurements; software rendering does not qualify the GPU target.
