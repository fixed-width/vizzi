# Compare adjacent function versions

Use this when stepping through a function's history leaves you unsure which lines changed.
Open a repository function in the call workspace, either at saved current source or at a
[historical version](read-function-history.md).

1. Choose **Compare**. The existing bubble becomes **After** and a new **Before** bubble
   opens beside it. Existing bubbles keep their positions. Both comparison views are
   pinned; a saved working-tree side is labelled as captured source.
2. Read the highlighted lines: Before marks removed/replaced source and After marks added
   or replacement source. Line numbers refer to each original file. Copying preserves
   source text and whitespace without adding diff signs.
3. Use **Previous edit** or **Next edit** in either bubble to reveal corresponding changes
   in both visible sides. The first edit opens automatically, even when it is beyond the
   initially loaded section. Scroll inside either bubble to read the surrounding code.
4. Close either bubble independently. Remaining highlights still describe the original
   frozen pair. Choose **Return to live** to resume saved-source tracking in that bubble.
   **History**, when available on a committed side, resumes reading older function versions
   and removes its comparison highlights.

Comparison uses complete captured declarations. Moving a function within its file or
editing other functions does not create extra highlighted changes. The comparison follows
the existing conservative declaration matching and first-parent history boundaries.

Saved edits, later commits and updates in other live bubbles do not change a comparison's
inputs. [Calls within each captured file](follow-captured-calls.md) open that side's exact
declarations. Callers, references, Save and Editor remain current-source actions. Existing
call arrows retain historical styling; comparing two bubbles does not create a call relationship.

Task saves retain comparison bindings and reading positions. Native captures remain
temporary: restart or quota eviction can make further paging unavailable. Loaded source
remains readable, and Return to live is still available. See [storage limits](../reference/storage.md).
