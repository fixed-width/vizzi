//! Source-pane actions refer to exact captured identifier bytes, not text-name guesses.
use std::sync::Arc;
use vizzi_core::{
    model::{CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance},
    rust::RustAnalyzer,
    structure::Structure,
};
fn index(text: &str) -> Structure {
    let workspace = WorkspaceId::derive(&[b"call actions"]);
    let analysis = RustAnalyzer::new()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    let doc = Arc::new(
        Document::capture(
            workspace.clone(),
            "lib.rs".into(),
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
    Structure::new(
        &Snapshot::new(workspace, vec![doc], CapturePolicyId::derive(&[b"fixture"])).unwrap(),
    )
}
#[test]
fn annotations_locate_the_identifier_after_unicode_and_line_wrapping() {
    let text = "fn process() { let _s=\"🙂\"; self.meter.run(); other.run();\n self.meter\n .run::<run>(); }";
    let index = index(text);
    let owner = index
        .functions("lib.rs", "process", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let first = index.actions(&owner.id, 1, 1, 0).unwrap();
    assert_eq!(first.matching, 2);
    assert_eq!(first.items[0].name, "run");
    assert_eq!(first.items[0].position.line, 1);
    assert_eq!(first.items[0].position.start, text.find("run()").unwrap());
    assert_eq!(
        first.items[0].position.end,
        first.items[0].position.start + 3
    );
    assert_ne!(first.items[0].id, first.items[1].id);
    assert_eq!(index.actions(&owner.id, 1, 2, 0).unwrap().matching, 2);
    let wrapped = index.actions(&owner.id, 3, 1, 0).unwrap();
    assert_eq!(wrapped.matching, 1);
    assert_eq!(wrapped.items[0].position.start, 2);
    assert_eq!(wrapped.items[0].position.end, 5);
    assert_eq!(
        index
            .call(&wrapped.items[0].id)
            .unwrap()
            .invocation
            .spelling,
        "self.meter\n .run::<run>"
    );
    assert!(index.actions(&owner.id, 4, 1, 0).unwrap().items.is_empty());
}
#[test]
fn annotations_are_bounded_paged_and_owned_by_the_selected_declaration() {
    let text = format!(
        "fn process() {{ {} fn nested() {{ private(); }} }} fn other() {{ hidden(); }}",
        "run();".repeat(257)
    );
    let index = index(&text);
    let owner = index
        .functions("lib.rs", "process", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let page = index.actions(&owner.id, 1, 1, 0).unwrap();
    assert_eq!(page.matching, 257);
    assert_eq!(page.items.len(), 256);
    assert_eq!(page.next, Some(256));
    assert!(page.items.iter().all(|a| a.name == "run"));
    let last = index.actions(&owner.id, 1, 1, 256).unwrap();
    assert_eq!(last.items.len(), 1);
    assert_eq!(last.next, None);
    assert!(index.actions(&owner.id, 0, 1, 0).is_err());
    assert!(index.actions(&owner.id, 1, 0, 0).is_err());
    assert!(index.actions(&owner.id, 1, 97, 0).is_err());
    assert!(index.actions("missing", 1, 1, 0).is_err());
}

#[test]
fn an_identifier_at_the_end_of_a_line_is_an_action_on_that_line() {
    let index = index("fn process() {\nrun\n();\n}\n");
    let owner = index
        .functions("lib.rs", "process", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let page = index.actions(&owner.id, 2, 96, 0).unwrap();
    assert_eq!(page.matching, 1);
    assert_eq!(page.items[0].position.line, 2);
    assert_eq!(page.items[0].position.start, 0);
    assert_eq!(page.items[0].position.end, 3);
    assert!(index.actions(&owner.id, 3, 1, 0).unwrap().items.is_empty());
}
