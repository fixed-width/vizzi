# Verified capture-plan reference

`vizzi_core::capture::CapturePlan` freezes a portable source policy and the complete
registered analyzer descriptors. Its `CapturePlanId` hashes a version domain, validation
implementation, capture-policy identity, and deterministic language-to-context mapping.
Context language, description, and identity all participate. It does not use workspace
paths, filesystem metadata, source-control revisions, or a caller's assertion of trust.

`CapturePlan::snapshot(workspace, documents)` validates every document's classification
against the frozen policy and its analyses against the registered context. Assigned
documents require one matching raw context; source-only documents require no analyses.
Normal document range/content validation remains in `Document::capture`. The resulting
snapshot receives a private compatibility marker only after validation succeeds.

`Snapshot::new` remains available for generic immutable observations. Those snapshots
have no compatibility marker, even when a caller supplies a matching policy identity.
The native engine conservatively uses complete capture before retaining their unread
documents. A marker is a proof of compatibility checks, not semantic analyzer accuracy
or a security attestation.

## Updating a compatible baseline

`CapturePlan::update(previous, changed, removed)` requires the same verified plan.
Only changed documents undergo classification/context validation. Removed paths must
be normalized; duplicate changed paths and change/removal conflicts are errors. A
sorted merge retains unchanged `Arc<Document>` values. Workspace and duplicate-path
validation still apply to the resulting snapshot. The previous observation is immutable.

The native directory owns policy and descriptors through the plan. Hint eligibility
checks plan identity, path/count limits, and directory/ancestor safeguards. Plan mismatch
or absent verification forces discovery, including for an empty baseline. New source
bytes are still reopened and hashed; timestamps never become proof of unchanged data.

`Snapshot::file_by_path` uses binary search over sorted paths. The engine no longer
rebuilds a full path map or reclassifies every baseline document for each hint. Full
captures can still reuse compatible old documents after verifying their bytes.

Snapshot construction checks adjacent sorted paths instead of allocating a path set.
It caches the number of files with analysis. Hinted updates still perform a linear
Arc merge, workspace/hash checks, and version composition. Total update cost is not
claimed to be proportional only to changed files; full-byte recovery remains scheduled.

## Identity and transport

Snapshot composition v4 includes presence and value of the verified plan as well as
workspace, policy, and document versions. Generic and verified snapshots are distinct.
Changing a context descriptor changes identity even for an empty workspace, preventing
unchanged-publication handling from retaining an obsolete compatibility marker.

Workspace/file/entity rules and document composition v2 remain. Model-source analyzer
fingerprinting conservatively invalidates prior analysis-cache contexts once. Captured
plans/snapshots are not persisted cache records or deserialized as trusted markers.

View/source schema remains 2. The additive selected-source `version` field is the
immutable `DocumentVersion`; clients can reuse source projections while that key and
file identity remain equal. See [identity composition](IDENTITIES.md),
[source policy](SOURCE_POLICY.md), and [host API](API.md).
