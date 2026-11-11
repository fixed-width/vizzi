# Follow calls, callers and references

Use the Call workspace to read related functions without replacing earlier context.
Start Vizzi [on your project](run-workspace.md), or use the guided
[Python](../tutorials/python-call-trail.md) or [Rust](../tutorials/first-call-trail.md) lesson.

## Open a function

Choose **Add bubble**, enter a function or qualified name, and select **Find functions**.
Choose the result using its name and source path. Qualified Rust names such as
`Greeter::greet` are accepted; supported separator forms are in the
[workspace reference](../reference/workspace.md).

## Open a call beside its caller

Click a highlighted call name and choose **Open definition**. Ctrl-click or Cmd-click
follows directly. With the keyboard, Tab to a call and press Enter, Space or Down for
its menu; Ctrl/Cmd+Enter follows directly. Escape closes the menu and returns focus.

A proven unique destination opens directly. A sole complete name candidate can also
open directly, retaining its weaker candidate status. If Vizzi offers choices, inspect
names, containing declarations and file paths before choosing. A matching name alone
is not proof of a call target; see [call destinations](../explanation/call-destinations.md).

The caller remains visible. Follow another step or return to an earlier bubble to open
another branch. Opening an existing destination reveals it without changing the zoom.
Arrows and their hover details show the relationship's evidence or historical status.
Hover a destination bubble to highlight its source call lines; hovering the lines
highlights opened destinations and arrows.

**Calls** lists written calls when their names cannot be decorated in the current source
section. Long functions have Previous/Next section controls. Scroll inside a bubble
and use [layout controls](manage-tasks.md) to make room.

## Find callers or other references

Open a bubble's **⋯** menu and choose **Find callers** or **Find references**. A result
stack opens alongside the source. Hover or focus an available row for a preview, then
select it to open the containing declaration. A caller result adds its supported connection
to the target; a non-call reference opens source without inventing a call. Locations outside
an admitted declaration can be listed without an openable function bubble.

The stack identifies semantic language results or name-based candidates. Without a
semantic provider, references fall back to written-call candidates and cannot enumerate
all function-value uses. [Configure a language server](configure-language-servers.md)
for Python, Rust or C/C++ to use semantic definitions and references.

Use Previous/Next to page results. Counts report admitted matches, omitted results and
known locations outside captured source. Several calls in one function can produce
several rows. **Repository updated** means results need **Refresh**, even if a different
file changed. No results do not prove a function is unused.

## Continue in the editor

Use **Editor** to open a current bubble at its original declaration line in VS Code.
Saving a source location first is optional. Changed or unverifiable working-file bytes
can prevent the handoff. [Configure the editor](investigate-with-vscode.md) if needed.

Use [live/pinned views](follow-live-changes.md) when the project changes, or
[groups and the task shelf](manage-tasks.md) to keep the reading arrangement.
