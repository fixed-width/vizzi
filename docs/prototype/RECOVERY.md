# Periodic recovery scheduling reference

This contract describes how the native host keeps missed-event recovery separate from
ordinary dirty-file updates. It complements [incremental capture](INCREMENTAL_CAPTURE.md)
and [publication ordering](CHANGES.md).

## Foreground and periodic work

With two or more configured parser workers, the host allows one private periodic full
verification alongside one foreground attempt. Background verification uses one parser
worker; foreground has at most the remaining configured workers while that task is
outstanding. Languages share this budget. Parsers remain worker-local. With one configured
worker, periodic verification uses the existing serial foreground path instead.

Periodic work does not request a foreground publication ticket or set `pending`. Its
starting observation and eventual candidate are immutable handles. Ordinary path hints
still invalidate obsolete foreground work and can publish while verification continues.
Directory changes, ambiguous/rescan events, explicit full refreshes, watcher failures,
and hint/journal overflow cancel the private candidate and require full discovery.
An unrepaired foreground failure also requires the next periodic attempt to reconcile
through the foreground path.

The recovery journal contains the union of affected paths since that verification
started, including paths whose foreground work has already published. It has the same
4,096-path limit as the ordinary hint set. Overflow requires discovery instead of
silently forgetting acknowledged paths.

## Verified publication

After full verification finishes, the host waits for foreground hints to settle. It
then reopens and hashes journal paths against the private verified candidate. It does
not copy those paths from an earlier foreground observation: a later missed edit could
otherwise be replaced with stale bytes. This overlay uses the normal context, source
policy, capture-plan, cache-validation, and conservative-fallback rules.

A new event during the overlay invalidates its ticket. The full private candidate and
journal survive for retry; neither is acknowledged by stale or failed work. Only an
accepted latest-ticket publication finishes recovery. Hard discovery invalidation
cancels the candidate through a separate revision token. New directory watch registration
still triggers a mandatory full pass to close the registration gap.

Published snapshots and retained source pins stay immutable. An observation is not an
atomic filesystem transaction. Paths outside the journal may change after verification
and require another event or periodic pass. Continuous writes still need a finite
latest-ticket window to publish; this is not a starvation-freedom or hard latency guarantee.

## Status fields

`status.pending` describes foreground work. Additive `status.recovery` metadata appears
in status responses and normal latest-state notifications:

| Field | Meaning |
| --- | --- |
| `running` | A private verification or its final overlay is outstanding |
| `completed` | Accepted private periodic passes, including unchanged observations |
| `discarded` | Cancelled, failed, or panicked private candidates |
| `verification` | Full verification measurements from the latest accepted private pass, initially null |
| `overlay` | That pass's final fresh path verification measurements, or null when unnecessary |
| `error` | Latest maintenance failure, separate from foreground `error`; cleared by the next start |

The normal `measurements` field still describes one accepted engine operation. When a
recovery needs an overlay, this field describes the overlay, whose `reconciled` may be
false. Use `recovery.completed` to observe private periodic completion. Do not add
verification/overlay counts or wall times and present them as a single nonoverlapping
capture. A serial one-worker pass uses normal measurements and does not increment these
private recovery counters.

## Native integration

`Host::start` uses native observation and a two-second reconciliation interval.
`Host::start_with_options` accepts `HostOptions { watch, reconcile_interval, idle_maintenance }` for native
embedders and controlled probes. Disabling watches deliberately selects polling. The
interval must be positive and representable; it runs from accepted full verification
to the next periodic request. Blocking filesystem operations can extend that delay.

`Host::request(CaptureScope)` submits workspace-relative hints or full discovery through
the same bounded queue used by native events. `Application::request()` remains a lower-level
full request; a missing hint association conservatively requires discovery. Neither
operation is an HTTP write endpoint. These interfaces do not depend on Git or an editor.

Shutdown cancels both classes of work and joins the private worker. Cooperative parser
cancellation and capture checks preserve existing limits: blocking I/O, cleanup,
snapshot construction, and arbitrary adapter code have no hard interruption deadline.
Additional concurrent input/parser buffers are outside the opt-in syntax retention quota;
there is no new whole-process memory limit.

## Idle source maintenance

When source spooling supplies maintenance work, the host admits one cleanup worker after
250 ms without foreground/recovery activity. Foreground/recovery work resets the quiet
interval. An in-memory provider hint reports queued retirements or charged tails; the
coordinator never polls source statistics or filesystem metadata. A 100 ms wake ceiling
notices external pin release even without another hint. Hosts without a provider retain
the existing wait schedule and spawn no cleanup worker.

The coordinator joins only finished workers. Already-running filesystem cleanup is not
cancelled by a new edit; the two-file spool permits capture elsewhere when aggregate
headroom exists. Cleanup runs outside publication/coordinator locks. Failures and panics
retry after at least one second, subject to the quiet rule. Success clears the last error.
Shutdown stops admission and joins the outstanding task; blocking IO has no hard deadline.

`HostOptions::idle_maintenance` defaults true. Set it false for caller-managed cleanup or
controlled probes. One-shot/non-host callers retain explicit `SourceSpool::reclaim`.
The native `Host::maintenance_status()` getter returns metadata without backing IO:

| Field | Meaning |
| --- | --- |
| `attempts` | Admitted attempts, including failed thread creation |
| `running` | A worker remains outstanding until coordinator collection |
| `completed` | Successful operations, including metadata-only retirement batches |
| `failed` | IO, worker panic or thread creation failures |
| `error` | Last failure, retained until successful cleanup |

This status is separate from `Application::Status` and does not publish a new snapshot,
overwrite foreground errors or alter recovery counters. Browser/HTTP schemas are unchanged.
Native embedders can expose these diagnostics through their own integration. See the
[design](../plans/idle-maintenance.md) and [qualification](../../experiments/idle-maintenance/RESULTS.md).
