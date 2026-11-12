//! Semantic selection joins provider positions to native captured declarations.
use serde_json::json;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use vizzi::{
    cache::Cache,
    definitions::{Answer, Location, Position, Provider},
    engine::Engine,
    host::{Host, HostOptions},
    source::Directory,
    structural::{DefinitionRequest, Request, StructureHost},
};
use vizzi_core::{model::Snapshot, structure::CallLookup};
struct Fixed {
    answer: Answer,
}
impl Provider for Fixed {
    fn resolve(&self, _: Arc<Snapshot>, _: &CallLookup) -> Result<Answer, String> {
        Ok(self.answer.clone())
    }
}
fn fixture() -> (tempfile::TempDir, tempfile::TempDir, Arc<Host>) {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.rs"), "fn start() { renamed(); }").unwrap();
    std::fs::write(root.path().join("target.rs"), "pub fn actual() {}\n").unwrap();
    let host = Arc::new(
        Host::start_with_options(
            Engine::new(
                Directory::open(root.path()).unwrap(),
                Cache::new(cache.path().join("analysis")),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                reconcile_interval: Duration::from_secs(3600),
                idle_maintenance: false,
            },
        )
        .unwrap(),
    );
    (root, cache, host)
}
fn request(app: &StructureHost, host: &Host) -> DefinitionRequest {
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let query:Request=serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"main.rs","search":"start","offset":0}})).unwrap();
    let symbols = app.query(&query).unwrap();
    let symbol = &symbols["items"][0]["symbol"]["id"];
    let query:Request=serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"calls","symbol":symbol,"area":"main.rs","offset":0}})).unwrap();
    let calls = app.query(&query).unwrap();
    DefinitionRequest {
        snapshot,
        call: calls["items"][0]["id"].as_str().unwrap().into(),
    }
}
fn answer(outside: usize) -> Answer {
    Answer {
        locations: vec![Location {
            capture: None,
            path: "target.rs".into(),
            position: Position {
                line: 0,
                character: 7,
            },
        }],
        outside,
        context: "a".repeat(64),
    }
}
#[test]
fn semantic_alias_targets_open_by_definition_position_and_preserve_native_witnesses() {
    let (_root, _cache, host) = fixture();
    let app = StructureHost::with_definitions(host.clone(), Arc::new(Fixed { answer: answer(0) }));
    let req = request(&app, &host);
    let result = serde_json::to_value(app.definition(&req).unwrap()).unwrap();
    assert_eq!(result["state"], "unique");
    assert_eq!(result["invocation"]["spelling"], "renamed");
    assert_eq!(result["invocation"]["candidates"][0]["name"], "actual");
    assert_eq!(
        result["invocation"]["binding"],
        "language_server_definition"
    );
    assert_eq!(result["invocation"]["candidate_count"], 1);
    assert_eq!(result["invocation"]["semantic_context"], "a".repeat(64));
}
struct External {
    catalog: vizzi::dependency_sources::Catalog,
    capture: String,
}
impl Provider for External {
    fn dependencies(&self) -> Option<&vizzi::dependency_sources::Catalog> {
        Some(&self.catalog)
    }
    fn resolve(&self, _: Arc<Snapshot>, call: &CallLookup) -> Result<Answer, String> {
        Ok(Answer {
            locations: vec![Location {
                capture: Some(self.capture.clone()),
                path: "target.rs".into(),
                position: Position {
                    line: if call.invocation.name.as_deref() == Some("leaf") {
                        1
                    } else {
                        0
                    },
                    character: 7,
                },
            }],
            outside: 0,
            context: "b".repeat(64),
        })
    }
}
#[test]
fn dependency_source_windows_calls_refresh_and_editor_verification_keep_their_own_identity() {
    let (_root, cache, host) = fixture();
    let dependency = tempfile::tempdir().unwrap();
    std::fs::write(
        dependency.path().join("target.rs"),
        "pub fn actual(){ leaf(); }\npub fn leaf(){}\n",
    )
    .unwrap();
    let catalog = vizzi::dependency_sources::Catalog::new(cache.path().join("dependencies.json"));
    let captured = catalog
        .capture(vizzi::dependency_sources::Input {
            root: dependency.path().canonicalize().unwrap(),
            path: "target.rs".into(),
            language: "rust".into(),
            package: "helper".into(),
            version: Some("1.2.3".into()),
            configuration: "a".repeat(64),
        })
        .unwrap();
    let app = StructureHost::with_definitions(
        host.clone(),
        Arc::new(External {
            catalog,
            capture: captured.origin.capture.clone(),
        }),
    );
    let req = request(&app, &host);
    let result = serde_json::to_value(app.definition(&req).unwrap()).unwrap();
    assert_eq!(result["state"], "unique");
    let target = &result["invocation"]["candidates"][0];
    assert_eq!(target["name"], "actual");
    assert_eq!(target["origin"]["label"], "helper");
    assert_ne!(
        target["source"]["file"],
        host.application()
            .snapshot()
            .unwrap()
            .file_by_path("target.rs")
            .unwrap()
            .id()
            .as_str()
    );
    let symbol = target["id"].as_str().unwrap();
    let file = target["source"]["file"].as_str().unwrap();
    let version = target["source"]["version"].as_str().unwrap();
    let calls:Request=serde_json::from_value(json!({"snapshot":req.snapshot,"query":{"kind":"calls","symbol":symbol,"area":"main.rs","offset":0}})).unwrap();
    let calls = app.query(&calls).unwrap();
    assert_eq!(calls["snapshot"], req.snapshot);
    let nested = app
        .definition(&DefinitionRequest {
            snapshot: req.snapshot.clone(),
            call: calls["items"][0]["id"].as_str().unwrap().into(),
        })
        .unwrap();
    assert_eq!(nested.invocation.candidates[0].name, "leaf");
    assert!(nested.invocation.candidates[0].origin.is_some());
    let open = vizzi::companion::OpenSource {
        snapshot: captured.origin.capture.clone(),
        file: file.into(),
        version: version.into(),
        line: 1,
    };
    assert_eq!(
        app.dependency_location(&open).unwrap().path,
        dependency.path().canonicalize().unwrap().join("target.rs")
    );
    let refresh:vizzi::structural::live::Request=serde_json::from_value(json!({"snapshot":req.snapshot,"views":[{"id":"v1","capture":captured.origin.capture,"symbol":symbol,"tracking":target["tracking"],"version":version,"file":file,"context":target["source"]["context"],"area":"main.rs","section_offset":0,"anchor":null}],"links":[]})).unwrap();
    assert_eq!(
        app.refresh(&refresh).unwrap()["views"][0]["symbol"]["origin"]["label"],
        "helper"
    );
    std::fs::write(dependency.path().join("target.rs"), "fn replacement(){}\n").unwrap();
    assert!(app.dependency_location(&open).is_err());
    let window = app
        .dependency_window(&captured.origin.capture, file, version, &Default::default())
        .unwrap();
    assert_eq!(window["snapshot"], captured.origin.capture);
    assert!(
        window["lines"][0]["text"]
            .as_str()
            .unwrap()
            .contains("actual")
    );
    assert!(
        app.dependency_window(
            &captured.origin.capture,
            &"0".repeat(64),
            version,
            &Default::default()
        )
        .is_err()
    );
}
#[test]
fn a_known_external_destination_prevents_a_false_unique_result() {
    let (_root, _cache, host) = fixture();
    let app = StructureHost::with_definitions(host.clone(), Arc::new(Fixed { answer: answer(1) }));
    let req = request(&app, &host);
    let result = serde_json::to_value(app.definition(&req).unwrap()).unwrap();
    assert_eq!(result["state"], "ambiguous");
    assert_eq!(result["invocation"]["candidate_count"], 2);
    assert_eq!(
        result["invocation"]["candidates"].as_array().unwrap().len(),
        1
    );
    let app = StructureHost::with_definitions(
        host.clone(),
        Arc::new(Fixed {
            answer: Answer {
                locations: vec![],
                outside: 1,
                context: "a".repeat(64),
            },
        }),
    );
    let req = request(&app, &host);
    let result = serde_json::to_value(app.definition(&req).unwrap()).unwrap();
    assert_eq!(result["state"], "outside");
    assert!(
        result["invocation"]["candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
struct Superseding {
    host: Arc<Host>,
    root: std::path::PathBuf,
}
impl Provider for Superseding {
    fn resolve(&self, snapshot: Arc<Snapshot>, _: &CallLookup) -> Result<Answer, String> {
        std::fs::write(self.root.join("main.rs"), "fn replacement() {}\n").unwrap();
        self.host.request(vizzi::source::CaptureScope::Reconcile);
        let start = Instant::now();
        while self.host.application().snapshot().unwrap().id() == snapshot.id() {
            assert!(start.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(answer(0))
    }
}
#[test]
fn a_provider_reply_cannot_act_after_the_visible_capture_changes() {
    let (root, _cache, host) = fixture();
    let app = StructureHost::with_definitions(
        host.clone(),
        Arc::new(Superseding {
            host: host.clone(),
            root: root.path().into(),
        }),
    );
    let req = request(&app, &host);
    assert!(matches!(
        app.definition(&req),
        Err(vizzi::companion::CompanionError::Conflict(_))
    ));
}
