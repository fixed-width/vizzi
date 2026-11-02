# Captured document and snapshot identities

This reference describes identity for source and analysis consumers. Identifiers are
opaque; clients compare them and do not parse their representation or depend on a
particular digest value across implementation versions.

`WorkspaceId` scopes a source adapter's workspace. `FileId` identifies a normalised
relative path within it. `ContentId` fingerprints captured raw bytes, including
unsupported encodings. These identities do not use source-control revision identifiers.

`CapturePolicyId` fingerprints the normalized source rules, policy implementation,
and available analyzer language names. Snapshots retain it even when empty; a mismatch
forces complete discovery before unread documents can be retained. See
[source policy](SOURCE_POLICY.md) for inputs and normalization.

`DocumentVersion` composition v2 identifies captured contents, source classification,
and all validated analyses. It combines file identity, content identity, and a digest
of classification/provenance plus the ordered analysis records. Those records include
content/context identity, language, analyzer description,
coverage, diagnostics, and every declaration's key, name, kind, byte range, and uniqueness.
Line indices are derived from captured bytes and do not introduce an independent input.

A document's version is computed once at capture, after validation, and retained in
the immutable document. Reusing an `Arc<Document>` reuses that version. Cached facts
are rebound and validated against newly captured bytes before computing a version;
no persisted document fingerprint is trusted as a substitute for validation.

Snapshot identity composition version 3 hashes the workspace, capture policy, and path-sorted document
versions using explicit domain separation. Snapshot construction still rejects duplicate
paths and mismatched workspaces. It no longer serialises unchanged analysis records.
Shared-document updates and freshly recaptured documents must produce identical
snapshot identities for identical input observations.

This changes digest values from document v1/snapshot v2. Existing identifiers
are not persisted lookup keys; live source/view pins refer to the host's retained
observations. Host restarts use a fresh notification stream identity. The analyzer
fingerprint conservatively includes model implementation sources, so this change also
invalidates old analysis-cache contexts without trusting stale facts.

`EntityId` remains separate: unique lexical declarations retain identity across position
changes, while ambiguous keys are content-scoped. A new document version does not imply
that its path or all declaration identities changed.
