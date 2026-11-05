# Contextual local-change review: technical contract

Status: portable inventory comparison, side-specific source binding and bounded textual
hunks are implemented, with source-only native acquisition and bounded review ownership.
Admitted analysis now shares the live engine, with a native review application and
[comparison HTTP/stdio transport](COMPARISON_API.md). The initial
[browser review implementation](REVIEW_INTERFACE.md) passes the
[guided rendered tasks](../../experiments/review-workflow/WALKTHROUGH_RESULTS.md).
Scale/usability qualification remains open. This reference supports the
[behaviour contract](COMPARISON_PRODUCT.md). Exact wire fields can be refined with
implementation while preserving these invariants.

## Delivered portable inventory interface

`vizzi_core::comparison` accepts supplied immutable observations without filesystem or
SCM access. Its public interface is exercised by the non-Git, in-memory cases in
`crates/vizzi-core/tests/comparison.rs`.

| Interface | Delivered contract |
| --- | --- |
| `ReviewPath::parse` | Validates lossless relative path bytes; rejects NUL, empty/dot components and absolute paths. `/` is the canonical separator. Backslash and non-UTF-8 names remain distinct inventory paths, without document aliases. |
| `Observation::from_inventory` | Takes an `Arc<Snapshot>`, review `CapturePolicyId` and complete inventory. Sorts paths and rejects duplicates. Every retained document must match a present regular entry's content identity and length. |
| `Comparison::between` | Requires the same logical workspace, review policy, snapshot analysis policy and capture-plan compatibility marker. Retains both observation owners. |
| `Comparison::entries` / `entry` | Lazy sorted union or path lookup, returning before/after evidence and added/removed/modified/unchanged/unassessed status. Raw content and metadata differences are separate. |
| `Comparison::document` | Selects retained source from the requested immutable `Side`; absence of a retained document does not establish absence of an inventory entry. |

The acquisition caller must finish enumeration before constructing an observation.
Omitted paths mean verified absence; `EntryState::Unobserved(reason)` represents unknown
presence, including sparse materialisation. A present path can separately have
`EntryContent::Unavailable(reason)`. Comparing two unknown values never proves equality.
Known presence/absence proves addition/removal even when that entry's content is
unavailable; a known mode/type change proves modification while the content dimension
can remain unknown. Consumers retain these overlapping availability gaps in their counts.

Review policy identifies the subtree, capture/exclusion rules and other acquisition
configuration, separately from the snapshot's analysis policy. Observation identity
hashes the analysed snapshot and sorted inventory/policy using `review-observation-v1`.
Ordered comparison identity includes `inventory-comparison-v1`. These content identities
are not admission/publication tokens and do not implement the lifetime rules below.

The inventory constructor streams metadata into its hash, without serialising source
bodies or copying a complete encoded inventory buffer. It sorts once; comparison
iteration merges both sorted inventories without a second full change list. Retained
source shares existing immutable owners. Source-body admission and bounded host views
remain necessary before exposing review capture to users.

The delivered [textual diff reference](TEXT_DIFF.md) defines exact line/byte ranges,
context grouping, hunk identities, cancellation and charged resource limits.
The [native acquisition reference](REVIEW_ACQUISITION.md) defines the delivered list/read
seam, pinned Git objects, observed working bytes, limits and platform/support gaps.
The [ownership reference](REVIEW_OWNERSHIP.md) defines admitted capture, lifetime tokens,
retired query leases and shared source charging.
The [analysis integration reference](REVIEW_ANALYSIS.md) defines frozen-plan rebinding,
shared parser/cache execution and aggregate foreground/background worker allocation.

## Source observations and the acquisition seam

The portable comparison module consumes two immutable observations in one logical
workspace/path namespace. Each observation pairs an existing analysis `Snapshot` with
a path inventory that records entry kind, mode, content identity/availability and
coverage. `Snapshot` alone is insufficient: it currently covers selected documents,
without symlink, gitlink or mode metadata. Regular inspectable source can reuse
`Document`/`CapturedSource`; documentation without analysis uses source-only
classification. Nonregular entries do not become synthetic text documents.

The native acquisition module supplies a frozen source handle with logical workspace
identity, enumeration and byte reads. Directory, Git revision and in-memory adapters
exercise this seam. Git refs, OIDs, index stages and object access stay in the native
adapter/provenance. The portable module compares verified content and entry metadata;
it does not use Git blob IDs as `ContentId` or require a Git commit to exist.

The directory adapter currently derives `WorkspaceId` from the canonical root. Extract
the minimal identity/list/read interface so a revision stored elsewhere retains that
same logical root. Ordinary representable paths retain existing `FileId` derivation.
Inventory must also retain a lossless opaque path key and escaped display for native
paths that the document model cannot represent, such as non-UTF-8 names. Such entries
are explicit metadata-only/unsupported-path records, never omitted or normalised onto
another path. Native locators are private; query tokens cannot supply arbitrary paths.

