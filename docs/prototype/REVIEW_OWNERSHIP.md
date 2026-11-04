# Review ownership and admission reference

`vizzi::review` admits capture, owns one active comparison and pins in-flight queries.
It wraps [native acquisition](REVIEW_ACQUISITION.md) and the portable comparison without
adding a transport, analysis pool or browser behaviour. This implements the ownership
part of the [comparison contract](COMPARISON_TECH.md).

## Request and publication

| Operation | Contract |
| --- | --- |
| `ReviewStore::open(source_bytes)` | Creates a fresh token namespace and explicit logical source allowance. A host can use the provisional 2-GiB default from capture configuration; zero permits empty/metadata-only inputs. |
| `request` | Issues newer intent, invalidating older candidate tickets. It retains the active pair. Sequence exhaustion fails without wrapping. |
| `begin(ticket, refresh)` | Admits one private candidate for the latest ticket. `None` creates a new pair; a current token reuses that pair's exact baseline owner and policy. |
| `capture_base` / `capture_working` | Reserve source capacity before input capture. Working capture uses the held base policy; refresh cannot replace the baseline. |
| `publish` | Validates compatible sides and atomically replaces the active pair only for the latest live ticket. No partial candidate is published. |
| `query(token)` | Pins exactly the active pair. New lookups for retired, closed, foreign or restarted-store tokens fail explicitly. |
| `close(token)` | Retires active ownership and invalidates outstanding intent. A wrong token cannot close the current pair. |

Capture and comparison construction run outside the short publication lock. Accepted
replacement extracts the old owner under that lock and destroys it afterwards. Capture
failure, cancellation, stale tickets and invalid stages preserve the active pair.
Another request can supersede an ongoing job, but its successor must wait for the old
candidate to release admission. There is no hidden queue or unbounded history registry.
Abandoning a candidate clears only its own in-flight marker.

Every publication gets a fresh lifetime token, including an identical-content refresh.
The portable comparison ID can stay equal while the token changes. A new store has a
fresh random namespace. Tokens do not revive after close, replacement, restart or
sequence exhaustion. They name ownership; they are not SCM or document identities.

## Query lifetime and two pair slots

There are exactly two total pair slots across active pairs, private candidates and
retired pairs held by queries. Multiple leases for the same pair share one slot.
An active pair plus a held retired pair therefore refuses a third candidate. Closing
or replacing a pair removes it from lookup immediately; its slot is released only
after its last in-flight lease is dropped.

For example, a source read starts against comparison A. Refresh publishes B while
that read is unfinished. The existing read can finish from A's captured source,
keeping its ranges and labels consistent. New requests for A fail. When comparison UI
integration is added, it must discard A's late response once B is current; this module
does not decide what is displayed. A lease is an in-flight request owner, not browser
history or permission to silently switch source versions.

`ReviewLease` exposes borrowed inventory/notices and owned, bounded source windows or
text-diff metadata. It does not expose `Arc<Snapshot>`, documents or captured source
owners that callers could detach from the lease. Returned window strings and diff
metadata can outlive it; they do not keep the original source bodies charged/alive.
Existing window limits remain 96 rows/32 KiB and the diff's explicit work/input bounds.
Transport paging, request limits and stale-response handling remain host work.

Dropping the store cancels its request namespace and retires active ownership. Already
acquired query leases remain coherent until released. Read-only `ReviewAccounting`
handles can outlive the store without retaining its pairs. Metadata, slots and byte
guards have no ownership cycle.

## Source allowance

Each input capture first reserves the lesser of its requested byte allowance and the
currently available review capacity. Native capture receives that bound before it
reads/retains content. A file that does not fit fails explicitly. A reservation cannot
consume capacity held by the active pair, another candidate input or a retired query.

On successful capture, unused reservation is released and actual retained logical
source bytes stay charged to the immutable input owner. Regular source storage is
counted by actual `CapturedSource` owner identity, with duplicate owner references
counted once within the input. Literal target/reference buffers are included. Digests,
discarded unrepresentable bodies and unavailable source do not become retained bodies.
Fresh captures with equal hashes still occupy distinct storage and are charged separately.

Refresh shares the admitted baseline input owner across both pairs; that owner is
charged once until all references disappear. Current acquisition creates fresh source
storage per read. There is no interface for adopting arbitrary external snapshots or
deduplicating separate captures by hash. The [analysis bridge](REVIEW_ANALYSIS.md)
preserves the charged input owner while rebinding facts; it validates original source
owner identity rather than creating an untracked source owner or dropping the guard.

Accounting reports `capacity_bytes`, `charged_bytes` (retained plus outstanding
reservations), `retained_bytes` and `pair_slots`. Input/pair destruction frees source
owners before releasing their guards. Failure, cancellation and unwinding release
private reservations. Snapshots/facts, line indices, metadata, transient buffers,
parser/diff heap and the live workspace remain outside this logical source allowance;
it is not a process-memory cap.

The [validation report](../../experiments/review-workflow/OWNERSHIP_RESULTS.md) records
tests, mutation scope and the frozen fixtures through this admitted path. Shared-engine
analysis is now delivered; host/browser comparison operations remain next.
