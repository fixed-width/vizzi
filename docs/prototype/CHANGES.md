# Workspace change-notification contract

This reference defines the native subscription and its HTTP adapter. It replaces the
browser's periodic status polling while leaving source and analysis in immutable
snapshots. The portable core has no transport or subscription dependency.

## Delivery model

The native application retains one latest status notification. Each subscriber first
receives the current state, then waits asynchronously for changes. Slow subscribers
can skip intermediate states; they do not block publication or grow an event queue.
The subscription closes explicitly on host shutdown. Static captured queries remain
independent of the subscription.

Each notification has `schema: 1`, an opaque per-application `stream` identity, a
monotonic `revision`, and the full existing `status` record. `revision` is encoded as
an unsigned decimal string to preserve integer precision in browser clients. It counts
observable status changes, including pending/error transitions; snapshot `generation`
still counts only distinct published observations. Identical status and superseded
work emit nothing. Stream identities change across host restarts.

## HTTP adapter

`GET /api/events` returns UTF-8 `text/event-stream`. Each `workspace` event has an
`id` of `<stream>:<revision>` and JSON notification data. Keep-alive comments carry no
state. Every connection starts with current state; `Last-Event-ID` does not request
replay. Consumers reconcile from the latest snapshot instead of replaying deltas.
The finite stdio diagnostic transport rejects this streaming endpoint explicitly.

`GET /api/view` accepts an optional `snapshot` parameter alongside the existing
selection/filter parameters. A pinned view uses that retained observation throughout;
an expired/unknown observation returns 409. Unpinned view and status requests remain
available for diagnostic clients.

## Browser consumer

The browser subscribes once and fetches views on snapshot or selection changes. It
validates versions, ignores duplicate/older revisions within a connection, and rejects
callbacks from replaced connections. Notifications arriving during a view request are
coalesced; an obsolete response cannot reconcile selection or replace current source.
The next request targets the latest advertised snapshot.

On disconnection or a failed view request, the browser retains its last coherent view
and reconnects with 100–4,000 ms bounded backoff. A fresh connection immediately resynchronises
status, including after a host restart, without a polling timer. Failed selections
retry even when the snapshot identity is unchanged. Source/outline data still come
from a single pinned view response. Future editor adapters can consume the native
subscription without implementing SSE.

The native channel uses [Tokio watch](https://docs.rs/tokio/latest/tokio/sync/watch/index.html),
cloning its current immutable value before awaiting again. The
[SSE adapter](https://docs.rs/axum/latest/axum/response/sse/index.html) uses the
[standard EventSource format](https://html.spec.whatwg.org/multipage/server-sent-events.html).
The browser closes failed EventSource objects before creating replacements, so its
own backoff and connection tokens govern resynchronisation.
