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
