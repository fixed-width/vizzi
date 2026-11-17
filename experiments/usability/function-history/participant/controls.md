# Trial control reference

This reference describes controls without prescribing a route through the task.
The trial launches no screen or audio recording. You can pause or stop at any time.

| Control | Behaviour |
| --- | --- |
| VS Code Command Palette → **Vizzi: Open Workspace** | Opens the call workspace in an editor tab for the source folder. |
| **Add bubble**, **Find a function**, **Find functions** | Finds declarations to add to the current working set. Qualified names and paths distinguish results. |
| Highlighted call / **Calls** | Offers destinations. A name candidate is not proof of receiver type or runtime dispatch. |
| **Previous change** / **Next change** | Reads distinct versions of this declaration on local first-parent history. Other bubbles retain their own versions. |
| **Compare** | Pins an **After** bubble and opens the adjacent distinct **Before** version beside it. It compares the version currently displayed. |
| **Previous edit** / **Next edit** | Reveals corresponding changed regions in the comparison. |
| **Previous section** / **Next section** | Pages longer functions while keeping original file line numbers. |
| **Open in captured file** | Follows a call within that exact captured file. Cross-file destinations may remain unresolved. |
| Destination chooser | Lists possible targets and their evidence. Choose explicitly when several candidates remain. |
| Bubble header / resize control | Moves or resizes that bubble. Pan, zoom and **Fit workspace** help recover other views. |
| **Return to live** | Resolves that bubble against saved source in the supplied checkout. Earlier captures and other bubbles retain their own context. |
| **Editor** | Opens verified current source in the originating VS Code window. Captured views restrict current-source actions. |
| **Vizzi: Stop Workspace** | Stops the extension-owned workspace when the session is finished. |

Revision labels identify the source represented by each bubble. History skips commits
that do not change the declaration; comparison sides need not be neighbouring Git commits.
Returning live refers to the disposable checkout, which stays at the supplied commit.
No checkout, fetch or inspected-project build is required to read these views.

If a label, choice or unavailable operation is unclear, describe what you expected before
seeking help. The facilitator records any assistance and any operations tried after your
answers separately.
