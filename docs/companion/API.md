# Investigation host interface

The companion uses one validated investigation per configured workspace. Portable data
has schema 1. At most 32 evidence cards, 64 reader connections and 512 KiB encoded state
are admitted. Excerpts cap at 4,096 UTF-8 bytes, titles at 128, question/notes at 2,048 and
connection explanations at 512. Native state is outside the source checkout.

| Request | Input/result |
| --- | --- |
| `GET /api/discovery` | Optional directory `path`, name/path `search`, and `offset` → at most 24 areas or source locations, scope/language/coverage counts, matching count and next offset. Metadata only; no source IO. |
| `GET /api/investigation` | Saved board, persistence/editor labels and separate current-index freshness per evidence ID. Does not start indexing. |
| `GET /api/investigation/lookup?path=…` | Normalised relative indexed path → snapshot/file/version/path/line-count. |
| `POST /api/investigation/evidence` | JSON revision, snapshot, file, version, line, title → captured evidence ID and current view. |
| `POST /api/investigation` | Tagged reader edit: question, annotate, connect, disconnect or remove, each with expected revision. |
| `POST /api/investigation/evidence/{id}/open` | JSON `{}` → verified editor handoff requested; the executable/arguments cannot be supplied by the request. |
| `POST /api/editor/open` | JSON `{snapshot,file,version,line}` → verified current-source handoff, without saving evidence. `line` is the original one-based source line. No path, executable or arguments are accepted. |
| `POST /api/visual/refresh` | Coherent current bindings, source windows and relationship checks for a visual working set. See [the live interface](../visual/LIVE_API.md) for its schema and 192 KiB body limit. |

Mutations require JSON and deny unknown fields. Except for visual refresh, bodies cap at 16 KiB. Obsolete board or
source versions return 409; invalid bounds/links 400; unavailable source/index 503;
persistence failure 500; editor command failure 502. Failed save/edit leaves the last
investigation active. Two blocking operations and one editor handoff are admitted at a
time. Index snapshots and source ownership retain their existing limits.

Editor verification requires current captured content/path/workspace agreement and a
no-follow regular-file read with matching complete content identity. Verification reads
cap at 16 MiB. Symlinks, deleted/changed paths, invalid saved line numbers and unsupported
verification fail explicitly. A successful CLI result does not confirm the current
editor buffer; unsaved edits and changes after verification need plugin-level handling.
Direct source handoff additionally requires the exact current snapshot and document version,
with a line inside its captured text. Stale source returns 409, invalid locations return
400, invalid JSON shape/extra fields return 422, and missing live indexing returns 503.
Neither the evidence list nor its revision changes. Direct and saved handoffs share the
same single editor-operation allowance.

The primary page is `/` (also `/investigate`); `/explore` and `/review` are supporting
surfaces. The primary page activates read-only indexing automatically; fetching saved
state or discovery metadata does not itself start indexing. Discovery uses one immutable
snapshot per response and returns exact file/version/line bindings for existing windows
and evidence capture. Paths cap at 4,096 UTF-8 bytes and search at 256; invalid input is
400 and absent live indexing is 503. Name search is case-insensitive lexical substring
matching, not source-body or semantic search. Existing file/window/review interfaces keep
their source identities and bounds.
