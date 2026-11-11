# Arrange bubbles, groups and saved tasks

Use these controls to keep a reading task manageable as you follow more calls.
Start with an open bubble in the [call workspace](follow-a-call-trail.md).

## Arrange and resize source

Drag a bubble's header to move it, or its lower-right grip to resize it. You can focus
the header or grip and use arrow keys. Collapse a bubble to keep its place without
showing the source; closing it removes the bubble and its visual connections.

**↕ Fit height** and **↔ Fit width** fit the currently loaded code section. They can grow
or shrink a bubble. Growing moves nearby bubbles down or right to make room; shrinking
leaves neighbours in place. They retain the bubble's position and the workspace zoom.
Long functions still use section paging and scrolling.

**Settings → Code layout → Original lines** disables smart reflow. Reflow only changes
display wrapping. Source locations and copied text retain their original meaning.

## Recover offscreen context

Drag or scroll the background to pan. Use **+**/**−**, or Ctrl/Cmd+wheel, to zoom.
**Fit workspace** shows the working set. Select a box in the bottom-right overview to
return to that bubble; hovering a box shows its name and source location.

Scrolling inside source reads that pane. Scrolling a caller/reference stack reads its
results rather than panning the workspace.

## Group related views

Choose **Workspace → Groups**, give the group a name and select its members. Source
bubbles and result stacks can share a group. Selecting an item from another group moves
it into this group; an item belongs to at most one task group.

Drag the group header, or focus it and use arrow keys, to move the members together.
Collapse/Expand retains their positions. **Ungroup** leaves the members on the canvas.
**Close** removes the group and its members. Newly opened callees inherit their caller's
group when they are first added.

Language module labels and colours are independent of task groups. Enable **Settings →
Module outlines** for optional contours around shared modules; unknown membership stays
neutral and the overlay does not rearrange source.

## Name and switch tasks

Use **Workspace → Rename task** to name the current arrangement. **New task** saves it
and opens an empty task. **Task shelf** switches tasks and offers Rename/Delete for saved
entries. Open another task before deleting the current one.

Source bindings, groups, geometry, zoom and reading positions are saved locally. Reference
stacks save their query and request fresh results when resumed. If storage fails, Vizzi
keeps the working layout and shows an error; free storage or delete an unneeded saved
task before trying again.

Keep using the same browser profile, server address and configured repository to return
to a task. A different port or browser has different storage. Current bounds and data
lifetimes are in [storage and persistence](../reference/storage.md). Visual tasks have no
export/import interface yet; exporting saved locations is a separate operation.
