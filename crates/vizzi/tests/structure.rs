//! Native structural operations preserve current captured source bindings.
use serde_json::json;
use std::sync::Arc;
use vizzi::{
    cache::Cache,
    engine::Engine,
    host::{Host, HostOptions},
    source::Directory,
    structural::{Request, StructureHost},
};
#[test]
fn native_operations_bind_symbols_to_the_current_capture_and_refuse_old_requests() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("pipe")).unwrap();
    std::fs::write(
        root.path().join("pipe/main.rs"),
        "fn begin() { relay(); } fn relay() {}\n",
    )
    .unwrap();
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
                reconcile_interval: std::time::Duration::from_secs(3600),
                idle_maintenance: false,
            },
        )
        .unwrap(),
    );
    let app = StructureHost::new(host.clone());
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let request:Request=serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"pipe","search":"begin","offset":0}})).unwrap();
    let result = app.query(&request).unwrap();
    assert_eq!(result["matching"], 1);
    let symbol = &result["items"][0]["symbol"];
    let native = host.application().snapshot().unwrap();
    let doc = native
        .file(symbol["source"]["file"].as_str().unwrap())
        .unwrap();
    assert_eq!(symbol["source"]["version"], doc.version().as_str());
    let calls:Request=serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"calls","symbol":symbol["id"],"area":"pipe","offset":0}})).unwrap();
    assert_eq!(app.query(&calls).unwrap()["items"][0]["spelling"], "relay");
    std::fs::write(root.path().join("pipe/main.rs"), "fn changed() {}\n").unwrap();
    host.request(vizzi::source::CaptureScope::Reconcile);
    let start = std::time::Instant::now();
    while host.application().snapshot().unwrap().id().as_str() == snapshot {
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(app.query(&calls).is_err());
    let current_snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let fresh:Request=serde_json::from_value(json!({"snapshot":current_snapshot,"query":{"kind":"inventory","area":"pipe","search":"changed","offset":0}})).unwrap();
    let current = app.query(&fresh).unwrap();
    assert_eq!(current["snapshot"], current_snapshot);
    assert_eq!(current["matching"], 1);
    assert_eq!(current["items"][0]["symbol"]["name"], "changed");
    assert!(serde_json::from_value::<Request>(json!({"snapshot":snapshot,"query":{"kind":"areas","search":"","offset":0,"execute":"code"}})).is_err());
}

#[tokio::test]
async fn structural_http_caps_schema_and_source_conflicts_are_bounded() {
    use vizzi::{
        companion::Companion,
        editor::VsCode,
        review_application::ReviewApplication,
        review_source::CaptureLimits,
        server::{diagnostic, diagnostic_request, router_with_companion},
    };
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("main.rs"),
        "fn start() { run(); } fn run() {}\n",
    )
    .unwrap();
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
                reconcile_interval: std::time::Duration::from_secs(3600),
                idle_maintenance: false,
            },
        )
        .unwrap(),
    );
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let app = Arc::new(
        Companion::new(
            host.clone(),
            None,
            Arc::new(VsCode::new("not-invoked".into())),
        )
        .unwrap(),
    );
    let reviews =
        Arc::new(ReviewApplication::start(host.clone(), CaptureLimits::default()).unwrap());
    let router = router_with_companion(host, reviews, app);
    let caps = diagnostic(router.clone(), json!(1), "/api/structure")
        .await
        .unwrap();
    assert_eq!(caps.status, 200);
    assert!(caps.body.contains("candidate_only"));
    for (body, status) in [
        (
            json!({"snapshot":snapshot,"query":{"kind":"areas","search":"","offset":0}}),
            200,
        ),
        (
            json!({"snapshot":"expired","query":{"kind":"areas","search":"","offset":0}}),
            409,
        ),
        (
            json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"../escape","search":"","offset":0}}),
            400,
        ),
        (
            json!({"snapshot":snapshot,"query":{"kind":"areas","search":"","offset":0,"executable":"code"}}),
            422,
        ),
        (
            json!({"snapshot":snapshot,"query":{"kind":"areas","search":"x".repeat(20000),"offset":0}}),
            413,
        ),
    ] {
        let response = diagnostic_request(
            router.clone(),
            json!(1),
            "POST",
            "/api/structure",
            Some(body),
        )
        .await
        .unwrap();
        assert_eq!(response.status, status, "{}", response.body);
    }
}

#[test]
fn function_search_requests_rank_and_filter_in_the_native_adapter() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let source = "struct process; struct LufsMeter; impl LufsMeter { fn process_sample() {} } fn process_sample() {} fn process() {}";
    std::fs::write(root.path().join("lib.rs"), source).unwrap();
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
                reconcile_interval: std::time::Duration::from_secs(3600),
                idle_maintenance: false,
            },
        )
        .unwrap(),
    );
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let app = StructureHost::new(host);
    let req:Request=serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"","search":"process","offset":0,"functions_only":true}})).unwrap();
    let results = app.query(&req).unwrap();
    assert_eq!(results["matching"], 3);
    assert_eq!(results["items"][0]["symbol"]["name"], "process");
    assert!(
        results["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["symbol"]["kind"] == "function")
    );
    let req:Request=serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"","search":"LufsMeter::process_sample","offset":0,"functions_only":true}})).unwrap();
    let results = app.query(&req).unwrap();
    assert_eq!(results["matching"], 1);
    assert_eq!(results["items"][0]["symbol"]["name"], "process_sample");
}
