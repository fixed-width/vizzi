# Use the trial workspace

The facilitator supplies a learning URL or review URL and a disposable source copy.
Changes to that copy do not change the original checkout. The trial launch does not
start screen/audio recording; agree separately if a recording would help. You can
pause or stop at any time.

The learning page starts with a source overview. Choose an area or use **Find a name or
file**, then select a name and **Keep this location**. No known path or line is required.
**Saved locations** keeps context while you read in VS Code. Questions and connection
controls are optional and open on request. **Inspect evidence** opens notes and a short captured preview.
Connections are your assertions. **Open in VS Code** verifies the saved source against
the current index and working file; changed or unavailable source stays in the
investigation and produces an explanation. The editor opens a live file and may have
unsaved edits; this CLI handoff cannot confirm the buffer's bytes. Export keeps a
portable copy; the save-mode label indicates whether host persistence is configured.

**Browse source** at `/explore` links a file map, captured source and a declaration outline. Use
the path filter or accessible file list to find files. Selecting a declaration reveals
its source. Kind badges identify types and callables; **Line N** is the starting source
line. Use the overview navigator to jump across the map, or +/− and Fit map to zoom.
Drag dividers to resize sections; focused dividers also accept Left/Right and Home/End.
Back/Forward returns to earlier reading locations.

The review page compares a baseline with captured working files. Use the baseline
from the task, repository path `.`, and **Compare with working tree**. The file list, hunks,
before/after panes and outlines share the same comparison. **Include unchanged
context** makes related unchanged source available inside that capture. The panes
scroll independently; long lines have fragment controls. Repository path selects a file
or directory. **Two commits** mode instead compares an explicit base and target; its
right pane is Target and working refresh is hidden. Invalid input appears beside the
form while the previous capture stays visible.

Source views are captured observations. **Refresh working** observes another working
state. Availability labels distinguish absent source from empty or unsupported text.
Outlines show lexical syntax; they do not establish resolved calls or semantic impact.

First complete the task without a prescribed route. Afterwards, if useful, try
returning to an earlier file and refreshing a comparison. Record whether the selected
file, side and reading positions behave as you expected. Keep these optional probes
separate from the task attempt.

Directory frames show the actual hierarchy. Use the directory controls or breadcrumbs
to focus/collapse an area. Drag rows or headers to reorder siblings; Alt+Up/Down and
the arrow buttons also reorder them. This changes the map, not source paths. Reset map
restores defaults. Theme offers System, Light and Dark. Only ephemeral full-width banners outside the panes can be closed with their labelled × controls.

A revision trial needs history in the configured checkout. Git expressions such as
HEAD~1 and HEAD^^ cannot reach missing shallow parents. The maintainer preparation can
include local history; the application itself never fetches.

Declaration rows show their nearest enclosing scope and depth while keeping names aligned.
