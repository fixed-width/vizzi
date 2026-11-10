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
fn capture(root: &std::path::Path, files: &[(&str, &str)]) -> Arc<Snapshot> {
    let workspace = WorkspaceId::derive(&[b"semantic fixture"]);
    let mut parser = RustAnalyzer::new().unwrap();
    let mut docs: Vec<_> = files
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
    let manifest = std::fs::read(root.join("Cargo.toml")).unwrap();
    docs.push(Arc::new(
        Document::capture(
            workspace.clone(),
            "Cargo.toml".into(),
            &manifest,
            vec![],
            Classification::SourceOnly {
                language: Some("cargo".into()),
                provenance: Provenance::ExplicitScope {
                    scope: "Cargo.toml".into(),
                },
                reason: vizzi_core::policy::SourceOnlyReason::ProjectMetadata,
            },
        )
        .unwrap(),
    ));
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
    let snapshot = capture(
        root.path(),
        &[
            ("lib.rs", caller),
            ("worker.rs", worker),
            ("build.rs", &build),
        ],
    );
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
    let next = capture(
        root.path(),
        &[
            ("lib.rs", caller),
            ("worker.rs", &changed),
            ("build.rs", &build),
        ],
    );
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
    let snapshot = capture(root.path(), &[("lib.rs", caller), ("worker.rs", worker)]);
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

#[test]
#[ignore = "requires installed rust-analyzer; run explicitly during semantic-provider integration"]
fn overlapping_definition_requests_share_the_same_preparation() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("Cargo.toml"),"[package]\nname=\"semantic_overlap\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[lib]\npath=\"lib.rs\"\n").unwrap();
    let caller = "mod worker; trait Plugin { fn process(&mut self); } struct Service { meter: worker::Meter } impl Plugin for Service { fn process(&mut self) { self.meter.run(); } }";
    let worker = "pub struct Meter; impl Meter { pub fn run(&mut self) {} }";
    for (path, text) in [("lib.rs", caller), ("worker.rs", worker)] {
        std::fs::write(root.path().join(path), text).unwrap();
    }
    let snapshot = capture(root.path(), &[("lib.rs", caller), ("worker.rs", worker)]);
    let service = Arc::new(Service::new(
        root.path().into(),
        cache.path().into(),
        "rust-analyzer".into(),
    ));
    let first_service = service.clone();
    let first_snapshot = snapshot.clone();
    let first = std::thread::spawn(move || {
        first_service.resolve(first_snapshot.clone(), &call(&first_snapshot))
    });
    let start = std::time::Instant::now();
    while std::fs::read_dir(cache.path()).unwrap().next().is_none() {
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let second = service.resolve(snapshot.clone(), &call(&snapshot));
    let first = first.join().unwrap();
    assert!(first.is_ok(), "{first:?}");
    assert!(second.is_ok(), "{second:?}");
    assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 1);
}
#[test]
#[ignore = "requires installed rust-analyzer; local dependencies only, no network or builds"]
fn local_dependency_definitions_can_be_followed_across_captured_files() {
    let root = tempfile::tempdir().unwrap();
    let dependency = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("Cargo.toml"),format!("[package]\nname='consumer'\nversion='0.1.0'\nedition='2024'\n[lib]\npath='lib.rs'\n[dependencies]\nhelper={{path={:?}}}\n",dependency.path().to_str().unwrap())).unwrap();
    std::fs::write(
        dependency.path().join("Cargo.toml"),
        "[package]\nname='helper'\nversion='1.2.3'\nedition='2024'\n[lib]\npath='lib.rs'\n",
    )
    .unwrap();
    let caller = "pub fn entry(){ helper::start(); }";
    let callee = "pub fn start(){ leaf::finish(); } mod leaf;";
    std::fs::write(root.path().join("lib.rs"), caller).unwrap();
    std::fs::write(dependency.path().join("lib.rs"), callee).unwrap();
    std::fs::write(dependency.path().join("leaf.rs"), "pub fn finish(){}\n").unwrap();
    std::fs::write(
        dependency.path().join("build.rs"),
        format!(
            "fn main(){{std::fs::write({:?},b\"built\").unwrap();}}",
            dependency.path().join("BUILT").to_str().unwrap()
        ),
    )
    .unwrap();
    let snapshot = capture(root.path(), &[("lib.rs", caller)]);
    let index = Structure::new(&snapshot);
    let entry = index
        .functions("", "entry", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let lookup = index
        .call(&index.outgoing(&entry.id, "lib.rs", 0).unwrap().items[0].id)
        .unwrap();
    let service = Service::new(
        root.path().into(),
        cache.path().into(),
        "rust-analyzer".into(),
    );
    let answer = service.resolve(snapshot.clone(), &lookup).unwrap();
    assert_eq!(answer.locations.len(), 1, "{answer:?}");
    let loc = &answer.locations[0];
    let source = service
        .dependencies()
        .unwrap()
        .get(loc.capture.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(source.origin.label, "helper");
    assert_eq!(source.origin.version.as_deref(), Some("1.2.3"));
    let start = source
        .index
        .definition(&loc.path, byte_offset(callee, loc.position).unwrap())
        .unwrap();
    assert_eq!(start.name, "start");
    let site = &source.index.outgoing(&start.id, "lib.rs", 0).unwrap().items[0];
    let next = service
        .resolve(snapshot.clone(), &source.index.call(&site.id).unwrap())
        .unwrap();
    assert_eq!(next.locations.len(), 1);
    let leaf = &next.locations[0];
    assert_ne!(leaf.capture, loc.capture);
    let leaf_source = service
        .dependencies()
        .unwrap()
        .get(leaf.capture.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        leaf_source
            .index
            .definition(
                &leaf.path,
                byte_offset(leaf_source.doc().resident_source().unwrap(), leaf.position).unwrap()
            )
            .unwrap()
            .name,
        "finish"
    );
    std::fs::write(dependency.path().join("lib.rs"), "pub fn start(){}\n").unwrap();
    let stale = service
        .resolve(snapshot, &source.index.call(&site.id).unwrap())
        .unwrap();
    assert!(stale.locations.is_empty());
    assert!(stale.outside > 0);
    assert_eq!(source.doc().resident_source().unwrap(), callee);
    assert!(!dependency.path().join("BUILT").exists());
    assert!(!root.path().join("Cargo.lock").exists());
}
#[test]
#[ignore = "requires installed rust-analyzer and the registry sources already used to build Vizzi"]
fn cached_registry_definitions_are_captured_without_fetching_or_building() {
    let lock: toml::Table = include_str!("../../../Cargo.lock").parse().unwrap();
    let version = lock["package"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"].as_str() == Some("blake3"))
        .unwrap()["version"]
        .as_str()
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("Cargo.toml"),format!("[package]\nname='cached_consumer'\nversion='0.1.0'\nedition='2024'\n[lib]\npath='lib.rs'\n[dependencies]\nblake3='={version}'\n")).unwrap();
    let caller = "pub fn entry(){ let _=blake3::hash(b\"already local\"); }";
    std::fs::write(root.path().join("lib.rs"), caller).unwrap();
    let snapshot = capture(root.path(), &[("lib.rs", caller)]);
    let index = Structure::new(&snapshot);
    let entry = index
        .functions("", "entry", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let site = &index.outgoing(&entry.id, "lib.rs", 0).unwrap().items[0];
    let service = Service::new(
        root.path().into(),
        cache.path().into(),
        "rust-analyzer".into(),
    );
    let answer = service
        .resolve(snapshot, &index.call(&site.id).unwrap())
        .unwrap();
    assert_eq!(answer.locations.len(), 1, "{answer:?}");
    let loc = &answer.locations[0];
    let source = service
        .dependencies()
        .unwrap()
        .get(loc.capture.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(source.origin.label, "blake3");
    assert_eq!(source.origin.version.as_deref(), Some(version));
    assert_eq!(
        source
            .index
            .definition(
                &loc.path,
                byte_offset(source.doc().resident_source().unwrap(), loc.position).unwrap()
            )
            .unwrap()
            .name,
        "hash"
    );
    assert!(!root.path().join("Cargo.lock").exists());
}
#[test]
#[ignore = "requires installed rust-analyzer and rust-src; never installs missing sources"]
fn installed_standard_library_source_can_be_opened_as_an_independent_capture() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("Cargo.toml"),"[package]\nname='standard_consumer'\nversion='0.1.0'\nedition='2024'\n[lib]\npath='lib.rs'\n").unwrap();
    let caller = "pub fn entry(){ let _=core::ptr::null::<u8>(); }";
    std::fs::write(root.path().join("lib.rs"), caller).unwrap();
    let snapshot = capture(root.path(), &[("lib.rs", caller)]);
    let index = Structure::new(&snapshot);
    let entry = index
        .functions("", "entry", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let site = &index.outgoing(&entry.id, "lib.rs", 0).unwrap().items[0];
    let service = Service::new(
        root.path().into(),
        cache.path().into(),
        "rust-analyzer".into(),
    );
    let result = service
        .resolve(snapshot, &index.call(&site.id).unwrap())
        .unwrap();
    assert_eq!(result.locations.len(), 1, "{result:?}");
    let loc = &result.locations[0];
    let source = service
        .dependencies()
        .unwrap()
        .get(loc.capture.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(source.origin.label, "core");
    assert_eq!(
        source
            .index
            .definition(
                &loc.path,
                byte_offset(source.doc().resident_source().unwrap(), loc.position).unwrap()
            )
            .unwrap()
            .name,
        "null"
    );
}
