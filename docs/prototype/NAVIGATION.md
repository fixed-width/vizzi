# Investigation navigation

This contract adds session-local return navigation to the linked workspace. It follows
the [baseline learning replay](../../experiments/learning-workflow/RESULTS.md).

## Behaviour

Back/Forward controls and browser Back/Forward revisit accepted file/declaration
selections. Fresh file selection still opens the file normally; selecting a declaration
still reveals it. Selecting an already selected location does not append a duplicate.
Filtering, scrolling, background publication and failed requests do not append visits.
Filter changes update the current visit.
After a failed return, an accepted explicit action branches from the requested history
position, including a filter action that re-establishes the displayed location.

Each visit records opaque file/declaration identities, document version, observed snapshot,
path, filter, inspector position and map camera. Positions include horizontal/fragment
and outline-cursor state. No source bodies, facts or snapshot leases are retained.

Returns query the latest published snapshot. When file, declaration and document version
still match, restore the recorded inspector position and filter. Restore the camera only
when the whole snapshot matches; otherwise reframe the current overview.
Coordinates clamp to the current pane dimensions.
When source changed, show a notice and reveal the current matching declaration, or the
current file start if the declaration is gone. Never show an old label over new bytes.
An unavailable file must not silently fall back to a different file: keep the current
coherent view and explain the failed return.

History tracks the requested browser-history position separately from the displayed
location. A failed return remains traversable/retryable; Back/Forward can continue from
that requested position while the last coherent source remains visible. Fresh navigation
from there discards the forward branch. Obsolete responses cannot restore positions,
create visits or overwrite a newer selection.

At most 50 visits are retained, as metadata only. New navigation after Back truncates
the forward branch. Browser history contains only a session marker and visit key; the
browser manages its own stack. Reaching a discarded marker resets the application trail
to the current view and explains expiry. Reload starts a new trail. Ordinary browser Back
at the beginning can leave the application; the page does not trap external navigation.

Controls have accessible names, disabled states at known ends and an independent live
notice. Traversal does not steal focus from the navigation controls. Lookup/semantic
relationships, persistent investigations, old-source browsing and Git comparison are
separate capabilities.

## Implementation and validation

The browser navigation module owns bounded records, branching, browser markers and
request intent. A real History adapter and a deterministic test adapter share that
interface. The request controller carries intent through retries, permits an explicit
pre-publication rejection and rolls back return queries on failure. The inspector exposes
capture/restore of its viewport without owning the trail.

The host adds only the navigation JavaScript asset. Existing snapshot/window contracts
and retention remain unchanged. Return notices are distinct from indexing/connection errors.

TDD covers accepted visits, deduplication, filter replacement, bounds, branches, browser
markers, changed/missing targets, failures, retries and superseded work. Real-browser
checks cover both toolbar and native history, viewport/selection restoration, changed
source and coherent failure. Mutation-test the navigation/controller logic and relevant
inspector restoration, then repeat the learning workflow. Functional evidence remains
separate from unfamiliar-developer usability claims.
