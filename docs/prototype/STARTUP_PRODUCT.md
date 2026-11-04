# Task-driven HTTP startup: behaviour contract

The [focused review measurements](../../experiments/review-workflow/SCALE_RESULTS.md)
show a 16–19-second one-worker PyTorch host wait while 5,079 live files are indexed,
before a 109-path review can be requested. This slice makes the HTTP host available
before live indexing and starts that work only when exploration requests it.

## Readiness and entry points

Opening `/review` serves the interface/status/subscription and permits explicit scoped
comparison without first capturing the live workspace. A review-only session does not
start live discovery, watchers, periodic reconciliation or idle source maintenance.
Review uses the same source policy, logical workspace, analysis cache and aggregate
parser-worker budget. No separate parser pool or alternate comparison semantics appear.

Opening exploration (`/`) automatically requests live indexing through an explicit
JSON activation operation. Repeated requests while indexing or already ready do not
restart it. Before a first observation, failed activation/indexing is actionable and
can be retried. After activation, normal watches, verification and maintenance continue
for the host lifetime, including after leaving the exploration page.

Live status distinguishes `not_started`, `initializing`, `ready` and `failed`.
An unstarted/incomplete index has no snapshot, not an invented empty repository.
A completed empty observation is ready. Refresh/failure with a previous coherent
observation remains ready for reads, with existing pending/error fields describing
the outstanding attempt. Status/subscription/query reads do not themselves activate.

The exploration interface explains initial indexing/failure and offers Start/Retry
when no observation is available. Its existing linked selection and history policies
continue once source is published. Review status/identity remains independent.

## Limits and compatibility

Explicitly opening exploration can contend with review under the shared worker budget;
this slice adds no preemption or priority scheduler. A status request stays metadata
only while analysis runs. Individual IO/parser cancellation remains cooperative.

Existing eager `Host::start`, one-shot and finite stdio defaults retain their initial
capture behaviour. Native/future editor hosts may use the deferred host interface and
the same activation operation. No source-control or language assumption moves into
portable models. No analysed-project build, script, fetch or tool probe is introduced.

Shutdown before activation performs no live capture. Shutdown during initialization
cancels/joins owned work and closes subscriptions. Existing held snapshots and review
leases keep their original read semantics.

## Acceptance

Use controlled adapters to prove deferred construction/metadata reads do not parse,
scoped review completes before any live publication, activation is idempotent, initial
failure/retry is explicit, and shutdown works in dormant/initializing states. Verify
watch-registration races and existing eager/live behaviour through regression tests.
HTTP/stdio and real Chromium checks cover activation content type, readiness, exploration
entry and independent review entry. Mutations use one lower-priority job.

Repeat the optimized scoped measurements against the existing pins/working-tree method.
Record host availability, first comparison and whether the live index stayed dormant.
Keep new first-comparison cost separate: it loses the earlier global working-tree warmup.
Finite samples do not declare startup deadlines or baseline-laptop qualification.
