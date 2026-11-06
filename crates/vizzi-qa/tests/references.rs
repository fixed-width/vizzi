//! Evidence IDs bind model answers to native-held source, without trusting model locations.
use vizzi_qa::{Corpus, IdAnswer, SourceFile, Status};

#[test]
fn id_answers_resolve_only_supplied_labels_to_canonical_source_locations() {
    let mut corpus = Corpus::new().unwrap();
    corpus
        .replace(vec![SourceFile {
            path: "src/retry.rs".into(),
            text: "// transport\npub const RETRIES: usize = 7;\n".into(),
        }])
        .unwrap();
    let context = corpus.retrieve_compact("RETRIES", 3, 2500).unwrap();
    let answer = IdAnswer {
        status: Status::Answered,
        answer: "Seven retries.".into(),
        citations: vec![1],
    };
    let references = corpus.validate_ids(&context, &answer).unwrap();
    assert_eq!(references.len(), 1);
    let source = &context.evidence()[0];
    let reference = &references[0];
    assert_eq!(reference.label, 1);
    assert_eq!(reference.snapshot, context.snapshot());
    assert_eq!(reference.path, "src/retry.rs");
    assert_eq!(reference.content, source.content);
    assert_eq!(reference.id, source.id);
    assert_eq!(reference.start_line, source.start_line);
    assert_eq!(reference.end_line, source.end_line);
    for citations in [
        vec![],
        vec![0],
        vec![2],
        vec![usize::MAX],
        vec![1, 1],
        vec![1; 9],
    ] {
        assert!(
            corpus
                .validate_ids(
                    &context,
                    &IdAnswer {
                        status: Status::Answered,
                        answer: "Seven retries".into(),
                        citations,
                    }
                )
                .is_err()
        );
    }
    for answer in ["".into(), " \n".into(), "x".repeat(4097)] {
        assert!(
            corpus
                .validate_ids(
                    &context,
                    &IdAnswer {
                        status: Status::Answered,
                        answer,
                        citations: vec![1]
                    }
                )
                .is_err()
        );
    }
    assert!(
        corpus
            .validate_ids(
                &context,
                &IdAnswer {
                    status: Status::InsufficientEvidence,
                    answer: "No evidence of the deployment configuration.".into(),
                    citations: vec![]
                }
            )
            .unwrap()
            .is_empty()
    );
    corpus
        .replace(vec![SourceFile {
            path: "src/retry.rs".into(),
            text: "pub const RETRIES: usize = 9;\n".into(),
        }])
        .unwrap();
    assert!(
        corpus
            .validate_ids(&context, &answer)
            .unwrap_err()
            .to_string()
            .contains("different source snapshot")
    );
}

#[test]
fn id_answer_schema_rejects_model_supplied_locations_and_quoted_citations() {
    for raw in [
        r#"{"status":"answered","answer":"Seven","citations":[{"label":1,"quote":"7"}]}"#,
        r#"{"status":"answered","answer":"Seven","citations":[1],"path":"invented.rs"}"#,
        r#"{"status":"answered","answer":"Seven","citations":[-1]}"#,
    ] {
        assert!(serde_json::from_str::<IdAnswer>(raw).is_err());
    }
}
