# Live working-tree fragments

For readers exploring source while editing in their usual editor. Working-tree bubbles
follow saved files by default; immutable captures remain the source of every displayed
version. Existing explicit revision-review interfaces retain their pinned semantics.

- On publication, refresh the working set against one observation. Preserve view IDs,
  geometry, collapse state, camera and selection. Update source and call metadata together;
  obsolete replies cannot replace newer intent. Unchanged source keeps its DOM/selection.
- Track unambiguous declarations through body edits and movement within a file. Do not
  infer a rename, cross-file move, overload or ambiguous duplicate. Retain earlier source
  with an explicit unresolved state when tracking fails; retry on later publications.
- Preserve reading anchors by matching an unchanged captured line near its expected new
  location. If mapping is unavailable or ambiguous, return to the declaration start and
  report the reset. Recompute the displayed function margin for a new source version.
- Retain the visual call trail. Revalidate unique written sites and source-local/candidate
  evidence; show removed or unverified relationships as historical. Old language-server
  evidence never silently becomes current evidence. Following again can establish a fresh
  relationship. Pinning either end makes a cross-version relation historical.
- **Pin version** holds one bubble's captured version; **Follow live** resumes tracking.
  Pinning is independent of **Keep**, which captures saved evidence. Loaded pinned bytes
  remain visible, while further paging after native capture expiry is explicitly unavailable;
  this slice does not create a durable source archive.
- Old actions are disabled while a refresh is pending or their view is retained from an
  older observation. Current views can navigate, keep evidence and request verified editor
  handoff. Historical/pinned-old views cannot request current-source actions.
- Active source selection defers applying a working-set refresh. New source supersedes
  pending work. Source failures leave prior coherent views available and allow retry.
- Follow saves visible to the filesystem watcher, with reconciliation as existing fallback.
  Unsaved editor buffers require a later editor adapter. This feature is language-neutral,
  build-free and independent of Git identifiers.

Validate through native identity/link/anchor interfaces, the presentation model and real
browser fixtures. Include mixed languages, insertion/deletion, missing/ambiguous functions,
selection deferral, supersession, pinned/live coexistence, reload and independent Keep.
Use recording-only editor adapters, low-concurrency mutation checks, and separate optimised
measurements of representative active working sets.
