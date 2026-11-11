# Command-line options

For users looking up launch and analysis options. Build instructions are in
[installation](../how-to/install.md); operational examples are in
[run a project](../how-to/run-workspace.md).

```text
vizzi [--root DIR] [--cache DIR] [options]
```

The HTTP server binds to `127.0.0.1`. It prints the address it actually uses. Browser
assets are embedded in the executable. There is no separate frontend server.

## Launch and integration

| Option | Default | Meaning |
| --- | --- | --- |
| `--root DIR` | Current directory | Directory to inspect, without searching parent directories for a repository. Git review uses a repository root; Rust semantic navigation uses a Cargo project/workspace. |
| `--cache DIR` | Platform user cache, scoped to the source root | Override the cache directory. Must be outside the inspected root, including through symlinks. See [cache locations](storage.md#cache-location). |
| `--workers N` | `4` | Shared analysis worker count, from 1 to 32. User guides use 1. |
| `--port N` | `7474` | Local HTTP port. `0` requests an available port; its address can change between runs. |
| `--investigation-file FILE` | None | Persist saved locations, notes and reader connections. Must be outside both root and cache. Does not persist visual task layouts or review captures. |
| `--editor-code EXECUTABLE` | `code` | VS Code launcher path/name. This accepts an executable, not an executable plus shell arguments. |
| `--rust-analyzer EXECUTABLE` | Disabled | Optional Rust definition/reference provider. Dependencies and toolchains must already be available locally. |
| `--pyright EXECUTABLE` | Disabled | Pyright-compatible Python stdio language server. |
| `--python-path INTERPRETER` | Server default | Python interpreter/environment; requires `--pyright`. |
| `--clangd EXECUTABLE` | Disabled | C/C++ language server; source scopes and compilation context remain explicit. |
| `--compile-commands FILE` | Root database or flags file | Captured compilation database; requires `--clangd`. |
| `--help`, `-h` | — | Print usage and exit. |

Relative root/cache overrides resolve against the process's current directory.

## Source selection

| Option | Meaning |
| --- | --- |
| `--c-family-root SCOPE` | Admit C-family candidate source in a relative file/directory scope. `.` selects the whole root. |
| `--syntax-language SCOPE=c` or `SCOPE=cpp` | Assign raw C/C++ syntax for a scope, including ambiguous headers. |
| `--exclude-source SCOPE` | Exclude a file/subtree from capture across all languages. |

These options are repeatable. Scopes are relative to `--root`; absolute paths and traversal
are rejected. More specific language assignments win; conflicting assignments at the same
scope fail startup. Flag ordering does not determine precedence. The
[language reference](languages.md) describes the resulting coverage.

## Resource controls

All three controls below default to `0`, which disables that optional mechanism. They
are independent controls, not a process-memory limit.

| Option | Meaning |
| --- | --- |
| `--syntax-cache-mib N` | Weighted budget for retained syntax trees. This is not a heap/RSS cap. |
| `--analysis-input-mib N` | Budget for active analysis input bytes. Files can become deferred when admission is unavailable; it is not a whole-process memory cap. |
| `--source-spool-mib N` | Capacity of private session backing storage for captured source. It is not durable pinned-version storage. |

Larger repositories and long/generated files can encounter other source, declaration or
view limits. Labels expose unavailable or omitted coverage; zero results are not proof
of complete analysis.

## Diagnostics

`--once` performs one capture, prints a JSON status report and exits. `--stdio` starts a
live diagnostic process accepting one JSON request per stdin line rather than an HTTP
listener. The options are mutually exclusive; both use the normal root and cache defaults.

The diagnostic protocol is a contributor interface. Its request envelopes, pinned views
and streaming limitations are documented in the [host API reference](../prototype/API.md).
