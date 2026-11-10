//! Native tracking keeps view identity separate from observation-bound evidence.
use std::sync::Arc;
use vizzi_core::{
    analysis::LanguageAdapter,
    model::{CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance},
    python::PythonAdapter,
    rust::RustAdapter,
    structure::Structure,
};
fn index(adapter: &dyn LanguageAdapter, text: &str) -> Structure {
    let workspace = WorkspaceId::derive(&[b"live"]);
    let analysis = adapter.create().unwrap().analyze(text.as_bytes()).unwrap();
    let language = analysis.context.language.clone();
    let doc = Document::capture(
        workspace.clone(),
        "source".into(),
        text.as_bytes(),
        vec![analysis],
        Classification::Assigned {
            language,
            provenance: Provenance::Extension {
                extension: "fixture".into(),
            },
        },
    )
    .unwrap();
    Structure::new(
        &Snapshot::new(
            workspace,
            vec![Arc::new(doc)],
            CapturePolicyId::derive(&[b"fixture"]),
        )
        .unwrap(),
    )
}
#[test]
fn unique_declarations_track_body_edits_and_line_movement_in_mixed_languages() {
    for (adapter, old, new) in [
        (
            &RustAdapter as &dyn LanguageAdapter,
            "fn run() {}",
            "// moved\nfn run(){ updated(); }",
        ),
        (
            &PythonAdapter,
            "class Meter:\n    def run(self):\n        pass\n",
            "# moved\nclass Meter:\n    def run(self):\n        updated()\n",
        ),
    ] {
        let a = index(adapter, old);
        let b = index(adapter, new);
        let symbol = a.functions("", "run", 0).unwrap().items.remove(0).symbol;
        let next = b.resolve(Some(&symbol.tracking), &symbol.id).unwrap();
        assert_ne!(next.id, symbol.id);
        assert_eq!(next.tracking, symbol.tracking);
        assert_eq!(next.source.line, symbol.source.line + 1);
        assert!(b.resolve(None, &symbol.id).is_none());
        assert!(a.resolve(None, &symbol.id).is_some());
    }
}
#[test]
fn rename_deletion_and_edited_duplicate_names_never_choose_a_neighbour() {
    let old = index(&RustAdapter, "fn run() {} fn other() {}");
    let symbol = old.functions("", "run", 0).unwrap().items.remove(0).symbol;
    for text in ["fn renamed() {} fn other() {}", "fn other() {}"] {
        assert!(
            index(&RustAdapter, text)
                .resolve(Some(&symbol.tracking), &symbol.id)
                .is_none()
        );
    }
    let old = index(&RustAdapter, "fn run() {} fn run() {}");
    let symbol = old.functions("", "run", 0).unwrap().items.remove(0).symbol;
    assert!(
        index(&RustAdapter, "fn run() {changed();} fn run() {}")
            .resolve(Some(&symbol.tracking), &symbol.id)
            .is_none()
    );
}
#[test]
fn refreshed_links_require_a_unique_current_site_and_current_destination_evidence() {
    let structure = index(
        &RustAdapter,
        "struct Meter; impl Meter { fn start(&self) { self.end(); } fn end(&self) {} } fn plain(){ end(); } fn end(){}",
    );
    let start = structure
        .functions("", "Meter::start", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let end = structure
        .functions("", "Meter::end", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let result = structure
        .check_link(&start.id, &end.id, "self.end", "self")
        .unwrap();
    assert_eq!(result.state, "current");
    assert_eq!(result.kind, Some("self"));
    assert_eq!(
        structure
            .check_link(&start.id, &end.id, "self.removed", "self")
            .unwrap()
            .state,
        "removed"
    );
    let plain = structure
        .functions("", "plain", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    assert_eq!(
        structure
            .check_link(&start.id, &plain.id, "self.end", "self")
            .unwrap()
            .state,
        "unverified"
    );
    assert_eq!(
        structure
            .check_link(&plain.id, &end.id, "end", "definition")
            .unwrap()
            .state,
        "unverified"
    );
    assert_eq!(
        structure
            .check_link(&plain.id, &end.id, "end", "candidate")
            .unwrap()
            .state,
        "current"
    );
    assert_eq!(
        structure
            .check_link(&plain.id, &start.id, "end", "candidate")
            .unwrap()
            .state,
        "unverified"
    );
    let duplicate = index(&RustAdapter, "fn start(){end();end();} fn end(){}");
    let from = duplicate
        .functions("", "start", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let to = duplicate
        .functions("", "end", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    assert_eq!(
        duplicate
            .check_link(&from.id, &to.id, "end", "candidate")
            .unwrap()
            .state,
        "unverified"
    );
}
