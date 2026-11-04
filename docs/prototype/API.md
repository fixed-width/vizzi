# Local host API reference

The [comparison API](COMPARISON_API.md) adds separate lifecycle/status/SSE and bounded
token/side/version-bound review queries. [Task-driven startup](STARTUP_PRODUCT.md)
keeps the HTTP live index dormant until exploration or explicit activation requests it.

Bulk views and pinned source use schema **2**. Browser summary views use schema **3**,
and pinned document windows use schema **1**; see [WINDOWS.md](WINDOWS.md).
Status and notification envelopes remain
schema **1**. These prototype contracts are independent of Git. Identities are opaque
strings. HTTP responses disable caching. The host serves `/`, `/app.js`, `/controller.js`, `/navigation.js`,
`/updates.js`, `/startup.js`, `/source-description.js`, `/outline.js`, `/windows.js`, `/inspector.js`, and `/app.css` without external assets.

| Request | Parameters | Response |
| --- | --- | --- |
| `GET /api/status` | None | Workspace name, generation, snapshot, readiness, foreground pending flag, measurements, recovery metadata, refresh error, observation mode |
| `POST /api/workspace/start` | JSON `{}` | Activate live indexing or retry an initial failure; **202** with current status |
| `GET /api/view` | Optional `snapshot`, `file`, `entity`, `filter`, `limit` | One snapshot's bounded overview, selection, captured source, and outlines |
| `GET /api/window` | Required `snapshot`, `file`, `version`; optional source/outline ranges | Bounded text fragments, declaration page, and logical cursor from that exact pin |
| `GET /api/source` | Required `snapshot`, `file` | Captured source from the requested retained observation |
| `GET /api/events` | None; `Last-Event-ID` does not request replay | Latest-state `workspace` events over SSE, followed by changes |

`readiness` is an additive status-schema-1 field, derived in this order:

| Value | Condition |
| --- | --- |
| `ready` | A completed snapshot exists, including an empty observation. |
| `initializing` | No snapshot exists and an initial attempt is pending. |
| `failed` | No snapshot exists and the completed initial attempt failed. |
| `not_started` | No snapshot, pending attempt or error exists. |

Refreshes and failures retain `ready` when a previous snapshot is available. The
pending/error fields describe those attempts independently. Before initial publication,
snapshot and measurements are null. GETs and subscriptions never activate live indexing.
Review has its own lifecycle and does not require a live publication.

The host-aware activation endpoint requires a JSON content type, denies unknown fields
and limits the body to 1 KiB (**415**, **422** and **413** respectively). Repeated
activation while pending or ready does not restart work. The response disables caching
and content sniffing; completion is reported through status/SSE. Application-only router
constructors omit activation. Native `Host::prepare`/`activate` expose the same lifecycle;
existing eager constructors, `--once` and `--stdio` retain initial capture on startup.

`filter` is a case-insensitive path substring. `limit` defaults to 2,000 and is clamped
to 1–2,000. `total` counts all discovered supported source files; `matching` counts matches before
limiting. The view includes world-space tile rectangles, directory labels, and height.
`policy` identifies capture policy, including empty snapshots. `analyzed` counts all
captured files with analysis records; `source_only` counts those without records.
These counts describe the entire snapshot, independently of filtering/overview bounds.
Tile coverage combines all configured analysis contexts. Source outlines retain each
context's identity, coverage, diagnostics, and declarations separately.

Every tile and selected source includes `classification`: `kind` is `assigned` or
`source_only`, `language` names the assigned/requested language (nullable for source-only),
and `provenance` is an `extension` or `explicit_scope` record. Source-only records also
contain a structured `reason`; see [SOURCE_POLICY.md](SOURCE_POLICY.md).
Selected source includes `encoding` (`utf8` or `unsupported`). Grammar recovery,
assignment, and encoding are separate facts. Current outlines carry `mode: "raw_syntax"`
and `configuration_coverage: "unknown"`; a clean grammar parse is not compiler coverage.
Selected and pinned source also include `version`, the immutable document version.
It changes with source, classification, or analysis; unchanged file/version keys allow
clients to reuse projection indexes across selection/filter requests.
Pinned `/api/source` replies include `policy`, `classification`, and `encoding` from
the retained snapshot along with the existing file/path/text fields.

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

Coverage values also include `deferred`: analysis was postponed by input admission,
with no facts and an explicit diagnostic. Captured source remains available.
`View.deferred` and `Measurements.deferred` count these files over the whole snapshot.
See [admission](ADMISSION.md) for retry, persistence and priority rules.

Other coverage values are `syntax` (parsed without grammar recovery), `partial` (recovery
occurred), and `unavailable`. None claims compiler-level resolution or complete build
configuration coverage.

Schema-1 view/source consumers need to upgrade and reload when connecting to this
host version. No downgrade endpoint is provided; browser validation explicitly rejects
the old view schema. Status/SSE consumers retain their existing envelope contract.

Measurements count total indexed `files`, actual read `bytes`, parsed files, in-memory
reuse, persisted cache hits, and cache write warnings. `tree_reuses` counts parsed files
supplied with retained syntax trees; it is distinct from document/persisted reuse. See
[syntax retention](SYNTAX_RETENTION.md). `read_files` counts reopened
source files; `retained` counts unread baseline documents carried by hints. Retained
documents also count as `reused` analysis. `shared_sources` counts reopened inputs
sharing verified byte/index storage with the baseline; their bytes are still read and
hashed. See [captured source](CAPTURED_SOURCE.md). `reconciled` distinguishes a complete pass
from targeted verification, including conservative fallbacks. None treats unchanged
metadata as content proof. `capture_ms` includes input validation, discovery/candidate
inspection, reads, content hashing, and new source/index construction;
`total_ms` includes capture, verification, analysis/cache activity, and snapshot creation.
Transport, debounce, notification delivery, and painting are outside those timings.

`pending` excludes private periodic verification. Additive `recovery` metadata contains
`running`, `completed`, `discarded`, `verification`, `overlay`, and `error`. Verification
and overlay measurements remain separate; normal `measurements` records the final
accepted operation, so an accepted recovery overlay can have `reconciled: false`.
Use `recovery.completed` to track private periodic completion. One-worker serial passes
use normal measurements. See [recovery scheduling](RECOVERY.md) for exact semantics.

In stdio mode, requests are JSON lines `{"id":1,"uri":"/api/view"}`. Responses are
`{"id":1,"status":200,"content_type":"application/json","body":"..."}`. The body
is the HTTP response body as text, including JSON encoding where applicable. Stdio
uses the same router and live application, with no listener required. The unbounded
`/api/events` endpoint returns **400** through diagnostics; use HTTP or the native
subscription interface for streaming.

Source access is fallible over either resident or private-backed data. Bulk view/source
loading is limited to 32 MiB of source; excess returns **413**, while pinned windows
retain complete logical access. Source-work concurrency saturation returns **429**;
backing IO, short reads, integrity/internal range failures return **503**. Typed malformed
HTTP queries remain **400**. Summaries do not load whole source. See
[source reads](SOURCE_READS.md) for bounds and native error types. Source IO/projection
runs in a bounded blocking pool outside the publication lock/async executor.
