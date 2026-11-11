# Storage and persistence

For users checking what will remain after navigation, reload or a server restart.

| Data | Stored where | Lifetime and limits |
| --- | --- | --- |
| Visual tasks | Browser local storage | Same browser profile and origin; task/source bindings may become unavailable |
| Theme and display preferences | Browser local storage | Shared across views at the same origin |
| Saved locations, notes and reader connections | Server memory, or `--investigation-file` | Session-only without a file; persisted with a configured file |
| Captured source and pinned views | Native retained source/session data | Older captures can expire; pinning is not permanent storage |
| Active review comparison | Running server | Not persisted across server restart |
| Analysis cache and semantic working data | `--cache` directory | Derived data, separate from saved-location state |

An origin includes scheme, hostname and port. `127.0.0.1:7474`, `localhost:7474` and
`127.0.0.1:7475` have different browser storage. Returning to a visual task therefore
requires the same address/profile as well as the intended repository. Clearing site
data removes browser tasks and preferences.

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
