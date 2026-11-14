# Captured-source handoff interface

For contributors implementing editor adapters. A captured-source handoff opens the
Call workspace or Source browser using an editor's original saved source. The native
module owns capture readiness, source matching and destination resolution; the editor
adapter owns its buffers, selection and presentation.

## Behaviour

- Capture the selected resource, saved UTF-8 text and position before changing focus.
  Refuse dirty or disk-divergent buffers. Never save buffers automatically.
- Wait at most 30 seconds for the original saved source to match a complete capture.
  A newer request in the same editor window silently supersedes its predecessor.
  Independent editor windows do not supersede each other.
- An eligible snapshot has no pending foreground capture. Private recovery does not
  block a handoff to otherwise verified source.
- Cancellation retires the handoff, leaving shared source-workspace capture running.
  After the target view becomes ready, the editor checks its original buffer, saved
  bytes and latest request immediately before presentation. Buffer checks bracket
  asynchronous saved-source reads.
- Native capture waiting holds neither a structural lock nor a compute permit.
  Blocking source/projection work is separately bounded; obsolete computation cannot
  publish a destination. Finite exchanges keep sequential stdio dispatch responsive.
- An idle stale capture receives one scoped refresh demand. Existing foreground work
  is joined first. Repeated checks never renew the deadline or restart capture.
- Only captured source admitted by the source workspace is addressable. Native source
  identity and positions are checked before returning opaque destination identities.

## Window scope

`POST /api/editor/handoff/window` takes `{"root":"/canonical/source/root"}` and returns
`{"schema":1,"scope":"<opaque token>"}`. The root must equal the native host's root.
The scope orders handoffs only; it does not own an editor, process or provider connection.
Scopes expire after five idle minutes, with at most 64 admitted scopes per host. An
expired scope is refused and cannot be silently recreated by an old request.

## Start or continue

`POST /api/editor/handoff` starts a request:

```json
{"scope":"<token>","sequence":1,"action":{"kind":"start","path":"src/main.py","text":"<original saved source>","target":{"kind":"call","position":{"line":2,"character":11}}}}
```

For source browsing, use `{"kind":"file","line":3}`. Call positions are zero-based
UTF-16; file lines are one-based. Source is limited to 4 MiB and request bodies to
16 MiB. Paths are normalised, source-root-relative and limited to 4096 UTF-8 bytes.
Sequences are increasing positive
integers no larger than JavaScript's maximum safe integer.

The native module drops uploaded text after deriving its content identity and validating
the position. A waiting request retains only bounded identity/target/deadline metadata.
Repeat the same sequence and input to replay the same operation without renewing it.
Older sequences are superseded; reusing a sequence with different input is refused.

```json
{"schema":1,"sequence":1,"state":"pending","handle":"<opaque token>","retry_after_ms":100}
```

Continue with `{"scope":"<token>","sequence":1,"action":{"kind":"continue","handle":"<handle>"}}`.
The module returns a terminal result immediately when available or waits briefly for
one. Each exchange is finite; HTTP and stdio use the same native module.

```json
{"schema":1,"sequence":1,"state":"ready","destination":{"kind":"call","snapshot":"<id>","call":"<id>"}}
```

File destinations carry `kind`, `snapshot`, `file`, `version` and `line`. Native IDs
cross into the browser; source text, absolute paths and provider credentials do not.

Terminal states are `ready`, `refused`, `superseded`, `cancelled` and `expired`.
Refusals contain a bounded `code` and `message`. Superseded and deliberately cancelled
results are silent. Active refusals and expiry remain actionable errors.

## Cancel

`POST /api/editor/handoff/cancel` takes `{"scope":"<token>","sequence":1}`. Cancellation
is idempotent. Cancelling an older sequence cannot affect a newer handoff; cancellation
that arrives before its start prevents that sequence from starting later.

## Verification

The finite interface preserves sequential stdio request handling. A single awaited
30-second request would prevent that transport from reading cancellation or a newer
intent. Native ordering already needs window-local state, so compact continuation
records keep the deadline and original source identity together without signed
continuation payloads or retained copies of uploaded text. Editor-side intent checks
remain necessary because a newer user action can exist before its request reaches
the native host.

Test readiness, source mismatch, refresh coalescing, fixed deadlines, independent scopes,
out-of-order requests, stale cancellation, publication races and resource bounds through
the native module. Keep editor-adapter tests for dirty buffers, URI aliases, selected
tabs, cursor capture and late-result suppression. Retain real HTTP/stdio and installed
editor checks. The shared-analysis [connection protocol](BRIDGE.md) remains separate.