Both sides freeze compatible source-selection/analysis configuration. Review inventory
covers the selected Git subtree independently of language policy. Rust/Python and
explicit raw C/C++ assignment rules continue through existing adapters; other text
remains source-only. In particular, LPC files are not assigned to C/C++ by extension.
Analysis reuse remains content/context/implementation bound. Policy mismatch requires
recapture/reanalysis, not rebinding stale facts. Capture/analysis gaps remain separate
from the content-change result.

## Git acquisition semantics

Resolve the selected commit/tree once. Enumerate base paths, index paths and non-ignored
untracked working paths in the selected subtree. Include base paths removed from the
index, and deduplicate paths with multiple index stages. Read actual filesystem bytes
for the working observation; an index entry, stat match, assume-unchanged flag or clean
Git status is not proof of unchanged bytes. Full initial capture/explicit refresh must
verify bytes or reuse an independently verified immutable owner. Git discovery hints
cannot remove this requirement.

The baseline uses raw object bytes and modes; working regular files use raw disk bytes
and observed executable mode. No attributes-based transforms, external diff/textconv,
hooks, project scripts, recursive submodule updates or fetches run. Standard Git
untracked exclusions apply; record the resulting inventory and effective scope policy.
Tracked/base candidates remain candidates even if a new ignore rule matches them.
The semantics follow the distinction between commit/working and cached comparisons in
[Git diff](https://git-scm.com/docs/git-diff); untracked enumeration is separate, with
[standard exclusions](https://git-scm.com/docs/git-ls-files).

| Observation | Portable result |
| --- | --- |
| Present on one side, verified absent on the other | Added or removed; absent differs from a zero-byte file. |
| Same path, different verified bytes | Content modified, independent of declaration matching. |
| Same bytes, different mode or entry kind | Metadata/type change, even without a textual hunk. |
| Symlink | Hash/compare the literal target, never traverse it. |
| Gitlink | Native adapter reports pointer identity as opaque provenance; compare pointer values, report dirty/unavailable nested state separately; do not recurse. |
| Binary/unsupported encoding | Verified byte identity and size; explicit unavailable text diff. NUL-text classification is separate from UTF-8 validity. |
| Explicit exclusion, unreadable content, sparse absent working path | Present/unavailable or unassessed with reason; cannot infer equality or deletion. |
| Unmerged index | Report provenance; working source is the actual file, including conflict markers. No stage is silently chosen as after source. |
| Unrepresentable native path or unsupported special file | Retain metadata and reason; never block reading a FIFO or collapse its name. |

No rename inference: removed/added paths remain separate. Enumerating a scope must
either complete or fail; per-entry unavailable content may remain in a complete
inventory. Distinguish enumeration coverage, verified-change counts and unassessed
counts. Exclusion from byte capture does not assert that Git itself never inspected
the path. Sparse-checkout absence is not a user deletion; unsupported detection must
fail explicitly rather than invent removals.

Capture is an observation, not an atomic filesystem transaction. A changing read must
retry boundedly or record unavailable content; old labelled source can never be
satisfied by rereading the mutable file. Enumeration/read errors and capture times are
retained. Cancellation and stale publication preserve the previous pair.

## Comparison, source and anchors

An observation identity includes workspace, inventory (including unavailable states),
capture policy and analysed snapshot. A comparison identity binds ordered base/working
observations and comparison-policy version. A publication token additionally identifies
the admitted comparison instance, so identical refreshed contents cannot revive closed
anchors. Native baseline provenance and capture times are metadata, not substitutes
for content identity. A baseline ref move never retargets the held base.

Every query anchor contains comparison token, side, path/file identity and, for source
ranges or declarations, document version. Hunk identities are scoped to the ordered
pair and diff policy. Validate these together before serving bounded views. Unchanged
context comes from the selected side of this pair, not the live workspace publication.
Side changes are explicit. Unique lexical keys may help align declaration context;
duplicate keys remain unmatched. File-level edits need no declaration association.

Text diffs operate on exact captured line bytes, retaining CRLF/final-newline markers.
Hunks expose ranges on both sides, including zero-width insertion/deletion anchors.
Independent tests reconstruct after bytes from base plus edits and check expected
coordinates; the implementation must not define its own oracle. Unsupported/resource
limited diff is distinct from a verified unchanged file. There is no semantic matching.

## Admission and lifetime

Start with one active comparison and at most one private candidate. Retired pairs held
by in-flight queries count towards **two total pair slots**, so a new candidate is
refused while a retired query occupies the second slot. No unbounded historical pair
registry is introduced. Active ownership lasts until replacement, close or host exit;
it is independent of the live publication history of three previous snapshots.
Queries briefly pin their pair/source owners and release on completion/cancellation.

The initial configurable allowance is **2 GiB of retained source bytes across review
owners**, including private candidates and retired queries. Reserve before retention;
count shared storage owners once, without treating matching hashes as shared ownership.
This is a source allowance, not a total memory limit: indexes, facts, transient/parser
memory and the normal live workspace are separate. Existing backing-store quotas and
read/input admission also apply. Large binary bodies can be streamed for a verified
digest without retaining a text body; preview availability is explicit. Refusal keeps
the active pair and explains how to narrow scope or change the allowance.

Capture/analysis uses the existing aggregate worker and admission budgets. Do not
allocate a fresh full parser pool for each side. Review work shares the background
allocation with recovery, reserving foreground capacity when the worker count permits;
with one worker, it serialises and yields cooperatively. No new continuous all-file
watcher/review loop is required for manual refresh.

Text diff admission initially permits at most **8 MiB combined text and 200,000 combined
lines per file pair**, **10,000 hunks per pair of files**, and **10 million algorithm
work steps** (defined by the chosen algorithm and cancellation checkpoints). Refuse
or stop explicitly when any limit is reached; never publish partial hunks as complete.
Source inspection remains available through existing fallible reads. Source panes keep
the [96-line/32-KiB windows](WINDOWS.md); lists/outline pages contain at most 200 rows,
with explicit totals/cursors. Visible source fragments retain truncation metadata.
These are starting safety bounds to qualify on the task, not measured latency claims.

## Application and transport interface

The application module owns capture tickets, admitted pairs and selection queries;
source acquisition, comparison and rendering remain independent. No hashing, parsing,
diffing or source IO occurs under its short publication lock. A later capture ticket
supersedes an earlier candidate. Accept a complete new pair atomically, then retire
the old owner. New requests for a retired token return conflict; already acquired
queries may finish but cannot replace the browser's newer state.

The local transport operations are:

| Operation | Contract |
| --- | --- |
| Create | Explicit base revision, optional target revision, and normalised repository path in the configured workspace; returns capture ticket. |
| Refresh working | Active working-tree token required; keeps that exact base/policy, returns new capture ticket. Fixed commit pairs reject refresh. |
| Status/subscribe | Retained latest ticket, active token, pending/error state; reconnect supplies current state without replay. Comparison state is separate from live snapshot status. |
| Inventory/changes | Token-bound, stable paged results with change/coverage totals. Filter is projection only. |
| Source/context/hunks | Token + side + version/range validation; bounded source windows and paged hunk/declaration metadata. |
| Close | Retire active ownership; old query tokens fail explicitly. |

[Refresh navigation](REVIEW_REFRESH.md) adds an opaque baseline-capture origin to
native labels and one optional indexed selected-row lookup to the inventory page.
The browser carries only bounded context metadata across a refresh, resolves current
versions and reveals surviving exact declaration identities. Equal side versions may
reuse viewport positions and the active expanded fragment; old tokens/hunks retain
their existing expiry semantics. No SCM-specific navigation rule enters the core model.

HTTP and finite stdio use these application operations, with the same seam available
to future editor adapters. The HTTP
mapping uses `/api/comparisons` plus token-scoped subresources; the
[comparison API reference](COMPARISON_API.md) defines the delivered routes. Change
notifications use a versioned comparison status schema, not reuse
of a live `status.snapshot` as if it named a pair. Invalid input, stale token, unavailable
source, capture failure and capacity refusal remain distinct in every adapter.

## Required correctness matrix

TDD the portable module with supplied immutable observations and a non-Git in-memory
provider before adding Git acquisition. Cover additions/deletions including empty
files, mode/type-only changes, unchanged content, newline/UTF-8 boundaries, binary
classification, exclusions/unavailable content, ambiguous declarations and policy
mismatch. Use manually specified outcomes alongside diff reconstruction properties.

Native tests add mixed staged/unstaged changes, staged edits reverted on disk,
untracked/ignored documents, staged deletions with restored disk bytes, rename-as-pair,
unmerged entries, sparse paths, unusual names, symlinks and gitlinks. Exercise moved
refs, edits during read, failed enumeration, cancellation, two storage roots sharing
one workspace, shared worker/admission bounds and baseline retention across more live
updates than the publication-history count. No test invokes analysed-project tooling.

Application/browser tests cover side/token binding, removed and unchanged context,
explicit refresh, superseded responses, history expiry, held retired queries,
capacity refusal and bounded pages. Mutation-test behaviour-changing comparison and
admission branches. Report timeouts separately from caught mutants. The frozen
good/bad patches are an additional workflow oracle, not a replacement for this matrix.
