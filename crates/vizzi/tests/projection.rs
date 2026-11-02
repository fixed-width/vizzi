//! Finite HTTP/stdio windows bind to captured snapshots and exact document versions.

use serde_json::{Value, json};
use std::sync::Arc;
use vizzi::{application::Application, cache::Cache, engine::Engine, source::Directory};

#[tokio::test]
async fn pinned_windows_reject_wrong_versions_and_keep_old_source_after_edits() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn first() {}\n").unwrap();
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().into()),
        1,
    )
    .unwrap();
    let app = Arc::new(Application::new("window fixture".into()).unwrap());
    let ticket = app.request();
    let first = engine.refresh(None, &|| false).unwrap().unwrap();
    let old = first.snapshot.clone();
    app.publish(ticket, first);
    let router = vizzi::server::router(app.clone());
    let response =
        vizzi::server::diagnostic(router.clone(), json!(1), "/api/view?projection=summary")
            .await
            .unwrap();
    assert_eq!(response.status, 200);
    let view: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(view["schema"], 3);
    assert!(view["source"]["text"].is_null());
    let base = format!(
        "/api/window?snapshot={}&file={}",
        old.id().as_str(),
        old.documents()[0].id().as_str()
    );
    let uri = format!("{base}&version={}", old.documents()[0].version().as_str());
    assert_eq!(
        vizzi::server::diagnostic(router.clone(), json!(2), &base)
            .await
            .unwrap()
            .status,
        400
    );
    assert_eq!(
        vizzi::server::diagnostic(router.clone(), json!(2), &format!("{base}&version=wrong"))
            .await
            .unwrap()
            .status,
        409
    );
    std::fs::write(root.path().join("a.rs"), "fn changed() {}\n").unwrap();
    let ticket = app.request();
    app.publish(
        ticket,
        engine.refresh(Some(&old), &|| false).unwrap().unwrap(),
    );
    let response = vizzi::server::diagnostic(router.clone(), json!(3), &uri)
        .await
        .unwrap();
    assert_eq!(response.status, 200);
    let data: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(data["lines"][0]["text"], "fn first() {}");
    assert_eq!(data["snapshot"], view["snapshot"]);
    assert_eq!(
        vizzi::server::diagnostic(
            router.clone(),
            json!(4),
            "/api/window?snapshot=expired&file=missing&version=wrong"
        )
        .await
        .unwrap()
        .status,
        409
    );
    assert_eq!(
        vizzi::server::diagnostic(router, json!(4), &format!("{uri}&line_start=-1"))
            .await
            .unwrap()
            .status,
        400
    );
}
