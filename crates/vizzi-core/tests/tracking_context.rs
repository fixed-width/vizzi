//! Behavioural checks for captured module/identity context.
use std::sync::Arc;
use vizzi_core::{
    analysis::LanguageAdapter,
    model::{CapturePolicyId, ContextId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance},
    rust::RustAdapter,
    structure::Structure,
};
fn observation(text: &str, path: &str, old: bool) -> (Arc<Document>, Structure) {
    let workspace = WorkspaceId::derive(&[b"tracking-context"]);
    let mut analysis = RustAdapter
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    if old {
        analysis.context.id = ContextId::derive(&[b"earlier-analyser"]);
    }
    let doc = Arc::new(
        Document::capture(
            workspace.clone(),
            path.into(),
            text.as_bytes(),
            vec![analysis],
            Classification::Assigned {
                language: "rust".into(),
                provenance: Provenance::Extension {
                    extension: "rs".into(),
                },
            },
        )
        .unwrap(),
    );
    let snapshot = Snapshot::new(
        workspace,
        vec![doc.clone()],
        CapturePolicyId::derive(&[b"fixture"]),
    )
    .unwrap();
    let index = Structure::new(&snapshot);
    (doc, index)
}
#[test]
fn exact_native_unique_lexical_tracking_survives_context_and_body_updates() {
    let (_, old) = observation("impl Meter { fn run(){} }", "lib.rs", true);
    let symbol = &old.functions("", "run", 0).unwrap().items[0].symbol;
    let (doc, new) = observation(
        "// moved\nimpl Meter { fn run(){ updated(); } }",
        "lib.rs",
        false,
    );
    assert!(new.resolve(Some(&symbol.tracking), &symbol.id).is_none());
    let matched = new
        .resolve_context(&doc, &symbol.tracking, &symbol.source.context)
        .unwrap();
    assert_eq!(matched.name, "run");
    assert_ne!(matched.id, symbol.id);
    assert_eq!(
        matched.source.context,
        doc.analyses()[0].context.id.as_str()
    );
    let (other, _) = observation("impl Meter { fn run(){} }", "other.rs", false);
    assert!(
        new.resolve_context(&other, &symbol.tracking, &symbol.source.context)
            .is_none()
    );
    assert!(
        new.resolve_context(&doc, &symbol.tracking, &"0".repeat(64))
            .is_none()
    );
}
#[test]
fn renamed_moved_or_duplicate_lexical_keys_are_never_recovered_by_name() {
    let (_, old) = observation("fn run(){}", "lib.rs", true);
    let symbol = &old.functions("", "run", 0).unwrap().items[0].symbol;
    for text in [
        "fn renamed(){}",
        "mod other { fn run(){} }",
        "fn run(){} fn run(){}",
    ] {
        let (doc, new) = observation(text, "lib.rs", false);
        assert!(
            new.resolve_context(&doc, &symbol.tracking, &symbol.source.context)
                .is_none()
        );
    }
    let (_, duplicates) = observation("fn run(){} fn run(){}", "lib.rs", true);
    let prior = &duplicates.functions("", "run", 0).unwrap().items[0].symbol;
    let (doc, new) = observation("fn run(){}", "lib.rs", false);
    assert!(
        new.resolve_context(&doc, &prior.tracking, &prior.source.context)
            .is_none()
    );
}
