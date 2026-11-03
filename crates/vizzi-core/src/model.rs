//! Captured source and immutable analysis identities.
pub use crate::captured_source::{
    BULK_SOURCE_LIMIT, CapturedSource, SourceInput, SourceReadError, SourceReader, SourceStorage,
    SourceVerifier,
};

use crate::policy::Classification;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::{Arc, OnceLock},
};

macro_rules! identity {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            /// Derives a namespaced identity from length-delimited parts.
            pub fn derive(parts: &[&[u8]]) -> Self {
                let mut hash = blake3::Hasher::new();
                hash.update(stringify!($name).as_bytes());
                for part in parts {
                    hash.update(&(part.len() as u64).to_le_bytes());
                    hash.update(part);
                }
                Self(hash.finalize().to_hex().to_string())
            }
            /// Returns the opaque transport representation.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}
identity!(WorkspaceId, "An adapter-supplied workspace identity.");
identity!(FileId, "A path identity scoped to a workspace.");
identity!(ContentId, "An identity of captured source bytes.");
identity!(
    CapturePlanId,
    "An identity of source policy and complete analyzer descriptors."
);
identity!(
    CapturePolicyId,
    "An identity of normalized source selection and analyzer availability."
);
identity!(
    DocumentVersion,
    "An identity of a validated captured file and all its analyses."
);
identity!(
    ContextId,
    "An identity of analyzer, configuration, and implementation."
);
identity!(
    SnapshotId,
    "An identity of immutable source and analysis observations."
);
identity!(
    EntityId,
    "An identity of a declaration in a file and analysis context."
);

/// The language and implementation responsible for a set of facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    /// Analyzer language identifier.
    pub language: String,
    /// Human-readable analyzer and configuration description.
    pub description: String,
    /// Fingerprint including analyzer implementation and configuration.
    pub id: ContextId,
}

impl ContentId {
    pub(crate) fn from_digest(digest: blake3::Hash) -> Self {
        Self(digest.to_hex().to_string())
    }
}

/// Coverage of syntax extraction, without a semantic-resolution claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    /// Parsing completed without grammar recovery.
    Syntax,
    /// Grammar recovery occurred; extracted declarations may be incomplete.
    Partial,
    /// Analysis could not run on this input.
    Unavailable,
    /// Analysis was postponed by resource admission; no facts were produced.
    Deferred,
}

/// Captured text availability, independent of analyzer assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceEncoding {
    /// Captured bytes are valid UTF-8.
    Utf8,
    /// The prototype has no decoder for these captured bytes.
    Unsupported,
}

/// A lexical declaration before binding to a workspace path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    /// Lexical qualification and kind, with a duplicate ordinal when needed.
    pub key: String,
    /// Display name.
    pub name: String,
    /// Declaration kind.
    pub kind: String,
    /// UTF-8 byte range with an exclusive end.
    pub span: Range<usize>,
    /// Whether this lexical identity is unambiguous in the captured file.
    pub unique: bool,
}

/// Path-independent facts that can be persisted in an analysis cache.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Analysis {
    /// Captured byte identity to which the facts apply.
    pub content: ContentId,
    /// Analyzer configuration and implementation identity.
    pub context: Context,
    /// Syntax coverage classification.
    pub coverage: Coverage,
    /// Lexical declarations, in source order.
    pub facts: Vec<Fact>,
    /// Recovery or availability explanations.
    pub diagnostics: Vec<String>,
}

/// Rejected inconsistent or ambiguous input.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ModelError(pub String);

/// A frozen source document with analyses for zero or more contexts.
#[derive(Clone, Debug)]
pub struct Document {
    workspace: WorkspaceId,
    id: FileId,
    path: String,
    version: DocumentVersion,
    source: Arc<CapturedSource>,
    analyses: Vec<Analysis>,
    classification: Classification,
    pub(crate) outline_index: OnceLock<BTreeMap<String, usize>>,
}

