# Follow a function across a rename or move

[Issue #9](https://github.com/fixed-width/vizzi/issues/9) covers an explicit history
transition. The interaction uses the existing bubble actions menu and dialog styles.

1. In a Git history bubble, choose **Find earlier declaration…** from the bubble's
   actions menu. Start from the committed version at the boundary. Use **Previous
   change** to reach that version or **Choose revision…** to open a known commit.
2. Vizzi examines that commit and its immediate first parent. A candidate must have
   the same lexical scope, language and exact declaration text apart from the declared
   name. The earlier declaration must disappear at the edge, and the current
   declaration must appear. The bounded scan must find one matching declaration on
   each side. The dialogue shows both names, paths and commit IDs before acceptance.
3. Select **Use earlier declaration** to record the correspondence and continue in the
   same bubble. It is a reader-confirmed source correspondence. Closing the dialogue
   or losing the source binding during preparation retains the displayed capture.
   Missing, ambiguous or unsupported source leaves that history boundary visible.
4. The accepted transition counts as an earlier version even if the only change was
   the file path. Previous and Next change retrace the accepted timeline. Compare
   can open adjacent accepted versions across the transition and labels each side
   with its own path and Git revision. Calls use the displayed capture's evidence.
5. Layout, task membership and independent views retain their existing behaviour.
   Saved tasks restore accepted correspondence while the history session is available.
   Return to live tracks the original declaration used to open that timeline and
   retains the displayed source if the original declaration cannot be resolved.

The first slice covers an adjacent rename, a file move, or both with exact source
evidence. Intervening deletions, changed lexical scope, overload ambiguity, split and
merge, changed implementations, and incomplete candidate scans are unresolved.
