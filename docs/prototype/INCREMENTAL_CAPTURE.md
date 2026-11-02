# Verified capture from dirty-path hints

This reference defines incremental capture for the directory source adapter. Hints
select files to verify; filesystem metadata is never proof that contents are unchanged.
The portable snapshot model remains independent of filesystem events and source control.

## Capture and reuse

Initial indexing, an invalid baseline/context, ambiguous events, directory changes,
watcher errors/rescan flags, and periodic recovery use full reconciliation. Ordinary
file writes and named rename sides/pairs can use workspace-relative path hints. Hinted
supported source files are reopened and content-hashed, including same-size and same-mtime edits.
Capture-policy mismatch also requires reconciliation, including an empty baseline.
The same policy selects analyzed and source-only candidates and prunes excluded paths.
Deleted or symlink-replaced files are removed. Ancestor changes and directory targets
fall back to full discovery. New source parents require discovery and watch registration.

Unhinted documents retain their captured bytes and facts from the last published
snapshot. They are not claimed to have been reread. The next full reconciliation
recovers missed events. Captures are observations, not atomic filesystem transactions.
Existing symlinks are excluded; source access retains the adapter's current concurrent
filesystem-mutation limitations.

## Scheduling and acknowledgement

Dirty paths coalesce in a bounded set; overflow promotes the request to reconciliation.
Every relevant event invalidates older work even if its path is already pending. Taking
a plan does not consume its hints. Only an accepted latest publication acknowledges
them; cancelled, superseded, and failed attempts leave them pending for the next job.
An explicit refresh with no associated hints also reconciles.

The coordinator checks a reconciliation deadline even during repeated file events.
Completing a full pass resets that deadline. New watch registrations trigger an
additional full pass to close the discovery/registration gap. Existing registrations
are refreshed during reconciliation so replaced directories can be observed again.

## Measurements

Status measurements distinguish total indexed files, actual files/bytes read, unread
retained documents, reused analysis, and whether capture reconciled.
`analyzed` and `source_only` additionally distinguish files with/without analysis
records; unavailable analysis still counts as an analysis record. A path-scoped
attempt can legitimately report reconciliation after a conservative fallback.
Incremental results must agree with a clean capture when the supplied hints cover the
changes. Silent edits outside the hints remain visible only after reconciliation.
