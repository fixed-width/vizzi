# Review working changes or two Git revisions

Start Vizzi with the repository as its configured root and a cache outside it:

```sh
cargo run --release -p vizzi -- \
  --root /path/to/repository --cache /tmp/vizzi-review-cache --workers 1
```

Append `/review` to the printed local URL to begin without starting live exploration.
Enter a **Base revision** and a literal **Repository path** (`.` or blank for the
repository root), then select **Compare with working tree**. The path selects a file
or directory; it does not accept a revision. Check the resolved baseline and
working observation interval above the panes. The working capture includes staged,
unstaged and non-ignored untracked source as observed on disk.

To compare existing commits, choose **Two commits**, enter the base and **Target
revision**, then select **Compare commits**. Both inputs resolve once, including
branch/tag names. The right pane is labelled **Target**; working disk changes are
excluded and working refresh is hidden. Comparing a new pair requires another submit.
An invalid revision or a non-root path absent from both sides shows an error beside
the form and keeps the previous comparison visible.

Choose a changed file, then a hunk to reveal its base and working ranges. The panes
scroll independently. Drag the dividers to resize the file list, paired panes or
declaration lists. Focus a divider and use Left/Right (Shift for a larger step), or
Home/End for its width limits. Width preferences survive reload when local storage
is available. Their declaration outlines provide lexical context on each
side; arrow keys, Home/End and Page Up/Down move through an outline, and Enter selects
a declaration. Each entry labels its kind and starting **Line N**. The **Base** and **Working** buttons select a side explicitly.

To inspect a related contract or test, select **Include unchanged context**, filter
its path and choose the file. The filter changes the list while retaining the same
captured scope. Back/Forward, including the browser controls, returns to accepted
locations and their reading positions within that exact comparison.

Use the page controls when more files, hunks or change ranges remain. Only loaded
change ranges receive addition/removal backgrounds. Lexical syntax colours are
independent of those backgrounds and can be unavailable when styling limits are reached. Expand **Exact change ranges** for raw line/byte
coordinates when whitespace, line-ending or final-newline edits look alike in the
text preview. Long source lines have First/Earlier/Later/Last fragment controls.

Read the availability labels when text is absent: a missing side, empty file,
unsupported encoding, literal symlink/reference, executable change and unobserved
path have different meanings. Unavailable textual hunks do not establish equality;
retained text can still be inspected where available.

Select **Refresh working** to observe disk changes against the pinned baseline.
Refresh keeps the selected file, side, filter and unchanged-context setting. Panes
whose source versions are unchanged keep their reading positions. A surviving selected
declaration in changed source opens at its current location; other changed source opens
at file start. Missing files/sides/declarations have a notice and explicit fallback.
Hunk selection clears; select a fresh hunk to reveal its ranges. Comparing a new
baseline resets context. Failed capture keeps the previous coherent view.
A history visit to a replaced or closed comparison
explains expiry; **Open current comparison** explicitly returns to the active capture.
Use **Retry inspection** for temporary query refusal and **Reconnect** after a lost
review stream. **Close comparison** retires the current pair.

At paired desktop widths, pan each source pane horizontally when a long condition is
clipped; Back/Forward retains those independent reading positions. The
[validation record](../../experiments/review-workflow/WALKTHROUGH_RESULTS.md) includes
both guided review variants, recordings and remaining scale/usability limits.
