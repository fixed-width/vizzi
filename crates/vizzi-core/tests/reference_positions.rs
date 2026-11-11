//! Native source-position matching for references and callers.
use std::sync::Arc;
use vizzi_core::{
    model::{CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance},
    rust::RustAnalyzer,
    structure::Structure,
};
#[test]
fn references_find_the_containing_function_and_only_exact_written_call_identifiers() {
    let text = "fn target(){}\nfn caller(){ target(); let saved=target; }\n";
    let workspace = WorkspaceId::derive(&[b"references"]);
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
    let snapshot =
        Snapshot::new(workspace, vec![doc], CapturePolicyId::derive(&[b"fixture"])).unwrap();
    let index = Structure::new(&snapshot);
    let call = text.find("target();").unwrap();
    let value = text.rfind("target;").unwrap();
    assert_eq!(index.enclosing("lib.rs", call).unwrap().name, "caller");
    assert!(index.call_at("lib.rs", call).is_some());
    assert!(index.call_at("lib.rs", value).is_none());
    let target = index
        .functions("", "target", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    assert_eq!(
        index.identifier(&target.id),
        Some(text.find("target").unwrap())
    );
    assert!(index.enclosing("missing.rs", call).is_none());
}