impl Document {
    /// Validates source-backed facts and captures immutable source text.
    pub fn capture(
        workspace: WorkspaceId,
        path: String,
        bytes: &[u8],
        analyses: Vec<Analysis>,
        classification: Classification,
    ) -> Result<Self, ModelError> {
        Self::capture_source(
            workspace,
            path,
            CapturedSource::capture(bytes.to_vec(), None),
            analyses,
            classification,
        )
    }
    /// Validates facts against owned immutable source without copying its text/index.
    pub fn capture_source(
        workspace: WorkspaceId,
        path: String,
        source: Arc<CapturedSource>,
        analyses: Vec<Analysis>,
        classification: Classification,
    ) -> Result<Self, ModelError> {
        Self::capture_validated(workspace, path, source, analyses, classification, None)
    }
    /// Binds facts using the same verified whole input supplied to analysis/cache validation.
    pub fn capture_loaded(
        workspace: WorkspaceId,
        path: String,
        source: Arc<CapturedSource>,
        input: &SourceInput<'_>,
        analyses: Vec<Analysis>,
        classification: Classification,
    ) -> Result<Self, ModelError> {
        if input.content() != source.content() {
            return Err(ModelError(
                "loaded input belongs to different captured source".into(),
            ));
        }
        Self::capture_validated(
            workspace,
            path,
            source,
            analyses,
            classification,
            Some(input),
        )
    }
    fn capture_validated(
        workspace: WorkspaceId,
        path: String,
        source: Arc<CapturedSource>,
        analyses: Vec<Analysis>,
        classification: Classification,
        input: Option<&SourceInput<'_>>,
    ) -> Result<Self, ModelError> {
        if path.is_empty()
            || path.contains('\\')
            || path.split('/').any(|part| matches!(part, "" | "." | ".."))
        {
            return Err(ModelError("expected a normalized relative path".into()));
        }
        let content = source.content();
        if matches!(classification, Classification::SourceOnly { .. }) && !analyses.is_empty() {
            return Err(ModelError(
                "source-only documents cannot contain analysis".into(),
            ));
        }
        let text = input.map_or_else(|| source.resident_text(), |input| input.text());
        let mut contexts = BTreeSet::new();
        for analysis in &analyses {
            if &analysis.content != content || !contexts.insert(&analysis.context.id) {
                return Err(ModelError("analysis content or context mismatch".into()));
            }
            let mut keys = BTreeSet::new();
            if analysis.coverage == Coverage::Deferred && !analysis.facts.is_empty() {
                return Err(ModelError("deferred analysis cannot contain facts".into()));
            }
            for fact in &analysis.facts {
                if !keys.insert(&fact.key)
                    || fact.span.start >= fact.span.end
                    || !text.is_some_and(|text| text.get(fact.span.clone()).is_some())
                {
                    return Err(ModelError(
                        "invalid or duplicate declaration range/key".into(),
                    ));
                }
            }
            if source.encoding() == SourceEncoding::Unsupported
                && !matches!(
                    analysis.coverage,
                    Coverage::Unavailable | Coverage::Deferred
                )
            {
                return Err(ModelError(
                    "non-UTF-8 source requires unavailable analysis".into(),
                ));
            }
        }
        let id = FileId::derive(&[workspace.as_str().as_bytes(), path.as_bytes()]);
        let encoded = serde_json::to_vec(&(&classification, &analyses))
            .map_err(|error| ModelError(error.to_string()))?;
        let analysis_digest = blake3::hash(&encoded);
        let version = DocumentVersion::derive(&[
            b"document-v2",
            id.as_str().as_bytes(),
            content.as_str().as_bytes(),
            analysis_digest.as_bytes(),
        ]);
        Ok(Self {
            id,
            workspace,
            path,
            version,
            source,
            analyses,
            classification,
            outline_index: OnceLock::new(),
        })
    }
    /// Returns the path identity.
    pub fn id(&self) -> &FileId {
        &self.id
    }
    /// Returns the workspace-relative path.
    pub fn path(&self) -> &str {
        &self.path
    }
    /// Returns the captured byte identity.
    pub fn content(&self) -> &ContentId {
        self.source.content()
    }
    /// Returns the cached identity of this captured file and its validated analyses.
    pub fn version(&self) -> &DocumentVersion {
        &self.version
    }
    /// Returns the captured byte count.
    pub fn bytes(&self) -> usize {
        self.source.len()
    }
    /// Returns resident UTF-8 source without IO; absence is not an encoding decision.
    pub fn resident_source(&self) -> Option<&str> {
        self.source.resident_text()
    }
    /// Returns captured encoding availability independently of classification.
    pub fn encoding(&self) -> SourceEncoding {
        self.source.encoding()
    }
    /// Returns shared immutable source storage, independently of classification/facts.
    pub fn captured(&self) -> &Arc<CapturedSource> {
        &self.source
    }
    /// Returns all analysis contexts for this document.
    pub fn analyses(&self) -> &[Analysis] {
        &self.analyses
    }
    /// Returns immutable source-selection metadata.
    pub fn classification(&self) -> &Classification {
        &self.classification
    }
    /// Binds a declaration identity, conservatively scoping duplicate keys to content.
    pub fn entity_id(&self, analysis: &Analysis, fact: &Fact) -> EntityId {
        let mut parts = vec![
            self.id.as_str().as_bytes(),
            analysis.context.id.as_str().as_bytes(),
            fact.key.as_bytes(),
        ];
        if !fact.unique {
            parts.push(self.content().as_str().as_bytes());
        }
        EntityId::derive(&parts)
    }
    /// Converts a nonempty byte span to inclusive one-based source lines.
    pub fn lines(&self, span: &Range<usize>) -> Range<usize> {
        self.source.lines(span)
    }
}

