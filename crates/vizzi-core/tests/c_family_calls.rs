//! C-family written calls have exact source identifiers for semantic lookup.
use std::sync::Arc;
use vizzi_core::{
    analysis::Languages,
    model::{CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance},
    structure::Structure,
};

#[test]
fn c_and_cpp_capture_call_positions_owners_and_definition_identifiers() {
    for (language, path, text, search) in [
        (
            "c",
            "lib.c",
            "int leaf(int x){return x;} int run(){return leaf(2);}",
            "run",
        ),
        (
            "cpp",
            "lib.cpp",
            "struct Worker { int leaf(int x){return x;} int run(){return this->leaf(2);} };",
            "Worker::run",
        ),
    ] {
        let workspace = WorkspaceId::derive(&[b"C calls"]);
        let mut parser = Languages::builtin()
            .unwrap()
            .select(language)
            .unwrap()
            .create()
            .unwrap();
        let doc = Arc::new(
            Document::capture(
                workspace.clone(),
                path.into(),
                text.as_bytes(),
                vec![parser.analyze(text.as_bytes()).unwrap()],
                Classification::Assigned {
                    language: language.into(),
                    provenance: Provenance::ExplicitScope { scope: ".".into() },
                },
            )
            .unwrap(),
        );
        let captured =
            Snapshot::new(workspace, vec![doc], CapturePolicyId::derive(&[b"fixture"])).unwrap();
        let index = Structure::new(&captured);
        let owner = index
            .functions("", search, 0)
            .unwrap()
            .items
            .remove(0)
            .symbol;
        let calls = index.outgoing(&owner.id, path, 0).unwrap();
        assert_eq!(calls.items.len(), 1, "{language}");
        let call = &calls.items[0];
        assert_eq!(call.name.as_deref(), Some("leaf"));
        let witness = index.call(&call.id).unwrap().identifier.unwrap();
        assert_eq!(&text[witness], "leaf");
        let leaf = index
            .functions("", "leaf", 0)
            .unwrap()
            .items
            .remove(0)
            .symbol;
        assert_eq!(index.identifier(&leaf.id), text.find("leaf"));
    }
}

#[test]
fn cpp_method_declarations_are_openable_but_function_pointer_fields_are_not_functions() {
    let text = "struct Worker { int run(int); int (*callback)(int); };\nint Worker::run(int x){return x;}\nint invoke(Worker& worker){return worker.run(2);}\n";
    let workspace = WorkspaceId::derive(&[b"C declarations"]);
    let mut parser = Languages::builtin()
        .unwrap()
        .select("cpp")
        .unwrap()
        .create()
        .unwrap();
    let doc = Arc::new(
        Document::capture(
            workspace.clone(),
            "lib.cpp".into(),
            text.as_bytes(),
            vec![parser.analyze(text.as_bytes()).unwrap()],
            Classification::Assigned {
                language: "cpp".into(),
                provenance: Provenance::ExplicitScope { scope: ".".into() },
            },
        )
        .unwrap(),
    );
    let captured =
        Snapshot::new(workspace, vec![doc], CapturePolicyId::derive(&[b"fixture"])).unwrap();
    let index = Structure::new(&captured);
    let methods = index.functions("", "Worker::run", 0).unwrap();
    assert_eq!(methods.items.len(), 2);
    assert!(index.functions("", "callback", 0).unwrap().items.is_empty());
    for entry in methods.items {
        assert!(index.identifier(&entry.symbol.id).is_some());
    }
    let invoke = index
        .functions("", "invoke", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let call = index
        .outgoing(&invoke.id, "lib.cpp", 0)
        .unwrap()
        .items
        .remove(0);
    assert_eq!(call.candidate_count, 2);
}

#[test]
fn calling_convention_and_pointer_declarators_keep_the_exact_function_identifier() {
    for text in [
        "int __stdcall run(void){return 1;}",
        "int (*factory(void))(int){return 0;}",
    ] {
        let workspace = WorkspaceId::derive(&[b"declarator positions"]);
        let mut parser = Languages::builtin()
            .unwrap()
            .select("c")
            .unwrap()
            .create()
            .unwrap();
        let doc = Arc::new(
            Document::capture(
                workspace.clone(),
                "lib.c".into(),
                text.as_bytes(),
                vec![parser.analyze(text.as_bytes()).unwrap()],
                Classification::Assigned {
                    language: "c".into(),
                    provenance: Provenance::ExplicitScope { scope: ".".into() },
                },
            )
            .unwrap(),
        );
        let captured =
            Snapshot::new(workspace, vec![doc], CapturePolicyId::derive(&[b"fixture"])).unwrap();
        let index = Structure::new(&captured);
        let name = if text.contains("run") {
            "run"
        } else {
            "factory"
        };
        let symbol = index.functions("", name, 0).unwrap().items.remove(0).symbol;
        assert_eq!(index.identifier(&symbol.id), text.find(name));
    }
}
