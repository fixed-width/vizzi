//! Source-pane actions refer to exact captured identifier bytes, not text-name guesses.
use std::sync::Arc;
use vizzi_core::{
    analysis::Languages,
    model::{Analysis, CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance},
    rust::RustAnalyzer,
    structure::{SourceAction, Structure},
};
fn index(text: &str) -> Structure {
    fixture(text).0
}
fn fixture(text: &str) -> (Structure, Arc<Document>) {
    let analysis = RustAnalyzer::new()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    captured_fixture("rust", "lib.rs", text, analysis)
}

fn captured_fixture(
    language: &str,
    path: &str,
    text: &str,
    analysis: Analysis,
) -> (Structure, Arc<Document>) {
    let workspace = WorkspaceId::derive(&[b"call actions"]);
    let doc = Arc::new(
        Document::capture(
            workspace.clone(),
            path.into(),
            text.as_bytes(),
            vec![analysis],
            Classification::Assigned {
                language: language.into(),
                provenance: Provenance::ExplicitScope { scope: ".".into() },
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
    (Structure::new(&snapshot), doc)
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
fn source_actions_include_calls_and_function_names_within_the_window() {
    let (index, doc) = fixture(
        "fn start() { step(); } struct Entry;\nfn step() { finish(); }\nfn outside() { skip(); }",
    );
    let page = index.source_actions(&doc, 1, 2, 0).unwrap();
    assert_eq!(page.coverage.available_files, 1);
    let actions = page
        .items
        .iter()
        .map(|action| match action {
            SourceAction::Call(action) => {
                let call = index.call(&action.id).unwrap();
                assert!(call.invocation.owner.is_some());
                (
                    "call",
                    action.name.as_str(),
                    action.position.line,
                    action.position.start,
                )
            }
            SourceAction::Function { id, name, position } => {
                assert_eq!(index.resolve(None, id).unwrap().name, *name);
                ("function", name.as_str(), position.line, position.start)
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actions,
        [
            ("call", "step", 1, 13),
            ("call", "finish", 2, 12),
            ("function", "start", 1, 3),
            ("function", "step", 2, 3)
        ]
    );
    assert!(index.source_actions(&doc, 1, 97, 0).is_err());
}

#[test]
fn source_actions_require_the_captured_document_version() {
    let (index, original) = fixture("fn start() { step(); }");
    let (_, edited) = fixture("fn start() { stop(); }");
    assert_eq!(original.id(), edited.id());
    assert!(
        index
            .source_actions(&edited, 1, 1, 0)
            .unwrap()
            .items
            .is_empty()
    );
}

#[test]
fn source_actions_skip_a_function_name_spanning_multiple_lines() {
    let text = "struct Worker { int run(); };\nint Worker::\nrun() { return 1; }";
    let analysis = Languages::builtin()
        .unwrap()
        .select("cpp")
        .unwrap()
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    let (index, doc) = captured_fixture("cpp", "lib.cpp", text, analysis);
    let definition = index
        .inventory("lib.cpp", "Worker::", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    assert!(index.identifier(&definition.id).is_some());
    assert!(
        index
            .source_actions(&doc, 2, 2, 0)
            .unwrap()
            .items
            .is_empty()
    );
}

#[test]
fn source_actions_only_offer_calls_with_an_openable_caller() {
    let text = "def destination():\n    pass\ndestination()\n";
    let analysis = Languages::builtin()
        .unwrap()
        .select("python")
        .unwrap()
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    let (index, doc) = captured_fixture("python", "lib.py", text, analysis);
    let call = index
        .call_at("lib.py", text.rfind("destination").unwrap())
        .unwrap();
    assert!(call.invocation.owner.is_none());
    assert!(
        index
            .source_actions(&doc, 3, 1, 0)
            .unwrap()
            .items
            .is_empty()
    );
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
