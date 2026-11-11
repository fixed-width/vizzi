# Follow saved changes or pin a source view

The call workspace follows files saved to disk. Keep Vizzi running while you edit in
your editor; unsaved editor buffers are not visible to it.

## Follow an edit

Save the file in your editor and wait for the bubble to update. Its arrangement stays
in place. Vizzi tries to retain a nearby unchanged reading line; if that context cannot
be recovered it returns to the function start and reports the reset.

If a function is removed, renamed, moved or ambiguous, its earlier view remains visible
with a notice. Use **Add bubble** to choose its current declaration. Vizzi does not
silently replace it with a similarly named function.

An earlier call arrow can become historical, unverified or removed. Hover it for the
current status. Earlier source is not evidence that the call is still present.

## Hold a version while reading

Choose **Pin version** in a bubble to retain that source while other views follow edits.
Choose **Follow live** to resume updates. Pinning does not edit the file or create a
saved source location.

Old pinned views disable actions that require current source. Their loaded text can
remain readable, but later paging or reloading can report that the capture has expired.
Pinning is not permanent source storage; see [data lifetimes](../reference/storage.md).

Dependency bubbles hold independent captured versions. Their directories are not watched
in the initial implementation. Reopening a definition can capture newer dependency source;
it does not overwrite an earlier bubble's bytes silently.

## Refresh caller/reference results

When a stack says **Repository updated**, choose **Refresh**. A different file can add or
remove callers even if the displayed function did not change. Background checks that
retain the same repository observation do not invalidate results.

If a status banner says source updates are disconnected, keep the layout open while
the connection retries. If it does not recover, check that the server is still running
and reload using the same address. Dismissing a banner hides the message, not the
underlying operation. See [troubleshooting](troubleshoot.md) for persistent failures.
