//! Editor call positions bind to saved captured source before reaching the workspace.
use serde_json::json;
use std::{sync::Arc, time::Duration};
use vizzi::{
    cache::Cache,
    engine::Engine,
    host::{Host, HostOptions},
    source::Directory,
    structural::{StructureHost, editor::Request},
};

#[tokio::test]
async fn editor_http_accepts_saved_files_beyond_the_generic_body_limit_and_bounds_text() {
    use vizzi::{
        companion::Companion,
        editor::VsCode,
        review_application::ReviewApplication,
        review_source::CaptureLimits,
        server::{diagnostic_request, router_with_companion},
    };
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let prefix = "def run(): leaf()\ndef leaf(): pass\n#";
    let text = format!("{prefix}{}", "x".repeat(4 * 1024 * 1024 - prefix.len()));
    std::fs::write(root.path().join("a.py"), &text).unwrap();
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
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let companion = Arc::new(
        Companion::new(
            host.clone(),
            None,
            Arc::new(VsCode::new("not-invoked".into())),
        )
        .unwrap(),
    );
    let reviews =
        Arc::new(ReviewApplication::start(host.clone(), CaptureLimits::default()).unwrap());
    let router = router_with_companion(host, reviews, companion);
    for (body, status) in [
        (
            json!({"snapshot":snapshot,"query":{"kind":"position","path":"a.py","text":text,"position":{"line":0,"character":12}}}),
            200,
        ),
        (
            json!({"snapshot":snapshot,"query":{"kind":"position","path":"a.py","text":"x".repeat(4*1024*1024+1),"position":{"line":0,"character":12}}}),
            400,
        ),
        (
            json!({"snapshot":snapshot,"query":{"kind":"invocation","call":"unknown","executable":"code"}}),
            422,
        ),
    ] {
        let response = diagnostic_request(
            router.clone(),
            json!(1),
            "POST",
            "/api/editor/call",
            Some(body),
        )
        .await
        .unwrap();
        assert_eq!(response.status, status, "{}", response.body);
    }
}

#[test]
fn editor_positions_select_the_exact_call_and_reject_mismatched_source() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let text = "# 😀\ndef leaf(): pass\ndef branch(): pass\ndef run(): leaf('😀'); branch()\n";
    std::fs::write(root.path().join("a.py"), text).unwrap();
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
    let app = StructureHost::new(host.clone());
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let request:Request=serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"position","path":"a.py","text":text,"position":{"line":3,"character":24}}})).unwrap();
    let answer = app.editor_call(&request).unwrap();
    assert_eq!(answer.caller.name, "run");
    assert_eq!(answer.invocation.name.as_deref(), Some("branch"));
    let again: Request = serde_json::from_value(
        json!({"snapshot":snapshot,"query":{"kind":"invocation","call":answer.invocation.id}}),
    )
    .unwrap();
    assert_eq!(
        app.editor_call(&again).unwrap().invocation.id,
        answer.invocation.id
    );
    for (path, body, line, character) in [
        ("../outside.py", text, 3, 24),
        ("/a.py", text, 3, 24),
        ("a.py", "older saved text", 3, 24),
        ("a.py", text, 0, 4),
        ("a.py", text, 1, 4),
    ] {
        let request:Request=serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"position","path":path,"text":body,"position":{"line":line,"character":character}}})).unwrap();
        assert!(
            app.editor_call(&request).is_err(),
            "{path}:{line}:{character}"
        );
    }
    let stale:Request=serde_json::from_value(json!({"snapshot":"0".repeat(64),"query":{"kind":"invocation","call":answer.invocation.id}})).unwrap();
    assert!(app.editor_call(&stale).is_err());
    let unknown: Request = serde_json::from_value(
        json!({"snapshot":snapshot,"query":{"kind":"invocation","call":"0".repeat(64)}}),
    )
    .unwrap();
    assert!(app.editor_call(&unknown).is_err());
    let changed = "def replacement(): pass\ndef fresh(): replacement()\n";
    std::fs::write(root.path().join("a.py"), changed).unwrap();
    host.request(vizzi::source::CaptureScope::Reconcile);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while host.application().snapshot().unwrap().id().as_str() == snapshot {
        assert!(
            std::time::Instant::now() < deadline,
            "saved source did not refresh"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let current = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let request:Request=serde_json::from_value(json!({"snapshot":current,"query":{"kind":"position","path":"a.py","text":changed,"position":{"line":1,"character":14}}})).unwrap();
    let answer = app.editor_call(&request).unwrap();
    assert_eq!(answer.caller.name, "fresh");
    assert_eq!(answer.invocation.name.as_deref(), Some("replacement"));
}
