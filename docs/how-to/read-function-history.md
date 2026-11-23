# Read a function's Git history

Use this when an earlier implementation may explain code you are exploring. Open Vizzi
at the root of a local Git worktree with available commits, then add the function to the
call workspace.

1. Choose **Previous change** in the bubble. Vizzi freezes local HEAD and finds an earlier
   version with different declaration text. Changes elsewhere in the file are skipped.
   A saved, uncommitted function can first step back to its HEAD version.
2. Continue with **Previous change**, or use **Next change** to retrace accepted versions.
   The bubble shows the selected commit's short ID. Hover over it for the full commit
   and the frozen history tip. At the newest historical version, **Return to live**
   takes the place of **Next change** in the history row.
3. Scroll through a long function inside the bubble, or use **Previous section** and
   **Next section** to jump. Other bubbles keep their positions and can follow saved edits.
4. Choose **Return to live** to resolve the original function against current saved files.
   Its bubble keeps its position, size and task membership. Choose **Compare** once live
   source is ready to review it alongside the previous version.

## Open a known commit, branch or tag

1. In a repository function bubble, choose **Choose revision…**. In a comparison bubble,
   open the bubble's **Actions** menu to find the command.
2. Enter a local commit ID, branch or tag and press Enter or choose **Open revision**.
   Use a full ref such as `refs/heads/review` to distinguish a branch from a tag with
   the same name. The bubble shows the commit even if the declaration was unchanged.
3. Use **Previous change** to read an older version or **Compare** to see it beside the
   selected commit. **Next change** returns through the visited versions to the exact commit.
   To pick a different branch or commit, choose **Choose revision…** again.

The commit stays pinned if the branch or tag moves. Press Escape or choose **Cancel**
to keep the current bubble and its captured calls readable while a selection is pending.
If Vizzi reports that the commit or declaration is unavailable, you can correct the input
and retry.

If that function has been renamed or deleted in current source, Vizzi keeps the actual
historical version with an unavailable message. You can continue browsing its history
or retry **Return to live** while Vizzi waits for a successful refresh. If the original
declaration reappears while live tracking is active, its bubble updates automatically.
This retained state also survives reload while the history session remains available.

Historical bubbles support reading, copying and [calls within the captured file](follow-captured-calls.md).
Callers, references, Save and Editor require current source. Existing connections remain
visible with historical styling; newly followed captured calls retain their own revision
and evidence. Returning live does not silently turn captured links into current facts.

History follows the first parent of merges from the frozen local HEAD or chosen commit. It considers
up to 64 file revisions and does not switch branches, check out files, fetch objects or
build the inspected project. New commits do not change an already-open timeline; return
live and open history again to start from a newer HEAD.

Ordinary matching stops when the declaration is missing or ambiguous. Boundary messages
distinguish a matching failure, shallow history and the 64-revision bound from the
beginning of complete available history.

## Continue across a rename or file move

1. Open the commit at the boundary with **Previous change** or **Choose revision…**.
2. Open the bubble's Actions menu and choose **Find earlier declaration…**.
3. Check the names, paths and commit IDs. Choose **Use earlier declaration** to add the
   match to the timeline. Use **Next change** to return across the accepted transition.

The match requires the same enclosing scope and exact declaration text apart from the
name. If the candidate is missing or ambiguous, the bubble keeps the displayed capture.

Captured history is temporary. A reload can restore it while its native session remains
available; server restart or session eviction can make it unavailable. **Return to live**
still works. See [storage limits](../reference/storage.md) for the file and session bounds.
