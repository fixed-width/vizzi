# Storage and persistence

For users checking what will remain after navigation, reload or a server restart.

| Data | Stored where | Lifetime and limits |
| --- | --- | --- |
| Visual tasks | Browser local storage | Same browser profile and origin; task/source bindings may become unavailable |
| Theme and display preferences | Browser local storage | Shared across views at the same origin |
| Saved locations, notes and reader connections | Server memory, or `--investigation-file` | Session-only without a file; persisted with a configured file |
| Captured source and pinned views | Native retained source/session data | Older captures can expire; pinning is not permanent storage |
| Active review comparison | Running server | Not persisted across server restart |
| Analysis cache and semantic working data | Platform workspace cache, or `--cache` override | Derived data, separate from saved-location state |
| VS Code bridge connection | VS Code extension global storage | Local session credential and endpoint; sharing republishes it and stopping closes the connection |

An origin includes scheme, hostname and port. `127.0.0.1:7474`, `localhost:7474` and
`127.0.0.1:7475` have different browser storage. Returning to a visual task therefore
requires the same address/profile as well as the intended repository. Clearing site
data removes browser tasks and preferences.

## Cache location

Without `--cache`, Vizzi uses the platform's user cache location:

| Platform | Vizzi cache base |
| --- | --- |
| Linux | `$XDG_CACHE_HOME/vizzi`, or `$HOME/.cache/vizzi` when XDG_CACHE_HOME is unset, empty or relative |
| macOS | `$HOME/Library/Caches/vizzi` |
| Windows | Local AppData known folder, normally `%LOCALAPPDATA%\vizzi\cache` |

These follow the [`directories` platform conventions](https://docs.rs/directories/6.0.0/directories/struct.ProjectDirs.html#method.cache_dir).
Each source root gets `workspaces/<root-hash>` beneath that base. The hash uses the
canonical absolute source path: symlink aliases share a cache, while different roots
have separate directories. No Git repository is required. An explicit `--cache DIR`
uses that directory directly, without adding a workspace subdirectory.

Both automatic and explicit cache paths must remain outside the inspected root after
resolving symlinks. If the platform location cannot be determined or lies inside the
source root, startup reports an error requesting an explicit `--cache DIR` override.
This can happen when inspecting the whole home directory. No source-local cache is
created as a fallback. Linux behaviour is checked locally; macOS and Windows path
selection uses the platform library and has not yet been qualified on those systems.

## Current task bounds

| Item | Bound |
| --- | --- |
| Source bubbles per task | 24 |
| Caller/reference stacks per task | 4 |
| Named groups per task | 8 |
| Saved tasks per shelf | 12 |
| Admitted reference rows per query | 256, displayed eight at a time |

Storage byte limits and browser quotas also apply. A failed task save/switch retains the
working layout and reports an error; the intended persistent write has not succeeded.
Visual tasks currently have no export/import interface and do not synchronise between
devices. [Managing tasks](../how-to/manage-tasks.md) describes the controls.

## What a saved layout contains

A visual layout retains source bindings, positions, groups, zoom and reading state.
Reference stacks retain their query rather than saved result text; reopening requests
current results. Loaded source may remain on screen after an edit, but a later reload
or page request can report that an old capture is unavailable. Layout persistence is
not a durable snapshot of all the code shown.

Saved locations are a different record: source anchors, captured excerpts, notes and
reader-authored connections. Export locations downloads that record, not the visual
workspace. The configured state file must be outside both the source root and cache.

The cache can contain captured/derived project data, including private semantic mirrors.
Stop the server before clearing it, and keep the saved-location state file separate.
Deleting derived data does not preserve expired source captures or review comparisons.
