//! Ownership, byte verification, and immutable source independent of analysis contexts.

use std::sync::Arc;
use vizzi_core::{
    analysis::Languages,
    model::{CapturedSource, ContentId, Document, SourceEncoding, WorkspaceId},
    policy::Classification,
};

#[test]
fn owned_capture_adopts_input_and_only_shares_after_matching_current_bytes() {
    let bytes = "α\r\nsecond\n".as_bytes().to_vec();
    let pointer = bytes.as_ptr();
    let source = CapturedSource::capture(bytes, None);
    assert_eq!(source.bytes().as_ptr(), pointer);
    assert_eq!(source.text(), Some("α\r\nsecond\n"));
    assert_eq!(source.encoding(), SourceEncoding::Utf8);
    assert_eq!(source.content(), &ContentId::derive(&[source.bytes()]));
    let same = CapturedSource::capture(source.bytes().to_vec(), Some(&source));
    assert!(Arc::ptr_eq(&source, &same));
    let changed = CapturedSource::capture("β\r\nsecond\n".as_bytes().to_vec(), Some(&source));
    assert!(!Arc::ptr_eq(&source, &changed));
    assert_eq!(changed.text(), Some("β\r\nsecond\n"));
    assert_eq!(source.text(), Some("α\r\nsecond\n"));
    let binary = vec![255, 0, 10];
    let pointer = binary.as_ptr();
    let binary = CapturedSource::capture(binary, None);
    assert_eq!(binary.bytes().as_ptr(), pointer);
    assert_eq!(binary.bytes(), &[255, 0, 10]);
    assert_eq!(binary.text(), None);
    assert_eq!(binary.encoding(), SourceEncoding::Unsupported);
    assert!(Arc::ptr_eq(
        &binary,
        &CapturedSource::capture(vec![255, 0, 10], Some(&binary))
    ));
    assert_ne!(
        binary.content(),
        CapturedSource::capture(Vec::new(), None).content()
    );
}

#[test]
fn documents_share_source_and_lines_while_validating_every_analysis() {
    let bytes = "// α\r\nfn café() {}\n".as_bytes();
    let source = CapturedSource::capture(bytes.to_vec(), None);
    let languages = Languages::builtin().unwrap();
    let analysis = languages
        .select("rust")
        .unwrap()
        .create()
        .unwrap()
        .analyze(bytes)
        .unwrap();
    let classification = Classification::Assigned {
        language: "rust".into(),
        provenance: vizzi_core::policy::Provenance::Extension {
            extension: "rs".into(),
        },
    };
    let workspace = WorkspaceId::derive(&[b"shared source"]);
    let first = Document::capture_source(
        workspace.clone(),
        "a.rs".into(),
        source.clone(),
        vec![analysis.clone()],
        classification.clone(),
    )
    .unwrap();
    let ordinary = Document::capture(
        workspace.clone(),
        "a.rs".into(),
        bytes,
        vec![analysis.clone()],
        classification.clone(),
    )
    .unwrap();
    assert_eq!(first.version(), ordinary.version());
    assert!(Arc::ptr_eq(first.captured(), &source));
    assert_eq!(first.lines(&first.analyses()[0].facts[0].span), 2..2);
    assert_eq!(first.bytes(), bytes.len());
    assert_eq!(first.content(), source.content());
    let mut revised = analysis.clone();
    revised.context.description.push_str("changed context");
    let second = Document::capture_source(
        workspace.clone(),
        "a.rs".into(),
        source.clone(),
        vec![revised],
        classification.clone(),
    )
    .unwrap();
    assert!(Arc::ptr_eq(first.captured(), second.captured()));
    assert_ne!(first.version(), second.version());
    let mut wrong = analysis;
    wrong.content = ContentId::derive(&[b"wrong source"]);
    assert!(
        Document::capture_source(
            workspace,
            "a.rs".into(),
            source,
            vec![wrong],
            classification
        )
        .is_err()
    );
}
