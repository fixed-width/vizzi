//! Captured input preserves validated analysis and version-driven invalidation.
use std::sync::Arc;
use vizzi_core::{
    model::{Document, WorkspaceId},
    policy::{Classification, Provenance, SourceOnlyReason},
    rust::RustAnalyzer,
};
use vizzi_search::Corpus;
fn document(name: Option<&str>) -> Arc<Document> {
    let text = b"fn source_name() {}\n";
    let provenance = Provenance::Extension {
        extension: "rs".into(),
    };
    let (analyses, classification) = if let Some(name) = name {
        let mut analysis = RustAnalyzer::new().unwrap().analyze(text).unwrap();
        analysis.facts[0].name = name.into();
        let classification = Classification::Assigned {
            language: analysis.context.language.clone(),
            provenance,
        };
        (vec![analysis], classification)
    } else {
        (
            vec![],
            Classification::SourceOnly {
                language: Some("rust".into()),
                reason: SourceOnlyReason::AnalysisNotRequested,
                provenance,
            },
        )
    };
    Arc::new(
        Document::capture(
            WorkspaceId::derive(&[b"fixture"]),
            "src/item.rs".into(),
            text,
            analyses,
            classification,
        )
        .unwrap(),
    )
}
#[test]
fn captured_analysis_is_reused_and_same_bytes_with_new_analysis_invalidate_metadata() {
    let first = document(Some("captured_name"));
    let mut corpus = Corpus::new().unwrap();
    let initial = corpus
        .replace_documents(std::slice::from_ref(&first))
        .unwrap();
    assert_eq!(initial.changed, 1);
    assert_eq!(initial.bytes, first.bytes());
    assert_eq!(initial.files, 1);
    let held = corpus.retrieve_compact("captured_name", 1, 1000).unwrap();
    assert_eq!(
        held.evidence()[0].scope.as_ref().unwrap().name,
        "captured_name"
    );
    let same = corpus.replace_documents(&[first]).unwrap();
    assert_eq!(same.changed, 0);
    assert_eq!(same.reused, 1);
    let changed = corpus
        .replace_documents(&[document(Some("updated_name"))])
        .unwrap();
    assert_eq!(changed.changed, 1);
    assert_eq!(changed.reused, 0);
    assert!(corpus.reorder(&held, &[1]).is_err());
    assert_eq!(
        corpus
            .retrieve_compact("updated_name", 1, 1000)
            .unwrap()
            .evidence()[0]
            .scope
            .as_ref()
            .unwrap()
            .name,
        "updated_name"
    );
}
#[test]
fn source_only_captures_do_not_silently_trigger_another_parser() {
    let mut corpus = Corpus::new().unwrap();
    corpus.replace_documents(&[document(None)]).unwrap();
    let context = corpus.retrieve_compact("source_name", 1, 1000).unwrap();
    assert!(context.evidence()[0].scope.is_none());
    assert_eq!(context.evidence()[0].role_basis, "path_convention");
}

#[test]
fn rejected_captures_leave_existing_results_and_context_valid() {
    let mut corpus = Corpus::new().unwrap();
    corpus.replace_documents(&[document(None)]).unwrap();
    let held = corpus.retrieve_compact("source_name", 1, 1000).unwrap();
    let snapshot = corpus.snapshot().to_owned();
    let bad = Arc::new(
        Document::capture(
            WorkspaceId::derive(&[b"fixture"]),
            "bad.rs".into(),
            &[255],
            vec![],
            document(None).classification().clone(),
        )
        .unwrap(),
    );
    assert!(corpus.replace_documents(&[bad]).is_err());
    assert_eq!(corpus.snapshot(), snapshot);
    assert!(corpus.reorder(&held, &[1]).is_ok());
    assert_eq!(
        corpus
            .retrieve_compact("source_name", 1, 1000)
            .unwrap()
            .evidence()[0]
            .text,
        held.evidence()[0].text
    );
}

#[test]
fn unavailable_captured_storage_fails_atomically_instead_of_falling_back_to_working_text() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use vizzi_core::model::{CapturedSource, SourceReadError, SourceReader};
    #[derive(Debug)]
    struct Backing(AtomicBool);
    impl SourceReader for Backing {
        fn read_at(&self, offset: usize, target: &mut [u8]) -> Result<(), SourceReadError> {
            if self.0.load(Ordering::SeqCst) {
                return Err(SourceReadError::Io("unavailable fixture".into()));
            }
            target.copy_from_slice(&b"fn backed_name() {}\n"[offset..offset + target.len()]);
            Ok(())
        }
    }
    let backing = Arc::new(Backing(AtomicBool::new(false)));
    let source =
        CapturedSource::capture_reader(b"fn backed_name() {}\n".len(), backing.clone(), &|| false)
            .unwrap()
            .unwrap();
    let doc = Arc::new(
        Document::capture_source(
            WorkspaceId::derive(&[b"fixture"]),
            "src/backed.rs".into(),
            source,
            vec![],
            document(None).classification().clone(),
        )
        .unwrap(),
    );
    let mut corpus = Corpus::new().unwrap();
    corpus.replace_documents(&[document(None)]).unwrap();
    let snapshot = corpus.snapshot().to_owned();
    backing.0.store(true, Ordering::SeqCst);
    assert!(
        corpus
            .replace_documents(std::slice::from_ref(&doc))
            .is_err()
    );
    assert_eq!(corpus.snapshot(), snapshot);
    backing.0.store(false, Ordering::SeqCst);
    assert_eq!(
        corpus
            .replace_documents(std::slice::from_ref(&doc))
            .unwrap()
            .changed,
        1
    );
    let held = corpus.retrieve_compact("backed_name", 1, 1000).unwrap();
    assert!(held.evidence()[0].text.contains("backed_name"));
    backing.0.store(true, Ordering::SeqCst);
    assert_eq!(corpus.replace_documents(&[doc]).unwrap().reused, 1);
    assert!(corpus.reorder(&held, &[1]).is_ok());
}

#[test]
fn the_window_allowance_accepts_65536_and_rejects_one_more_atomically() {
    use vizzi_core::model::Fact;
    let text = "x".repeat(4096);
    let mut analysis = RustAnalyzer::new()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    analysis.coloring = None;
    analysis.facts = (0..4096)
        .map(|n| Fact {
            parent: None,
            key: format!("token:{n}"),
            name: "x".into(),
            kind: "variable".into(),
            span: n..n + 1,
            unique: true,
        })
        .collect();
    let classification = Classification::Assigned {
        language: analysis.context.language.clone(),
        provenance: Provenance::Extension {
            extension: "rs".into(),
        },
    };
    let mut documents = (0..16)
        .map(|n| {
            Arc::new(
                Document::capture(
                    WorkspaceId::derive(&[b"window-fixture"]),
                    format!("unit-{n:02}.rs"),
                    text.as_bytes(),
                    vec![analysis.clone()],
                    classification.clone(),
                )
                .unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let mut corpus = Corpus::new().unwrap();
    assert_eq!(corpus.replace_documents(&documents).unwrap().chunks, 65536);
    let snapshot = corpus.snapshot().to_owned();
    documents.push(Arc::new(
        Document::capture(
            WorkspaceId::derive(&[b"window-fixture"]),
            "extra.rs".into(),
            b"x",
            vec![],
            document(None).classification().clone(),
        )
        .unwrap(),
    ));
    assert!(corpus.replace_documents(&documents).is_err());
    assert_eq!(corpus.snapshot(), snapshot);
}
