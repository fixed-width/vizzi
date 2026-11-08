# Try an investigation, leave it, and return

The 2026-11-08 direction correction supersedes this search-led protocol as the active
next step. See [visual working sets](../explanation/visual-working-sets.md). Preserve
existing notes; the prior setup check is not a completed visual-working-set evaluation.

For the maintainer deciding whether Vizzi adds useful context beside VS Code. Allow about
20 minutes. This is a usability/retention check on a task you care about, not a fresh search
or model evaluation. Existing source guides and previous familiarity can affect the result;
record that before starting.

The current task is: **Trace Minitosis's input-to-output path in tract-plugin-pack. Explain
which source supports your understanding, and record important branches or uncertainties.**
The participant investigation starts empty. No expected files, symbols or answers are supplied.
The source checkout is used for reading; avoid edits/builds during this short comparison.

1. **Start in your editor, about five minutes.** Use normal VS Code navigation, searches,
   open tabs and your usual notes/bookmarks. Write a short current explanation and what you
   would need to reconstruct if interrupted. Stop at five minutes even if incomplete.
2. **Add Vizzi, about five to eight minutes.** Open the session's trial URL. Find source
   areas or declaration names; inspect/use candidate calls if helpful. Keep only locations
   you expect to need later. Read normally in the editor and use its normal tools too.
   Record duplicated work, confusion or a missing view instead of working around everything.
3. **Retain the explanation.** In **Saved locations**, use **Add or edit a question**.
   **Inspect evidence** opens a saved item's note; explain why a location matters or what is
   uncertain. **Explain a connection** is available after two items if a relationship is
   worth retaining. Connections are your assertions; candidate calls are not semantic proof.
   Check whether the saved excerpt includes the code you meant to keep.
4. **Take a five-minute break.** Do unrelated work. Close the Vizzi tab if convenient;
   leave the editor in the state you would normally leave it. Avoid consulting agent-written
   source answers during the session. A configured investigation file preserves saved state;
   the temporary selected-call trail itself is not a saved investigation.
5. **Return, about two minutes.** Reopen the session's Saved locations URL. Before new
   searching, explain what you now understand and the next question you would investigate.
   Record what the saved question/anchors/notes/connections recovered, and what you still
   had to rediscover. Then work normally if more reading is needed.

Use the short [worksheet](../../experiments/companion/maintainer/WORKSHEET.md); the running
session provides a local writable copy beside the investigation file. Rough times are
optional. One concrete helpful moment and one concrete friction point matter more than
counts of cards or clicks. Stop early if the workflow is getting in your way.

The structural view searches literal names/areas, not whole natural-language questions.
**Area inventory** clears the current name filter. Call destinations are same-name
candidates; unsupported/partial/omitted analysis is shown. **Inspect call → Keep this location**
starts a finite saved window at the invocation. **Open in VS Code** verifies the working
file before issuing the CLI handoff; it cannot check an unsaved editor buffer. No editor
opens from browsing alone. The ordinary overview is available at the same host's `/`.

Repeating the editor-only task with Vizzi teaches the answer, so a faster second attempt
cannot establish a speed improvement. This session instead asks whether the retained
context earns its place beside the editor and helps you resume. Results remain pending
until the maintainer records observations; automated checks are setup verification only.
