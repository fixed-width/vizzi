//! Optional real language-server integration; normal workspace tests use protocol/fake providers.
use std::sync::Arc;
use vizzi::{
    definitions::{Provider, byte_offset},
    rust_definitions::RustAnalyzer as Service,
};
use vizzi_core::{
    model::{CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance},
    rust::RustAnalyzer,
    structure::Structure,
};
fn capture(files: &[(&str, &str)]) -> Arc<Snapshot> {
    let workspace = WorkspaceId::derive(&[b"semantic fixture"]);
    let mut parser = RustAnalyzer::new().unwrap();
    let docs = files
        .iter()
        .map(|(path, text)| {
            Arc::new(
                Document::capture(
                    workspace.clone(),
                    (*path).into(),
                    text.as_bytes(),
                    vec![parser.analyze(text.as_bytes()).unwrap()],
                    Classification::Assigned {
                        language: "rust".into(),
                        provenance: Provenance::Extension {
                            extension: "rs".into(),
                        },
                    },
                )
                .unwrap(),
            )
        })
        .collect();
    Arc::new(Snapshot::new(workspace, docs, CapturePolicyId::derive(&[b"fixture"])).unwrap())
}
fn call(snapshot: &Snapshot) -> vizzi_core::structure::CallLookup {
    let index = Structure::new(snapshot);
    let process = index
        .inventory("lib.rs", "process", 0)
        .unwrap()
        .items
        .into_iter()
        .find(|e| e.symbol.parents.iter().any(|p| p.contains("for Service")))
        .unwrap();
    let call = index
        .outgoing(&process.symbol.id, "lib.rs", 0)
        .unwrap()
        .items
        .remove(0);
    index.call(&call.id).unwrap()
}
#[test]
#[ignore = "requires installed rust-analyzer; run explicitly during semantic-provider integration"]
fn field_types_resolve_across_captured_files_and_reuse_the_session_after_changes() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("Cargo.toml"),"[package]\nname=\"semantic_fixture\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[lib]\npath=\"lib.rs\"\n").unwrap();
    let caller = "mod worker; trait Plugin { fn process(&mut self); } struct Service { meter: worker::Meter } impl Plugin for Service { fn process(&mut self) { self.meter.run(); } }";
    let worker = "pub struct Meter; impl Meter { pub fn run(&mut self) {} }";
    let build = format!(
        "fn main() {{ std::fs::write({:?}, b\"unexpected\").unwrap(); }}",
        root.path().join("BUILT").to_str().unwrap()
    );
    for (path, text) in [
        ("lib.rs", caller),
        ("worker.rs", worker),
        ("build.rs", build.as_str()),
    ] {
        std::fs::write(root.path().join(path), text).unwrap();
    }
    let snapshot = capture(&[
        ("lib.rs", caller),
        ("worker.rs", worker),
        ("build.rs", &build),
    ]);
    std::fs::write(
        root.path().join("worker.rs"),
        "pub struct Wrong; impl Wrong { fn unrelated() {} }",
    )
    .unwrap();
    let service = Service::new(
        root.path().into(),
        cache.path().into(),
        "rust-analyzer".into(),
    );
    let answer = service.resolve(snapshot.clone(), &call(&snapshot)).unwrap();
    assert_eq!(answer.locations.len(), 1);
    assert_eq!(answer.outside, 0);
    let location = &answer.locations[0];
    assert_eq!(location.path, "worker.rs");
    assert_eq!(
        Structure::new(&snapshot)
            .definition("worker.rs", byte_offset(worker, location.position).unwrap())
            .unwrap()
            .name,
        "run"
    );
    assert!(!root.path().join("BUILT").exists());
    let mirrors = std::fs::read_dir(cache.path())
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(mirrors.len(), 1);
    let changed = format!("// moved definition\n{worker}");
    let next = capture(&[
        ("lib.rs", caller),
        ("worker.rs", &changed),
        ("build.rs", &build),
    ]);
    let result = service.resolve(next.clone(), &call(&next)).unwrap();
    assert_eq!(result.locations.len(), 1);
    assert_eq!(result.locations[0].position.line, 1);
    assert_ne!(result.context, answer.context);
    assert_eq!(
        std::fs::read_dir(cache.path())
            .unwrap()
            .map(|p| p.unwrap().path())
            .collect::<Vec<_>>(),
        mirrors
    );
    assert!(!root.path().join("BUILT").exists());
    drop(service);
    assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 0);
}

#[test]
#[ignore = "requires installed rust-analyzer; run explicitly during semantic-provider integration"]
fn aliases_and_returned_receiver_values_resolve_without_terminal_name_filters() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("Cargo.toml"),"[package]\nname=\"semantic_aliases\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[lib]\npath=\"lib.rs\"\n").unwrap();
    let caller = "mod worker; use worker::actual as renamed; fn factory()->worker::Meter { worker::Meter } fn entry() { renamed(); let mut meter=factory(); meter.run(); }";
    let worker = "pub fn actual() {} pub struct Meter; impl Meter { pub fn run(&mut self) {} }";
    for (path, text) in [("lib.rs", caller), ("worker.rs", worker)] {
        std::fs::write(root.path().join(path), text).unwrap();
    }
    let snapshot = capture(&[("lib.rs", caller), ("worker.rs", worker)]);
    let index = Structure::new(&snapshot);
    let entry = index
        .inventory("lib.rs", "entry", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let calls = index.outgoing(&entry.id, "lib.rs", 0).unwrap();
    let service = Service::new(
        root.path().into(),
        cache.path().into(),
        "rust-analyzer".into(),
    );
    for (spelling, expected) in [("renamed", "actual"), ("meter.run", "run")] {
        let site = calls.items.iter().find(|s| s.spelling == spelling).unwrap();
        let lookup = index.call(&site.id).unwrap();
        let answer = service.resolve(snapshot.clone(), &lookup).unwrap();
        assert_eq!(answer.locations.len(), 1);
        let location = &answer.locations[0];
        assert_eq!(location.path, "worker.rs");
        assert_eq!(
            index
                .definition(
                    &location.path,
                    byte_offset(worker, location.position).unwrap()
                )
                .unwrap()
                .name,
            expected
        );
    }
}
