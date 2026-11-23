# Follow calls within captured source

Use this to inspect an older helper implementation while keeping your historical caller
visible. Start from a [function-history bubble](read-function-history.md) or either side
of a [function comparison](compare-function-versions.md).

1. Click a highlighted call and choose **Open in captured file**, or Ctrl-click/Cmd-click
   to follow directly. **Calls** provides the same written invocations as a list, including
   calls outside the currently loaded section.
2. A supported source-local destination, or a sole complete same-file name candidate,
   opens beside the caller. Multiple matches use the normal chooser. Check their qualified
   names: a name match does not establish a receiver type or runtime dispatch. Partial or
   limited name analysis requires an explicit choice rather than assuming completeness.
3. Read the pinned destination's captured revision. Follow more calls from that bubble to
   stay at that revision. Revisiting an open function or following a cycle
   reuses its view in that capture, preserving the earlier reading position.
4. Page long functions normally. Indentation, line numbers, source styling and copying
   still describe the original captured file.
5. Close a child independently or choose **Return to live**. The current declaration is
   resolved through normal saved-source tracking; missing/ambiguous declarations retain
   the actual old capture instead of selecting a guessed replacement.

## Look in another file at the same commit

1. On a named call in a Git history bubble or a committed comparison side, choose
   **Look in another file…** from the inline menu or next to the call in **Calls**.
2. Browse files at the displayed commit or enter a repository-relative file path and
   choose **Find candidates**.
3. Select a declaration to open it alongside the caller. A cross-file name candidate
   always asks for your choice, even when there is only one. Paths and code come from
   the displayed commit. Open a live bubble separately for current project-aware navigation.

The file chooser uses the configured source policy and the same name-candidate flow for
all written-call languages. Saved working-tree comparison sides support their own
captured file. Callers, references, Editor and Save remain current-source actions.

Captured arrows record the actual call witness, revision and candidate/source-local
evidence. Unrelated saved edits do not change them. If an endpoint changes revision or
returns live, the old link remains historical; following again can establish fresh evidence.

The child shares its parent's native capture lifetime. See [expiry and storage limits](../reference/storage.md).
Loaded text can remain readable after expiry, but further reads may fail.
A child does not inherit a function-history timeline that tracks its caller: its Return
to live control and a newly opened current declaration provide the next navigation entry.
