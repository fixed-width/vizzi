# Review working changes or two Git commits

Start Vizzi [on the Git repository](run-workspace.md), then choose **Review changes**.
To review before live exploration starts, open the printed server address with `/review`
appended. The review captures only the repository path you select.

## Compare working changes

Choose **Working tree**, enter a **Base revision** and a **Repository path**, then select
**Compare with working tree**. Use `.` or blank for the repository root. The path is a
literal file/directory under that root, not a revision.

The working side includes staged, unstaged and non-ignored untracked source observed on
disk. Unsaved editor buffers are not included. Check the resolved baseline and the
working observation interval above the panes.

## Compare existing commits

Choose **Two commits**, enter the base and **Target revision**, then select **Compare
commits**. Branches, tags, `HEAD~1`, `HEAD^^` and unique short hashes use Git's local
revision resolution. Both sides resolve once; the right pane is labelled **Target**.
Working disk edits are not part of that comparison.

The requested objects must exist locally. A shallow checkout can lack parent commits;
make that history available outside Vizzi or choose an available revision. Capture never
fetches. Invalid revisions or paths show an error while retaining the previous comparison.

## Read changes and surrounding code

Choose a file and a hunk to reveal the corresponding source ranges on each side. The
panes scroll independently. Select declarations for lexical context. Use **Include
unchanged context** and the path filter to inspect related source in the captured scope.

Back/Forward, including browser history, returns to accepted locations and reading
positions in the same comparison. Drag dividers to resize the file list, paired panes
or declaration lists. Focus a divider and use Left/Right, Shift for a larger step,
or Home/End for its limits.

Use the file/hunk page controls when more results remain. Only loaded change ranges
receive addition/removal backgrounds. Syntax colours are independent of those backgrounds.
**Exact change ranges** shows raw line/byte ranges when whitespace or final-newline edits
look alike. Long lines have First/Earlier/Later/Last fragment controls.

Missing, empty, unsupported-encoding and unobserved source have different availability
labels. An unavailable hunk is not evidence of equality.

## Refresh or replace a comparison

**Refresh working** captures disk changes against the pinned baseline. It retains the
selected file, side, filter and unchanged-context setting. Unchanged source keeps its
reading position; surviving declarations in changed source open at their current location.
Other changed source returns to file start with a notice. Hunk selection clears.

Two-commit comparisons have no working refresh. Submit another pair to change either
revision. **Close comparison** retires the active capture. A history visit to an expired
comparison offers **Open current comparison** rather than silently reusing another pair.

Use **Retry inspection** for temporary query refusal and **Reconnect** after a lost review
stream. Failed capture retains the previous coherent view. Server restart does not persist
a review capture; reopen the desired revisions. See [storage](../reference/storage.md).

**Settings → Theme** is shared with the other views. Dismiss transient error/notice
banners with their × controls; this does not close the comparison.
