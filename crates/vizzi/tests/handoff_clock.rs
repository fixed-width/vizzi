//! Exact handoff timing through the same native interface, with Tokio's paused clock.
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use vizzi::{
    cache::Cache,
    engine::Engine,
    handoff::{Handoffs, Limits},
    host::{Host, HostOptions},
    source::Directory,
    structural::StructureHost,
};

fn module(temp: &tempfile::TempDir, limits: Limits) -> (Arc<Host>, Handoffs) {
    let root = temp.path().join("source");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("a.rs"), "fn first() {}\n").unwrap();
    let host = Arc::new(
        Host::prepare_with_options(
            Engine::new(
                Directory::open(&root).unwrap(),
                Cache::new(temp.path().join("cache")),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                reconcile_interval: Duration::from_secs(600),
                idle_maintenance: false,
            },
        )
        .unwrap(),
    );
    let handoffs = Handoffs::with_limits(
        host.clone(),
        Arc::new(StructureHost::new(host.clone())),
        limits,
    );
    (host, handoffs)
}
fn request(scope: &str) -> vizzi::handoff::Request {
    serde_json::from_value(json!({"scope":scope,"sequence":1,"action":{"kind":"start","path":"a.rs","text":"fn first() {}\n","target":{"kind":"file","line":1}}})).unwrap()
}

#[tokio::test(start_paused = true)]
async fn handoff_an_idle_scope_expires_at_its_exact_deadline() {
    let temp = tempfile::tempdir().unwrap();
    let (host, handoffs) = module(
        &temp,
        Limits {
            windows: 1,
            window_idle: Duration::from_secs(5),
            ..Limits::default()
        },
    );
    let scope = handoffs.attach_window(host.root()).unwrap();
    tokio::time::advance(Duration::from_secs(5)).await;
    let next = handoffs.attach_window(host.root()).unwrap();
    assert_ne!(next, scope);
    let result: Value = serde_json::to_value(handoffs.handoff(request(&scope)).await).unwrap();
    assert_eq!(result["code"], "scope_expired");
}

#[tokio::test(start_paused = true)]
async fn handoff_an_already_expired_request_never_waits_or_starts_capture() {
    let temp = tempfile::tempdir().unwrap();
    let (host, handoffs) = module(
        &temp,
        Limits {
            deadline: Duration::ZERO,
            observe_wait: Duration::ZERO,
            ..Limits::default()
        },
    );
    let scope = handoffs.attach_window(host.root()).unwrap();
    let result: Value = serde_json::to_value(handoffs.handoff(request(&scope)).await).unwrap();
    assert_eq!(result["state"], "expired");
    tokio::task::yield_now().await;
    assert_eq!(
        host.application().status().readiness,
        vizzi::application::Readiness::NotStarted
    );
}

#[tokio::test(start_paused = true)]
async fn handoff_a_worker_delayed_until_the_exact_deadline_does_not_activate_shared_capture() {
    let temp = tempfile::tempdir().unwrap();
    let (host, handoffs) = module(
        &temp,
        Limits {
            deadline: Duration::from_secs(5),
            observe_wait: Duration::ZERO,
            ..Limits::default()
        },
    );
    let scope = handoffs.attach_window(host.root()).unwrap();
    let first: Value = serde_json::to_value(handoffs.handoff(request(&scope)).await).unwrap();
    assert_eq!(first["state"], "pending");
    tokio::time::advance(Duration::from_secs(5)).await;
    let result: Value = serde_json::to_value(handoffs.handoff(request(&scope)).await).unwrap();
    assert_eq!(result["state"], "expired");
    assert_eq!(
        host.application().status().readiness,
        vizzi::application::Readiness::NotStarted
    );
}
