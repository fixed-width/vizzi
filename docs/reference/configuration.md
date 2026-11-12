# Configuration files

For users looking up persisted native launch settings. Setup examples are in
[save launch settings](../how-to/configure-vizzi.md).

`--config FILE` selects one TOML file. There is no implicit project/user discovery or
environment-variable settings layer. The file is read once at startup and is limited
to 1 MiB. Unknown keys, duplicate keys, incorrect types and invalid effective settings
are errors. `--help` skips config loading. A second `--config` is an error.

## Precedence and paths

Built-in defaults apply first, then the file, then explicit command-line options.
Scalar options on the CLI replace file values. Repeated CLI source-selection options
append to their file arrays; normal scope specificity and conflict checks apply.

File paths are relative to the configuration file's directory, including a relative
`root`. An omitted root still means the process's current directory. Bare executable
names use PATH; executable paths with directory components use the file's directory.
Source scopes always use the selected root. Explicit CLI paths use the process's
current directory. No `~`, shell-command or environment-variable expansion is performed.

## Top-level keys

| Key | TOML type | Corresponding option/default |
| --- | --- | --- |
| `root` | String path | `--root`; current directory |
| `cache` | String path | `--cache`; platform workspace cache |
| `workers` | Integer, 1–32 | `--workers`; 4 |
| `port` | Integer, 0–65535 | `--port`; 7474 |
| `investigation-file` | String path | `--investigation-file`; session memory |
| `editor-code` | String executable | `--editor-code`; `code` |
| `syntax-cache-mib` | Nonnegative integer | `--syntax-cache-mib`; 0 |
| `analysis-input-mib` | Nonnegative integer | `--analysis-input-mib`; 0 |
| `source-spool-mib` | Nonnegative integer | `--source-spool-mib`; 0 |
| `c-family-root` | Array of strings | Repeated `--c-family-root`; empty |
| `exclude-source` | Array of strings | Repeated `--exclude-source`; empty |
| `syntax-language` | Array of `"SCOPE=c"`/`"SCOPE=cpp"` strings | Repeated `--syntax-language`; empty |

Resource controls retain their [CLI meanings and limits](cli.md#resource-controls).
`once`, `stdio`, `help` and `config` are command-line-only and are rejected as file keys.
The config file holds launch settings; [browser preferences and saved state](storage.md)
have separate storage.

## Language-server keys

These keys appear under `[language-servers]` and accept strings:

| Key | Meaning |
| --- | --- |
| `rust-analyzer` | Executable for Vizzi-managed Rust definitions/references |
| `pyright` | Executable for Vizzi-managed Python definitions/references |
| `ruby-lsp` | Executable for Vizzi-managed Ruby definitions/references |
| `clangd` | Executable for Vizzi-managed C/C++ definitions/references |
| `python-path` | Python executable/environment; requires `pyright` |
| `compile-commands` | Compilation database path; requires `clangd` |
| `vscode-connection` | VS Code plugin connection file; equivalent to `--vscode-connection FILE` |

All providers default to disabled. Multiple managed servers can coexist. Selecting
`vscode-connection` uses editor-owned analysis for supported languages and cannot be
combined with managed-server, interpreter or compilation-database options. A disconnected
editor never triggers a managed fallback server. Each query rereads the connection file.

The bridge uses VS Code's saved project context. Dirty or changed buffers and mismatched
captured source make results unavailable. External/non-file targets remain outside capture.
It does not freeze the editor's full dependency/configuration environment or supply
semantic results for older captured bytes that differ from disk. Managed servers retain
their private captured-source mirrors and dependency-source support.
VS Code's provider commands cannot distinguish every missing-provider case from an
empty result; an empty shared result does not establish complete analysis coverage.
