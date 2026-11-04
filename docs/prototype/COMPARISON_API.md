# Local comparison API reference

The normal native HTTP host and finite stdio diagnostic now expose review application
operations. They share the same handlers and the configured workspace/analysis engine.
Review status, tokens and source are independent of the live `/api/status` snapshot.
The [ownership](REVIEW_OWNERSHIP.md) and [analysis](REVIEW_ANALYSIS.md) contracts still apply.
Wire schema is **1**. Identities are opaque strings and responses disable caching.

## Lifecycle and notifications

| Request | Input | Result |
| --- | --- | --- |
| `POST /api/comparisons` | JSON `provider`, `revision`, optional `scope` | **202**, `{schema:1,ticket}`. |
| `GET /api/comparisons/status` | None | Latest complete review status. |
| `GET /api/comparisons/events` | None | Latest-state SSE `comparison` events. |
| `GET /api/comparisons/{token}` | Current token | Immutable pair labels. |
| `POST /api/comparisons/{token}/refresh` | JSON `{}` | **202**, new ticket for the same pinned baseline/scope. |
| `DELETE /api/comparisons/{token}` | JSON `{}` | **204**; retire lookup and cancel pending intent. |

Create requires `provider:"git"` and an explicit revision containing 1–256 bytes
without NUL. Scope is a literal normalised relative path, at most 4,096 bytes; absent
or `.` means root. Dot components, absolute paths, empty components and traversal fail.
Capture roots cannot be supplied by a request. JSON denies unknown lifecycle fields.
Mutations require JSON content type; no CORS/preflight grant is added. The lifecycle
body limit is 8 KiB.

Acceptance creates intent, not completed analysis. A single worker processes the latest
queued request; a new request supersedes older intent. Capture/analysis run outside
publication metadata locks. Failure of the latest request records an error and keeps
the last active pair; obsolete failure cannot overwrite a newer request. Pair/source
admission can refuse a private capture without replacing active source.

Status contains `schema`, latest `ticket`, `pending`, `active` pair labels and optional
`error:{code,message}`. Messages are bounded to 512 UTF-8 bytes. Labels include token,
portable comparison ID, provider, requested revision, resolved object, literal scope,
and working capture start/completion in Unix milliseconds. These describe an observed
interval, not an atomic filesystem transaction or a promise that disk is unchanged.

Events contain `schema`, fresh `stream`, decimal-string `revision` and full `status`.
SSE id is `<stream>:<revision>`; first delivery is current state and slow subscribers
may skip intermediate states. Reconnect resynchronises rather than replaying events.
Review never reuses a live workspace notification's snapshot identity as a pair token.
Shutdown closes current/future subscriptions and cancels capture. An existing held
query can finish with its original labels; the future UI must reject obsolete responses.

## Immutable query projections

| Request | Required anchors | Optional projection parameters |
| --- | --- | --- |
| `GET /api/comparisons/{token}/files` | Current token | `offset`, `count`, `filter`, `unchanged` |
| `GET /api/comparisons/{token}/window` | `file`, `side`, `version` | Existing [window](WINDOWS.md) parameters |
| `GET /api/comparisons/{token}/diff` | `file`; versions for each retained document side | `base_version`, `working_version`, `hunk_start/count`, `edit_start/count` |

An opaque file identity is resolved through the held pair's prebuilt inventory index.
It is not a filesystem path. Side is exactly `base` or `working`. A missing side differs
from a retained empty document. Diff requests omit that side's version only when no
retained document exists; omitting a present version is a conflict, not a request to
use whatever happens to be current. Queries never read live workspace source as a fallback.

File results contain token/comparison labels, `files`, matched `total`, actual `offset`,
`next`, `remaining` and full-scope `totals`. Count defaults to 200 and clamps to 1–200.
Unchanged rows are omitted by default; `unchanged=true` exposes related unchanged
context. Filter is a case-insensitive literal display-path substring capped at 256
bytes. It changes the projection, not inventory scope or the full-scope counts.
Offsets stay stable within one immutable pair.

Each row gives `file`, escaped `path`, `path_truncated`, change kind, side versions and
base/working evidence. Display paths cap at 512 UTF-8 bytes. Entry evidence distinguishes
unobserved presence from present type/content, with kind, executable bit, verified
length/content ID, unavailable reason and bounded literal target/reference display.
Literal values are escaped and cap at 512 bytes with a truncation flag; targets are
never followed. Full totals include added/removed/modified/unchanged/unassessed counts
and overlapping unavailable-content/presence count. Binary identity can be verified
even when textual inspection is unavailable.

Window responses include token, comparison ID, explicit side, existing bounded window,
bounded classification, context descriptors and hidden-context count. Source stays
within 96 rows/32 KiB and outline within 200 entries. At most four contexts are emitted;
language labels cap at 32 bytes, descriptions at 512, classification provenance at 256.
Each context includes coverage, declaration total, at most three 512-byte diagnostics,
hidden diagnostic count and truncation indicators. No unbounded fact list is copied.

Diffs use the [fixed default input/work limits](TEXT_DIFF.md). Response pages contain
at most 200 hunks and 200 edits independently, plus total/remaining counts and next
offsets. Hunk edit ranges refer to global edit indices; a large hunk may require more
edit pages. Ranges are exact zero-based exclusive line/byte ranges. Source text is
loaded separately through pinned windows. The current prototype recomputes a file's
bounded diff for each page; it does not claim a cached diff-index latency.

Query projection and source work run outside the async executor/publication lock in
at most two blocking jobs, with at most one diff computation. Excess work receives
**429**, without an unbounded queue. Window/context panes can load concurrently.

## Errors and finite diagnostics

| Status | Meaning |
| --- | --- |
| **400** | Malformed typed query or invalid native request/filter. |
| **404** | File or requested source side is absent/unretained. |
| **409** | Token expired/retired/closed or document version differs. |
| **413** | Diff/source allowance or lifecycle body limit exceeded. |
| **415/422** | Framework JSON content-type/body rejection, or unavailable binary/unsupported textual diff. |
| **429** | Query or pair admission is occupied. Capture refusal after acceptance is reported in status. |
| **503** | Native source/worker/internal failure or closed application. |

Native error bodies are `{schema:1,code,message}`. Typed extractor rejections retain
Axum's rejection body. Unsupported text and exhausted diff work never return successful
empty hunks as if a file were unchanged. A failed/refused refresh leaves active labels
and ownership coherent.

Stdio requests keep `id`/`uri` and add optional `method` (default `GET`) and JSON `body`:

```json
{"id":1,"method":"POST","uri":"/api/comparisons","body":{"provider":"git","revision":"HEAD"}}
```

Responses use the existing diagnostic envelope. Both live/review streaming event
paths return **400** through finite diagnostics. Native tests consume the independent
subscription interface; browser clients will use SSE. The
[transport validation report](../../experiments/review-workflow/TRANSPORT_RESULTS.md)
records tested behaviour and the frozen-fixture replay. Linked review UI is next.
