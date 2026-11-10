# Live working-set refresh interface

Reference for native-host and browser contributors. Behaviour and implementation rationale
are in [the live contract](LIVE_PRODUCT.md) and [architecture](LIVE_TECH.md).

`POST /api/visual/refresh` accepts JSON with these fields. Unknown fields are rejected.
All native identity references are 64 lowercase hexadecimal characters. Lines are one-based;
offsets and byte positions are zero-based. String limits are UTF-8 bytes.

| Field | Value and bound |
| --- | --- |
| `snapshot` | Expected current observation reference. |
| `views` | At most 24 live view bindings; pinned views are omitted. |
| `views[].id` | Unique presentation ID beginning `v`, with decimal suffix, at most 8 bytes. |
| `views[].tracking` | Native declaration tracking reference or null for exact current-ID migration. |
| `views[].symbol` | Earlier observation-bound symbol reference. |
| `views[].version` | Captured document version, or a different valid reference to request source bytes. |
| `views[].area` | Original candidate-ordering area, at most 4,096 bytes. Empty uses the function's path. |
| `views[].section_offset` | Section offset from the earlier declaration start, at most 100,000,000. |
| `views[].anchor` | Null, or `{offset, byte, text}`. Offset at most 100,000,000; byte at most 32,768; complete earlier line text at most 1,024 bytes. |
| `links` | At most 96 retained relationship descriptions. |
| `links[].id` | Unique presentation relationship ID, at most 1,024 bytes. |
| `links[].from`, `links[].to` | Endpoint presentation IDs, at most 8 bytes each. Missing live endpoints receive no fresh check. |
| `links[].spelling` | Written callee spelling, 1–256 bytes. |
| `links[].kind` | Earlier evidence kind: `self`, `candidate` or `definition`. |

The response has the same `snapshot`, one result per requested view and one per link.
An unresolved view has `{id, state: "unresolved"}`. A resolved view has `state: "current"`,
its current `symbol`, `position`, visible `actions`, and the first eight written `calls`.
`position` contains the loaded section's first line, a matched reading `line` or null,
segment `byte`, and a `reset` flag. The visible window has at most 96 physical lines.
Changed or missing display buffers receive a `window` and a declaration-header `origin`;
unchanged captured versions receive null for both. Source windows retain existing byte,
style and reflow bounds.

Each link has `{id, check}`. A null check means an endpoint was not resolved in this batch.
Otherwise `check.state` is `current`, `removed` or `unverified`; only a current check
provides fresh `kind` and call-site `source` evidence. Previous language-server evidence
does not become current without another explicit definition lookup.

The route body limit is 192 KiB. Invalid identity/size bounds return 400; malformed JSON
shape or unknown fields return 422; oversized bodies return 413. An observation change
before or during preparation returns 409. Missing indexing or a busy structural operation
returns 503. The route shares the host's existing blocking-operation allowance and native
structural cache. An error does not mutate the reader's layout or source bindings.
