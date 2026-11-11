# Working sets for call exploration

For readers who follow calls, investigate uses and return to interrupted tasks.
Editing, refactoring and debugging are outside Vizzi's scope. The editor handoff remains
available for source work.

Each source bubble offers Find callers and Find references. Results open in independent
canvas stacks, preserving earlier bubbles. Rows show the enclosing declaration, matched
source line and location. Hover/focus previews source. Opening a caller adds its source
bubble and the supported caller-to-target connection. Other references can open source
without manufacturing a call. Language-supported results retain semantic evidence.
Name-only candidates are labelled explicitly. Counts expose omissions and unavailable
analysis. Source changes make earlier results stale until refreshed.

Named task groups collect chosen source bubbles and result stacks. The group editor names
and selects members. Each item belongs to at most one task group. Move, collapse, expand,
ungroup and close operate on the collection. New callees inherit their caller's group when
they are first opened. Module cues continue to describe language membership independently.

The task shelf names, saves, opens, renames and deletes tasks. Each task retains source
bindings, result queries, groups, layout, camera and reading positions. Switching tasks
autosaves the current task first. Storage errors retain the working layout and show a
dismissible error. Existing source-only browser layouts migrate on first use. Source
capture expiry stays visible during resume. Referenced files are never guessed by name.

The first implementation uses explicit group membership and a browser-local task shelf.
It admits 12 tasks, four result stacks per task and eight task groups. The source working
set retains its existing 24-bubble bound. Each result query is paged and bounded. Saved
tasks retain query bindings and re-run results when opened.

Validate through model interfaces, actual browser interactions, native protocol fixtures
and a real local Rust project. Check old layouts, stale results, same-name ambiguity,
dependency sources, group geometry and closure, task isolation, storage failures, reload
and late replies after task switches.