/// An immutable, path-sorted collection of captured documents.
#[derive(Debug)]
pub struct Snapshot {
    id: SnapshotId,
    workspace: WorkspaceId,
    documents: Vec<Arc<Document>>,
    policy: CapturePolicyId,
    capture_plan: Option<CapturePlanId>,
    analyzed: usize,
    deferred: usize,
}

impl Snapshot {
    /// Rejects duplicate paths and cross-workspace documents.
    pub fn new(
        workspace: WorkspaceId,
        documents: Vec<Arc<Document>>,
        policy: CapturePolicyId,
    ) -> Result<Self, ModelError> {
        Self::build(workspace, documents, policy, None)
    }
    pub(crate) fn verified(
        workspace: WorkspaceId,
        documents: Vec<Arc<Document>>,
        policy: CapturePolicyId,
        plan: CapturePlanId,
    ) -> Result<Self, ModelError> {
        Self::build(workspace, documents, policy, Some(plan))
    }
    fn build(
        workspace: WorkspaceId,
        mut documents: Vec<Arc<Document>>,
        policy: CapturePolicyId,
        capture_plan: Option<CapturePlanId>,
    ) -> Result<Self, ModelError> {
        documents.sort_by(|a, b| a.path.cmp(&b.path));
        if documents
            .windows(2)
            .any(|pair| pair[0].path() == pair[1].path())
        {
            return Err(ModelError("duplicate captured path".into()));
        }
        let mut hash = blake3::Hasher::new();
        hash.update(b"snapshot-v4");
        hash.update(workspace.as_str().as_bytes());
        hash.update(policy.as_str().as_bytes());
        match &capture_plan {
            Some(plan) => {
                hash.update(&[1]);
                hash.update(plan.as_str().as_bytes());
            }
            None => {
                hash.update(&[0]);
            }
        }
        let mut analyzed = 0;
        let mut deferred = 0;
        for document in &documents {
            if document.workspace != workspace {
                return Err(ModelError("mismatched captured workspace".into()));
            }
            analyzed += usize::from(!document.analyses().is_empty());
            deferred += usize::from(
                document
                    .analyses()
                    .iter()
                    .any(|analysis| analysis.coverage == Coverage::Deferred),
            );
            hash.update(document.version.as_str().as_bytes());
        }
        Ok(Self {
            id: SnapshotId::derive(&[hash.finalize().as_bytes()]),
            workspace,
            documents,
            policy,
            capture_plan,
            analyzed,
            deferred,
        })
    }
    /// Returns this observation's identity.
    pub fn id(&self) -> &SnapshotId {
        &self.id
    }
    /// Returns the workspace identity.
    pub fn workspace(&self) -> &WorkspaceId {
        &self.workspace
    }
    /// Returns the capture policy used for complete discovery and retention.
    pub fn policy(&self) -> &CapturePolicyId {
        &self.policy
    }
    /// Returns a compatibility marker only after capture-plan validation.
    pub fn capture_plan(&self) -> Option<&CapturePlanId> {
        self.capture_plan.as_ref()
    }
    /// Counts captured files with analysis records, including unavailable analysis.
    pub fn analyzed_files(&self) -> usize {
        self.analyzed
    }
    /// Counts files with at least one deferred analysis record.
    pub fn deferred_files(&self) -> usize {
        self.deferred
    }
    /// Finds a captured path without scanning all documents.
    pub fn file_by_path(&self, path: &str) -> Option<&Arc<Document>> {
        self.documents
            .binary_search_by(|file| file.path().cmp(path))
            .ok()
            .map(|index| &self.documents[index])
    }
    /// Returns documents in path order.
    pub fn documents(&self) -> &[Arc<Document>] {
        &self.documents
    }
    /// Finds a captured document by opaque identity.
    pub fn file(&self, id: &str) -> Option<&Arc<Document>> {
        self.documents.iter().find(|file| file.id.as_str() == id)
    }
}

