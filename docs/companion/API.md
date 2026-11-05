# Investigation host interface

The companion uses one validated investigation per configured workspace. Portable data
has schema 1. At most 32 evidence cards, 64 reader connections and 512 KiB encoded state
are admitted. Excerpts cap at 4,096 UTF-8 bytes, titles at 128, question/notes at 2,048 and
connection explanations at 512. Native state is outside the source checkout.

| Request | Input/result |
| --- | --- |
| `GET /api/investigation` | Saved board, persistence/editor labels and separate current-index freshness per evidence ID. Does not start indexing. |
| `GET /api/investigation/lookup?path=…` | Normalised relative indexed path → snapshot/file/version/path/line-count. |
| `POST /api/investigation/evidence` | JSON revision, snapshot, file, version, line, title → captured evidence ID and current view. |
| `POST /api/investigation` | Tagged reader edit: question, annotate, connect, disconnect or remove, each with expected revision. |
| `POST /api/investigation/evidence/{id}/open` | JSON `{}` → verified editor handoff requested; the executable/arguments cannot be supplied by the request. |

Mutations require JSON and deny unknown fields. Bodies cap at 16 KiB. Obsolete board or
source versions return 409; invalid bounds/links 400; unavailable source/index 503;
persistence failure 500; editor command failure 502. Failed save/edit leaves the last
investigation active. Two blocking operations and one editor handoff are admitted at a
time. Index snapshots and source ownership retain their existing limits.

Editor verification requires current captured content/path/workspace agreement and a
no-follow regular-file read with matching complete content identity. Verification reads
cap at 16 MiB. Symlinks, deleted/changed paths, invalid saved line numbers and unsupported
verification fail explicitly. A successful CLI result does not confirm the current
editor buffer; unsaved edits and changes after verification need plugin-level handling.

The primary page is `/` (also `/investigate`); `/explore` and `/review` are supporting
surfaces. Existing file/window/review interfaces keep their source identities and bounds.
