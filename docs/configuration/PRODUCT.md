# Launch configuration and shared editor analysis

For contributors implementing persistent launch settings and editor integration.
This contract covers the first VS Code plugin slice; other editors can implement the
same connection later.

## Summary

Users can save Vizzi launch settings in a TOML file and override them on the command
line. A VS Code plugin can serve definition/reference queries through VS Code's existing
providers so Vizzi does not need a second language-server process.

## Behaviour

1. `vizzi --config FILE` loads one explicitly selected TOML file. No file is loaded
   implicitly from an inspected repository. Existing launches without this option
   retain their defaults. Files are read once at startup; changes require a restart.
2. File keys use the existing long option names, without `--`. Language-server options
   occupy `[language-servers]`. Diagnostic modes remain command-line-only. Unknown keys,
   wrong types, unreadable/oversized files and invalid effective settings fail startup
   with actionable errors before analysis or server processes start.
3. Explicit command-line values override file values. Repeating a source-selection
   option appends command-line rules to the corresponding file rules. Existing policy
   specificity and conflicts still apply; ordering never changes language precedence.
4. File paths resolve relative to the file's directory. Source-selection scopes remain
   relative to the selected source root. Bare executable names use PATH; executable
   paths containing directory components resolve relative to the file. CLI paths retain
   their current-directory interpretation. No shell or environment expansion is done.
5. Help remains available without reading the configuration or creating storage.
6. `--vscode-connection FILE` or `language-servers.vscode-connection` selects editor
   analysis instead of Vizzi-managed servers. Combining the two ownership modes is an
   error. No connection attempt or server process is needed for `--once` capture.
7. The VS Code plugin offers an explicit **Vizzi: Share Language Providers (Advanced)** command,
   selects one local workspace folder, and displays its connection file and launch
   command. It requires a trusted local workspace. Different roots get independent
   connection files. Sharing the same root from another window selects the most recent
   window; stopping the earlier connection does not disrupt the selected one.
8. The connection is local, authenticated, versioned and bound to the canonical source
   root. Vizzi reads the connection file afresh for each query, allowing a restarted
   bridge to reconnect. Unsupported versions, wrong roots, missing connections,
   timeouts and disconnects produce unavailable semantic analysis. They never start a
   duplicate server. Existing labelled name fallback remains available.
9. The plugin invokes VS Code's definition/reference providers. VS Code retains
   ownership of its servers and configuration. Vizzi never sends initialization,
   document edits, shutdown or exit to those servers. Provider invocation may activate
   the normal VS Code language extension; this is not a guarantee that VS Code already
   had that provider running.
10. Editor analysis is available only while the captured workspace matches saved disk
    source, the query text matches the editor document, and the selected workspace has
    no dirty file documents. Query-time editor/configuration changes invalidate the
    response. Returned source positions must match captured target text. Pinned or
    older source that differs from disk receives unavailable analysis, never a result
    from substituted live bytes. The plugin never saves, reloads or edits user buffers.
11. The bridge admits captured repository results and locally available Rust dependency
    source reached through editor definitions. Dependency files use independent bounded
    captures, never enlarge the repository scope and retain package provenance. Other
    external languages and non-file targets remain outside capture. Shared results do
    not claim frozen whole-project editor analysis.
12. The connection file contains a session credential and stays outside the inspected
    source. Stopping sharing or deactivating the extension closes the bridge without
    stopping the editor's language servers. The last endpoint file remains for stable
    configuration paths; its credential cannot authenticate to a stopped bridge. No browser
    request may choose an executable, connection address or connection file.

## Installed VS Code workflow

The VSIX includes the matching native executable for its operating system and processor.
After installation, **Vizzi: Open Workspace** selects the open project, shares its editor
providers, starts that executable and opens the visual workspace in an editor tab. A
status-bar entry makes this command discoverable. No TOML file or separate terminal is
required for the default launch.

- With one workspace folder, use it directly. With several, ask which folder to open.
  With none, show an actionable error. Starting, cancellation and errors are recorded
  in the Vizzi Output channel, which opens as soon as a command starts.
- Automatic launches configure the editor connection internally on every run and never
  prompt the user to copy connection paths into TOML. Optional external-launch settings
  are shown only by **Share Language Providers (Advanced)**, which points to Open Workspace
  as the automatic setup path. A project's connection path stays stable within a profile;
  its session credentials are refreshed when sharing restarts.
- An optional `vizzi.toml` in the selected project root supplies launch settings when
  the user invokes Open Workspace. The selected root and shared editor connection take
  precedence. Native CLI launches still select configuration explicitly with `--config`.
- Reopening the same project reuses the extension-owned native process. Changing projects,
  stopping the workspace or deactivating the extension closes the owned process.
  Stopping sharing alone retains the existing native process and its source views.
- Startup errors include the native diagnostic. Browser-opening errors retain the
  running workspace and display its address. Existing manual provider-sharing commands
  remain available for independently launched Vizzi processes.
- Project-local TOML files are supported. Only the generated session credential and
  runtime storage require locations outside the inspected source.

**Open Workspace** presents the local UI in a Vizzi editor tab by default. Reopening
reveals the existing tab without reloading its reading state, and hiding it retains
the live workspace. Closing the tab leaves the native process available until Stop
Workspace or window shutdown; reopening creates a new tab at the same local address.
Stopping the workspace or changing projects disposes the old tab. The separate
**Vizzi: Open Workspace in Browser** command opens the same running server externally.
Saved tasks in an external browser and the VS Code tab belong to their respective
browser storage profiles.

## Shared Rust dependency source

Following an editor-resolved Rust call can open a locally installed dependency or standard
library method beside its caller. No additional language server, download or build is
started. Captured dependency source remains immutable; saved bytes must still match for
another semantic query. Existing dependency limits, persistence and editor handoff apply.

When native syntax recovery misses the returned function, the editor may supply its
function range and exact identifier. Both are checked against saved source and retained
with that capture. This supports standard-library methods using newer syntax such as
`[const]` bounds without guessing a same-name declaration or rewriting source. Such
function ranges do not manufacture raw call analysis; missing syntax analysis remains
explicit. Dependency queries use paths from retained native captures, never browser paths.

## Explore a call from the editor

**Vizzi: Explore Call** appears in the source editor's right-click menu and Command
Palette. From a saved local project file, it captures the call-name position before
changing focus, starts or reuses Vizzi, and opens its Call workspace tab. The current
task gains the enclosing declaration and follows the selected call beside it. Existing
bubbles, groups and reading positions are retained; repeating the action reveals the
existing views. Ambiguous or unavailable targets use the ordinary destination chooser.

The editor text must match saved disk source and Vizzi's current capture. Dirty files,
source-file symlinks, excluded/unsupported source, non-call positions and calls without
a captured enclosing declaration produce actionable errors. The command does not save
buffers or guess a same-name target. It starts from captured project source; following
the resulting call can enter an admitted Rust dependency as usual.

The handoff uses a source-checked native call identity. Only opaque snapshot/call IDs
cross the tab's URL fragment; source text, credentials and native paths do not. Changing
from another Vizzi mode returns to Call workspace. The fragment is consumed, so the
same call can be requested again without reloading an existing Call workspace.
