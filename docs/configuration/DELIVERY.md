# Confirmed visual-workspace delivery

For contributors implementing editor presentation adapters. Native captured-source handoff
resolves destinations; this module delivers those opaque destinations into a visual view.
It is separate from semantic sharing, originating-editor return and native lifetime.

## Behavior

- A call or file action succeeds only after the browser confirms applied navigation with
  readable requested source. A normal call destination chooser counts as applied; human
  choice is not awaited. Review confirms its selected mode and requested capture, including
  a normal pending-capture state. Reveal preserves the existing viewing mode.
- A newer editor action replaces a waiting action. Manual file, declaration, call, task,
  history or mode navigation cancels pending editor delivery. Reading, scrolling, panning
  and resizing do not. Cancelled/superseded results and successful delivery are silent.
- Prepare captured data and readable source without changing the working set or history.
  Once prepared, recheck original editor buffer, saved bytes and current intent, then
  commit synchronously in the browser after final identity, snapshot and expiry checks.
  Editor verification and remote UI application cannot be an atomic filesystem operation.
- Retain call-trail geometry, reused fragments and source-browser Back/Forward. A legacy
  fragment already created a browser history entry; consume/replace it. A same-page
  message creates its source visit on application, without overwriting the previous visit.
- Use one fixed 60-second browser deadline across routing, preparation, verification and
  confirmation, separately from native capture's 30 seconds. No automatic commit retry.
  Lost confirmation says "could not confirm"; it must not claim nothing opened. Effects
  already applied are retained rather than rolled back over newer reader actions.

## Delivery seam

The editor adapter presents a session with a fixed action (`reveal`, `call`, `file` or
`review`) and a scoped signal/source-verification callback. Source actions carry only
captured opaque identities and a reading line. No source text, absolute file paths,
provider credential, executable or arbitrary navigation URL crosses into the browser.

The shared delivery module owns action validation, routes, readiness, request identity,
preparation tickets and terminal outcomes (`applied`, `refused`, `cancelled`). Each page
supplies its existing mode-specific preparation and synchronous application adapter.
The parent retains one current request across deliberate mode changes; native pages
route themselves using fixed same-origin destinations and an opaque transition ticket.
An already dispatched transition belongs to the document switch, independently of the
current request. A newer action waits for that switch and then prepares in its destination.
Manual navigation cancels both an active request and an unresumed continuation; a later
preparation cannot revive the cancelled transition.
There is no server broker or persisted request queue. Standalone legacy fragments enter
through the same browser delivery controller; automatic external action routing is not
introduced.

Review preparation waits until its requested comparison ticket is represented by the
page. When the local status stream lags, fresh server status distinguishes a pending
ticket from a replaced comparison. Cancellation removes that wait; application rechecks
the represented ticket.

## Trust and document identity

Keep one native iframe and the existing native frame-ancestor policy. A native document
announces a nonsecret identity; the outer shell accepts only the exact iframe window and
native loopback origin. Bind a transferred MessagePort to that document using a challenge
and echo. A late hello cannot replace a newer channel. The existing private VS Code shell
nonce stays between the editor host and outer shell and is never exposed to native pages.

Every preparation and commit binds request, document and ticket identities. Only an
expected opaque mode transition continues preparation in a new document. Manual reload,
unexpected document replacement or tab closure cancels pending work; a dispatched commit
is never replayed on reload. Old replies cannot complete or fail another delivery.

## Verification

Test the delivery interface over an in-memory channel, including delayed/hidden readiness,
source changes after preparation, stale tickets/replies, manual/newer cancellation,
repeated payloads versus replayed IDs, authorized mode transfers, timeout and lost ack.
Test explicit file refusal and call lookup/source-load failure. Preserve real call-layout,
file-history and installed-editor checks. Treat mutation evidence and integration results
as bounded evidence rather than proof.
