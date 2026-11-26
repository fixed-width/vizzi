# Navigation between views

For contributors maintaining Vizzi's shared browser interface.

The shared navigation row places the Vizzi home link before **Source browser**, **Call
workspace**, **Review changes**, and **Saved locations**, followed by the **Settings** cog. The
current view is visibly selected and exposed with `aria-current="page"`. Links navigate
in the same tab. Source browser opens at the launch URL and at `/explore`. Call workspace
opens at `/workspace`. A newly opened VS Code tab also defaults to Source browser when
no specific destination was requested. Opening the tab is confirmed as soon as the header
is visible. Opening a specific file waits for captured source and space in the code pane.
Saved locations opens `/investigate#saved`. The bare `/investigate` route shows the same view.

Mode-specific actions occupy a second row. Workspace task/group, bubble and zoom controls,
source/review history, and location actions retain their current behaviour. Status text
belongs beside the controls for the view it describes. Settings always starts with
**Theme**. **Code layout** follows it in Source browser and Call workspace and shares
one preference across both. **Module outlines** follows **Code layout** in Call workspace.
Navigation and location actions do not appear in Settings. Menu exclusivity, outside
dismissal and Escape/focus return work consistently across views.

The shared header module owns link definitions, current-view indication, common settings
and menu behaviour. Each page supplies its existing controls and any extra settings through
one installation interface. Theme application stays with the existing appearance module,
including the source browser's palette callback. Moving between modes retains theme,
code layout and saved call-workspace state.

Reuse the existing Noto Sans/DejaVu Sans type and theme variables. The light palette remains
paper `#ffffff`, background `#f3f6f9`, text `#183044`, muted `#526477`, border `#b8c7d6`
and accent `#265dc5`; dark mode uses the existing theme equivalents. Keep the brand at 23px,
navigation/actions around 12–13px and status at 11–12px. Use alignment and the selected-view
indicator to establish hierarchy, without adding decorative motion or a new palette.

On wide screens, navigation sits between the brand and Settings. On narrow screens it
wraps onto its own row; actions wrap independently. Links, menus and settings stay inside
the viewport and retain visible keyboard focus. The header must not cover source controls.

Validate real navigation through all four modes, theme persistence, task restoration,
keyboard/menu behaviour, narrow layouts and both colour schemes. Exercise the shared module
through its rendered interface, including its extra-settings seam.

[Validation evidence](../../experiments/visual/HEADER_RESULTS.md) records the browser,
native and mutation checks and their limits.