#[cfg(test)]
pub(crate) fn fixture_document(
    workspace: WorkspaceId,
    path: String,
    bytes: &[u8],
    analyses: Vec<Analysis>,
) -> Result<Document, ModelError> {
    use crate::policy::{Provenance, SourceOnlyReason};
    let provenance = Provenance::Extension {
        extension: "fixture".into(),
    };
    let classification = match analyses.first() {
        Some(analysis) => Classification::Assigned {
            language: analysis.context.language.clone(),
            provenance,
        },
        None => Classification::SourceOnly {
            language: None,
            provenance,
            reason: SourceOnlyReason::LanguageUnassigned,
        },
    };
    Document::capture(workspace, path, bytes, analyses, classification)
}
#[cfg(test)]
pub(crate) fn fixture_snapshot(
    workspace: WorkspaceId,
    documents: Vec<Arc<Document>>,
) -> Result<Snapshot, ModelError> {
    Snapshot::new(
        workspace,
        documents,
        CapturePolicyId::derive(&[b"fixture-policy"]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn workspace() -> WorkspaceId {
        WorkspaceId::derive(&[b"test"])
    }
    fn analysis(text: &[u8]) -> Analysis {
        Analysis {
            content: ContentId::derive(&[text]),
            context: Context {
                language: "rust".into(),
                description: "test".into(),
                id: ContextId::derive(&[b"test"]),
            },
            coverage: Coverage::Syntax,
            facts: vec![Fact {
                key: "fn::f".into(),
                name: "f".into(),
                kind: "function".into(),
                span: 0..9,
                unique: true,
            }],
            diagnostics: vec![],
        }
    }
    #[test]
    fn rejects_facts_for_different_source_or_invalid_spans() {
        assert!(
            crate::model::fixture_document(
                workspace(),
                "a.rs".into(),
                b"fn g() {}",
                vec![analysis(b"fn f() {}")]
            )
            .is_err()
        );
        let mut facts = analysis(b"fn f() {}");
        facts.facts[0].span = 0..100;
        assert!(
            crate::model::fixture_document(workspace(), "a.rs".into(), b"fn f() {}", vec![facts])
                .is_err()
        );
    }
    #[test]
    fn stable_file_and_unique_declaration_ids_survive_position_changes() {
        let old = crate::model::fixture_document(
            workspace(),
            "a.rs".into(),
            b"fn f() {}",
            vec![analysis(b"fn f() {}")],
        )
        .unwrap();
        let mut facts = analysis(b"\nfn f() {}");
        facts.facts[0].span = 1..10;
        let new =
            crate::model::fixture_document(workspace(), "a.rs".into(), b"\nfn f() {}", vec![facts])
                .unwrap();
        assert_eq!(old.id(), new.id());
        assert_ne!(old.content(), new.content());
        assert_eq!(
            old.entity_id(&old.analyses[0], &old.analyses[0].facts[0]),
            new.entity_id(&new.analyses[0], &new.analyses[0].facts[0])
        );
        assert_eq!(new.lines(&new.analyses[0].facts[0].span), 2..2);
        assert_eq!(old.resident_source(), Some("fn f() {}"));
    }
    #[test]
    fn rejects_traversal_duplicate_contexts_and_utf8_midpoints() {
        assert!(
            crate::model::fixture_document(workspace(), "../a.rs".into(), b"", vec![]).is_err()
        );
        let a = analysis(b"fn f() {}");
        assert!(
            crate::model::fixture_document(
                workspace(),
                "a.rs".into(),
                b"fn f() {}",
                vec![a.clone(), a]
            )
            .is_err()
        );
        let mut a = analysis("éabcdefg".as_bytes());
        a.facts[0].span = 1..9;
        assert!(
            crate::model::fixture_document(
                workspace(),
                "a.rs".into(),
                "éabcdefg".as_bytes(),
                vec![a]
            )
            .is_err()
        );
    }
    #[test]
    fn snapshots_sort_and_include_analysis_in_their_identity() {
        let a = Arc::new(
            crate::model::fixture_document(workspace(), "z.rs".into(), b"", vec![]).unwrap(),
        );
        let b = Arc::new(
            crate::model::fixture_document(workspace(), "a.rs".into(), b"", vec![]).unwrap(),
        );
        let first =
            crate::model::fixture_snapshot(workspace(), vec![a.clone(), b.clone()]).unwrap();
        let second = crate::model::fixture_snapshot(workspace(), vec![b, a.clone()]).unwrap();
        assert_eq!(first.id(), second.id());
        assert_eq!(first.documents()[0].path(), "a.rs");
        assert!(crate::model::fixture_snapshot(workspace(), vec![a.clone(), a]).is_err());
        let changed = Analysis {
            content: ContentId::derive(&[b""]),
            facts: vec![],
            ..analysis(b"")
        };
        let changed = Arc::new(
            crate::model::fixture_document(workspace(), "z.rs".into(), b"", vec![changed]).unwrap(),
        );
        assert_ne!(
            crate::model::fixture_snapshot(workspace(), vec![changed])
                .unwrap()
                .id(),
            first.id()
        );
    }

    #[test]
    fn validates_empty_spans_duplicate_keys_and_unsupported_encoding() {
        for path in ["", "a\\b.rs", "/a.rs", "a//b.rs", "./a.rs"] {
            assert!(crate::model::fixture_document(workspace(), path.into(), b"", vec![]).is_err());
        }
        let mut a = analysis(b"fn f() {}");
        a.facts[0].span = 0..0;
        assert!(
            crate::model::fixture_document(workspace(), "a.rs".into(), b"fn f() {}", vec![a])
                .is_err()
        );
        let mut a = analysis(b"fn f() {}");
        a.facts.push(a.facts[0].clone());
        assert!(
            crate::model::fixture_document(workspace(), "a.rs".into(), b"fn f() {}", vec![a])
                .is_err()
        );
        let mut a = analysis(&[0xff]);
        a.facts.clear();
        assert!(
            crate::model::fixture_document(workspace(), "a.rs".into(), &[0xff], vec![a.clone()])
                .is_err()
        );
        a.coverage = Coverage::Unavailable;
        let unsupported =
            crate::model::fixture_document(workspace(), "a.rs".into(), &[0xff], vec![a]).unwrap();
        assert_eq!(unsupported.resident_source(), None);
        assert_eq!(unsupported.bytes(), 1);
        let a = crate::model::fixture_document(
            workspace(),
            "a.rs".into(),
            b"// first\n// next\nfn f() {}\n",
            vec![],
        )
        .unwrap();
        assert_eq!(a.bytes(), 27);
        assert_eq!(a.lines(&(17..26)), 3..3);
        assert_eq!(a.lines(&(0..9)), 1..1);
        assert_eq!(a.lines(&(9..17)), 2..2);
        assert_eq!(a.lines(&(26..27)), 3..3);
    }

    #[test]
    fn document_versions_cover_source_identity_and_every_analysis_field() {
        let bytes = b"fn f() {}";
        let capture = |facts| {
            crate::model::fixture_document(workspace(), "a.rs".into(), bytes, facts).unwrap()
        };
        let base = capture(vec![analysis(bytes)]);
        assert_eq!(base.version(), capture(vec![analysis(bytes)]).version());
        let changes: [fn(&mut Analysis); 11] = [
            |a| a.context.id = ContextId::derive(&[b"different"]),
            |a| a.context.language = "another language".into(),
            |a| a.context.description = "another configuration".into(),
            |a| a.coverage = Coverage::Partial,
            |a| a.diagnostics.push("recovered syntax".into()),
            |a| a.facts[0].key = "another key".into(),
            |a| a.facts[0].name = "another name".into(),
            |a| a.facts[0].kind = "another kind".into(),
            |a| a.facts[0].span = 1..8,
            |a| a.facts[0].unique = false,
            |a| a.facts.clear(),
        ];
        for change in changes {
            let mut a = analysis(bytes);
            change(&mut a);
            assert_ne!(base.version(), capture(vec![a]).version());
        }
        assert_ne!(base.version(), capture(vec![]).version());
        let other_path = crate::model::fixture_document(
            workspace(),
            "b.rs".into(),
            bytes,
            vec![analysis(bytes)],
        )
        .unwrap();
        assert_ne!(base.version(), other_path.version());
        let other_workspace = crate::model::fixture_document(
            WorkspaceId::derive(&[b"other"]),
            "a.rs".into(),
            bytes,
            vec![analysis(bytes)],
        )
        .unwrap();
        assert_ne!(base.version(), other_workspace.version());
        let empty_analysis =
            crate::model::fixture_document(workspace(), "a.rs".into(), b"fn g() {}", vec![])
                .unwrap();
        assert_ne!(capture(vec![]).version(), empty_analysis.version());
        let invalid_a =
            crate::model::fixture_document(workspace(), "a.rs".into(), &[0xff], vec![]).unwrap();
        let invalid_b =
            crate::model::fixture_document(workspace(), "a.rs".into(), &[0xfe], vec![]).unwrap();
        assert_ne!(invalid_a.version(), invalid_b.version());
    }

    #[test]
    fn ordered_analysis_contexts_and_fresh_capture_define_the_same_snapshot_as_reuse() {
        let bytes = b"fn f() {}";
        let first = analysis(bytes);
        let mut second = first.clone();
        second.context.id = ContextId::derive(&[b"second context"]);
        let capture = |path: &str, analyses| {
            Arc::new(
                crate::model::fixture_document(workspace(), path.into(), bytes, analyses).unwrap(),
            )
        };
        let a = capture("a.rs", vec![first.clone(), second.clone()]);
        let b = capture("b.rs", vec![]);
        let original = crate::model::fixture_snapshot(workspace(), vec![a.clone(), b]).unwrap();
        let fresh = crate::model::fixture_snapshot(
            workspace(),
            vec![
                capture("b.rs", vec![]),
                capture("a.rs", vec![first.clone(), second.clone()]),
            ],
        )
        .unwrap();
        assert_eq!(original.id(), fresh.id());
        let reversed = capture("a.rs", vec![second, first]);
        assert_ne!(a.version(), reversed.version());
        let changed = crate::model::fixture_snapshot(
            workspace(),
            vec![a.clone(), capture("b.rs", vec![analysis(bytes)])],
        )
        .unwrap();
        let rebuilt = crate::model::fixture_snapshot(
            workspace(),
            vec![
                capture("a.rs", a.analyses().to_vec()),
                capture("b.rs", vec![analysis(bytes)]),
            ],
        )
        .unwrap();
        assert_eq!(changed.id(), rebuilt.id());
        assert_ne!(changed.id(), original.id());
        assert_eq!(original.documents()[0].resident_source(), Some("fn f() {}"));
    }

    #[test]
    fn empty_snapshots_are_workspace_scoped_and_foreign_documents_are_rejected() {
        let first = crate::model::fixture_snapshot(workspace(), vec![]).unwrap();
        let other = WorkspaceId::derive(&[b"other"]);
        assert_ne!(
            first.id(),
            crate::model::fixture_snapshot(other.clone(), vec![])
                .unwrap()
                .id()
        );
        let foreign =
            Arc::new(crate::model::fixture_document(other, "a.rs".into(), b"", vec![]).unwrap());
        assert!(crate::model::fixture_snapshot(workspace(), vec![foreign]).is_err());
    }
    #[test]
    fn deferred_analysis_has_no_facts_and_can_describe_undecodable_captured_bytes() {
        let mut deferred = analysis(b"fn f() {}");
        deferred.coverage = Coverage::Deferred;
        assert!(
            crate::model::fixture_document(
                workspace(),
                "a.rs".into(),
                b"fn f() {}",
                vec![deferred.clone()]
            )
            .is_err()
        );
        deferred.facts.clear();
        let source = b"\xff";
        deferred.content = ContentId::derive(&[source]);
        let document =
            crate::model::fixture_document(workspace(), "a.rs".into(), source, vec![deferred])
                .unwrap();
        assert_eq!(document.encoding(), SourceEncoding::Unsupported);
        assert_eq!(document.analyses()[0].coverage, Coverage::Deferred);
    }
}
