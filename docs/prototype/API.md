# Local host API reference

Schema version **1**. These are prototype contracts, independent of Git. Identities are
opaque strings. `schema` appears in status, view, and pinned-source JSON responses.
HTTP responses disable caching. The host serves `/`, `/app.js`, `/controller.js`, and
`/app.css` and `/updates.js` without external assets.

| Request | Parameters | Response |
| --- | --- | --- |
| `GET /api/status` | None | Workspace name, generation, snapshot, pending flag, measurements, refresh error, observation mode |
| `GET /api/view` | Optional `snapshot`, `file`, `entity`, `filter`, `limit` | One snapshot's bounded overview, selection, captured source, and outlines |
| `GET /api/source` | Required `snapshot`, `file` | Captured source from the requested retained observation |
| `GET /api/events` | None; `Last-Event-ID` does not request replay | Latest-state `workspace` events over SSE, followed by changes |

`filter` is a case-insensitive path substring. `limit` defaults to 2,000 and is clamped
to 1–2,000. `total` counts all discovered Rust files; `matching` counts matches before
limiting. The view includes world-space tile rectangles, directory labels, and height.
Tile coverage combines all configured analysis contexts. Source outlines retain each
context's identity, coverage, diagnostics, and declarations separately.

An existing selected file remains selected even if it is outside the overview filter.
An absent selected file falls back to the first filtered file; an empty result has null
selection and source. A declaration absent from the selected file falls back to file
selection. Each declaration contains inclusive, one-based start/end lines derived from
the captured source's UTF-8 byte ranges. Source text is null for unsupported encoding.

`snapshot` identifies captured contents and facts. `generation` counts distinct published
observations and stays unchanged after a no-op reconciliation. A source request for an
unknown or expired snapshot returns **409**. A missing file within a retained snapshot
returns **404**. The host retains three previous observations alongside the current one;
outstanding queries may hold their own immutable handles. A view before initial capture
returns **503**, and malformed typed query parameters return **400**.
The optional `snapshot` view parameter has the same retention and 409 rules; omitted,
the handler uses the current observation. A pinned view never substitutes another
snapshot for an expired version.

Change notifications have `schema`, `stream`, decimal-string `revision`, and the full
`status` record. SSE uses event type `workspace` and id `<stream>:<revision>`. The first
event is current state; slow consumers may skip intermediate states. Every reconnect
resynchronises from current state, including after a server restart. See the
[notification contract](CHANGES.md) for ordering, closure, and client rules.

Coverage values are `syntax` (parsed without grammar recovery), `partial` (recovery
occurred), and `unavailable`. None claims compiler-level resolution or complete build
configuration coverage.

Measurements count files, captured bytes, parsed files, in-memory reuse, persisted
cache hits, and cache write warnings. `capture_ms` includes discovery and reads;
`total_ms` includes capture, verification, analysis/cache activity, and snapshot creation.
Transport, debounce, notification delivery, and painting are outside those timings.

In stdio mode, requests are JSON lines `{"id":1,"uri":"/api/view"}`. Responses are
`{"id":1,"status":200,"content_type":"application/json","body":"..."}`. The body
is the HTTP response body as text, including JSON encoding where applicable. Stdio
uses the same router and live application, with no listener required. The unbounded
`/api/events` endpoint returns **400** through diagnostics; use HTTP or the native
subscription interface for streaming.
