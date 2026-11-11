# Shared header validation

Contributor reference for [the header contract](../../docs/visual/HEADER_PRODUCT.md),
2026-11-11.

All four modes use one header module and stylesheet. The navigation order is Call
workspace, Review changes, Source browser and Saved locations, with one current-page
indicator. Settings stays in the same place and starts with Theme. The call workspace
supplies its extra settings; each mode retains its own controls in the second row.
Previous header-specific styling and menu handlers were removed from the mode files.

## Behaviour checked

- Actual same-tab navigation through all four native routes retains the theme choice;
  returning to the call workspace restores the existing task's bubble geometry.
- Common links, current-page indication, retained control elements/listeners and extra
  settings are exercised through the rendered header interface.
- Pointer and keyboard menu opening are exclusive. Escape closes the menu and returns
  focus; outside clicks dismiss it. System/Light/Dark choices remain available.
- Wide and 360px layouts keep header controls visible. Settings and Locations action
  menus stay within the viewport when opened. Narrow mode menus use the header width.
- Light/dark screenshots were inspected for the call workspace and wide/narrow shots
  for the other modes. No new palette, font dependency or UI framework was added.

The initial browser test failed because no common view navigation existed. The default
integration check builds the native executable and uses its embedded assets. The optional
source-assets route is only a development shortcut. Older source-browser tests now name
the file-list disclosure explicitly, and theme interactions open Settings first.

## Verification

Formatting, workspace Clippy with warnings denied and the native workspace suite pass:
651 passing tests, seven opt-in tests ignored and 92 successful result summaries including
doctests. The full browser suite passes with the packaged header. The final toolbar CSS
cleanup is covered by the browser/interface checks rather than another native-suite run.

The complete shared JavaScript header module generated 94 Stryker cases. All 94 are caught
by failing tests, with zero survivors, timeouts or runner errors and no exclusions. Passing
unmodified baselines precede the runs. Initial survivors prompted keyboard-only menu,
invalid-input diagnostic and rendered hierarchy tests. Runs use one low-priority worker;
the browser receives the active mutation identifier explicitly.

[Compressed evidence](results/header/validation) retains the initial failure, passing
checks and final mutation report. The mutation scope is `app-header.js`; it does not
establish exhaustive coverage of the existing mode applications or stylesheets. Screenshot
inspection is not a usability study or a performance benchmark. Browser verification uses
the configured Chromium; no additional cross-browser qualification is claimed.
