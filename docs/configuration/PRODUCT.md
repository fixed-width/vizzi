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
7. The VS Code plugin offers an explicit **Vizzi: Share Language Providers** command,
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
11. The first bridge admits repository-local results only. External/non-file targets
    remain explicitly outside capture. Existing Vizzi-managed adapters retain their
    independent dependency-source support. Editor context can include dependencies
    outside capture; shared results do not claim frozen whole-project analysis.
12. The connection file contains a session credential and stays outside the inspected
    source. Stopping sharing or deactivating the extension closes the bridge without
    stopping the editor's language servers. The last endpoint file remains for stable
    configuration paths; its credential cannot authenticate to a stopped bridge. No browser
    request may choose an executable, connection address or connection file.
