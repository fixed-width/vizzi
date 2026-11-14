//! Behaviour at the editor-neutral captured-source handoff interface.
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;
use vizzi::{
    cache::Cache,
    engine::Engine,
    handoff::{Handoffs, Limits, Request},
    host::{Host, HostOptions},
    source::Directory,
    structural::StructureHost,
};
use vizzi_core::{
    analysis::{Analyzer, LanguageAdapter, Languages},
    model::{Analysis, Context, ContextId, ModelError},
    rust::RustAnalyzer,
};

struct Controlled {
    released: Arc<AtomicBool>,
    entered: Arc<AtomicUsize>,
    fail: Arc<AtomicBool>,
}
struct Parser {
    released: Arc<AtomicBool>,
    entered: Arc<AtomicUsize>,
    inner: RustAnalyzer,
    context: Context,
    fail: Arc<AtomicBool>,
}
impl LanguageAdapter for Controlled {
    fn context(&self) -> Context {
        Context {
            id: ContextId::derive(&[b"handoff-controlled"]),
            language: "rust".into(),
            description: "Controlled handoff fixture".into(),
        }
    }
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        Ok(Box::new(Parser {
            released: self.released.clone(),
            entered: self.entered.clone(),
            inner: RustAnalyzer::new()?,
            context: self.context(),
            fail: self.fail.clone(),
        }))
    }
}
impl Analyzer for Parser {
    fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(ModelError("Controlled capture failed".into()));
        }
        let mut analysis = self.inner.analyze(bytes)?;
        analysis.context = self.context.clone();
        Ok(analysis)
    }
    fn analyze_cancellable(
        &mut self,
        bytes: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Analysis>, ModelError> {
        self.entered.fetch_add(1, Ordering::SeqCst);
        while !self.released.load(Ordering::SeqCst) && !cancelled() {
            std::thread::sleep(Duration::from_millis(1));
        }
        if cancelled() {
            Ok(None)
        } else {
            self.analyze(bytes).map(Some)
        }
    }
}
struct Fixture {
    _temp: tempfile::TempDir,
    root: std::path::PathBuf,
    host: Arc<Host>,
    handoffs: Handoffs,
    released: Arc<AtomicBool>,
    entered: Arc<AtomicUsize>,
    fail: Arc<AtomicBool>,
}
const TEXT: &str = "pub fn caller() { leaf(); }\nfn leaf() {}\n";
fn fixture(released: bool, limits: Limits) -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("a.rs"), TEXT).unwrap();
    let released = Arc::new(AtomicBool::new(released));
    let entered = Arc::new(AtomicUsize::new(0));
    let fail = Arc::new(AtomicBool::new(false));
    let languages = Arc::new(
        Languages::new(vec![Arc::new(Controlled {
            released: released.clone(),
            entered: entered.clone(),
            fail: fail.clone(),
        })])
        .unwrap(),
    );
    let directory = Directory::with_languages(&root, languages).unwrap();
    let root = directory.root().to_owned();
    let engine = Engine::new(directory, Cache::new(temp.path().join("cache")), 1).unwrap();
    let host = Arc::new(
        Host::prepare_with_options(
            engine,
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
    Fixture {
        _temp: temp,
        root,
        host,
        handoffs,
        released,
        entered,
        fail,
    }
}
fn start(scope: &str, sequence: u64, text: &str, target: Value) -> Request {
    serde_json::from_value(json!({"scope":scope,"sequence":sequence,"action":{"kind":"start","path":"a.rs","text":text,"target":target}})).unwrap()
}
async fn value(f: &Fixture, request: Request) -> Value {
    serde_json::to_value(f.handoffs.handoff(request).await).unwrap()
}
async fn finish(f: &Fixture, scope: &str, sequence: u64, mut result: Value) -> Value {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while result["state"] == "pending" {
        assert!(std::time::Instant::now() < deadline, "{result}");
        result=value(f,serde_json::from_value(json!({"scope":scope,"sequence":sequence,"action":{"kind":"continue","handle":result["handle"]}})).unwrap()).await;
    }
    result
}
async fn entered(f: &Fixture, n: usize) {
    let until = std::time::Instant::now() + Duration::from_secs(3);
    while f.entered.load(Ordering::SeqCst) < n {
        assert!(std::time::Instant::now() < until);
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[tokio::test]
async fn handoff_cold_capture_waits_without_blocking_other_scopes_and_superseded_work_is_silent() {
    let f = fixture(false, Limits::default());
    let one = f.handoffs.attach_window(&f.root).unwrap();
    let two = f.handoffs.attach_window(&f.root).unwrap();
    let first = value(&f, start(&one, 1, TEXT, json!({"kind":"file","line":2}))).await;
    assert_eq!(first["state"], "pending");
    entered(&f, 1).await;
    let independent = value(&f, start(&two, 1, TEXT, json!({"kind":"file","line":1}))).await;
    assert_eq!(independent["state"], "pending");
    let newer = value(
        &f,
        start(
            &one,
            2,
            TEXT,
            json!({"kind":"call","position":{"line":0,"character":18}}),
        ),
    )
    .await;
    let old = value(
        &f,
        serde_json::from_value(
            json!({"scope":one,"sequence":1,"action":{"kind":"continue","handle":first["handle"]}}),
        )
        .unwrap(),
    )
    .await;
    assert_eq!(old["state"], "superseded");
    f.released.store(true, Ordering::SeqCst);
    let call = finish(&f, &one, 2, newer).await;
    assert_eq!(call["state"], "ready", "{call}");
    assert_eq!(call["destination"]["kind"], "call");
    let file = finish(&f, &two, 1, independent).await;
    assert_eq!(file["state"], "ready");
    assert_eq!(file["destination"]["line"], 1);
    assert_eq!(
        f.entered.load(Ordering::SeqCst),
        1,
        "handoffs join the same capture"
    );
}

#[tokio::test]
async fn handoff_stale_capture_requests_one_refresh_and_keeps_the_original_deadline() {
    let f = fixture(true, Limits::default());
    let scope = f.handoffs.attach_window(&f.root).unwrap();
    let initial = value(&f, start(&scope, 1, TEXT, json!({"kind":"file","line":1}))).await;
    let first = finish(&f, &scope, 1, initial).await;
    assert_eq!(first["state"], "ready");
    f.released.store(false, Ordering::SeqCst);
    let changed = "pub fn changed() {}\n";
    std::fs::write(f.root.join("a.rs"), changed).unwrap();
    let waiting = value(
        &f,
        start(&scope, 2, changed, json!({"kind":"file","line":1})),
    )
    .await;
    assert_eq!(waiting["state"], "pending");
    entered(&f, 2).await;
    let ticket = f.host.application().desired();
    for _ in 0..3 {
        let replay = value(
            &f,
            start(&scope, 2, changed, json!({"kind":"file","line":1})),
        )
        .await;
        assert_eq!(replay["handle"], waiting["handle"]);
    }
    assert_eq!(f.host.application().desired(), ticket);
    f.released.store(true, Ordering::SeqCst);
    let result = finish(&f, &scope, 2, waiting).await;
    assert_eq!(result["state"], "ready");
    assert_ne!(
        result["destination"]["version"],
        first["destination"]["version"]
    );
}

#[tokio::test]
async fn handoff_cancellation_is_sequence_scoped_and_never_cancels_shared_capture() {
    let f = fixture(false, Limits::default());
    let scope = f.handoffs.attach_window(&f.root).unwrap();
    let first = value(&f, start(&scope, 1, TEXT, json!({"kind":"file","line":1}))).await;
    entered(&f, 1).await;
    let second = value(&f, start(&scope, 2, TEXT, json!({"kind":"file","line":2}))).await;
    let cancel = serde_json::from_value(json!({"scope":scope,"sequence":1})).unwrap();
    assert_eq!(
        serde_json::to_value(f.handoffs.cancel(cancel)).unwrap()["state"],
        "superseded"
    );
    assert!(f.host.application().status().pending);
    f.released.store(true, Ordering::SeqCst);
    assert_eq!(finish(&f, &scope, 2, second).await["state"], "ready");
    assert_eq!(first["state"], "pending");
    let future = serde_json::from_value(json!({"scope":scope,"sequence":3})).unwrap();
    assert_eq!(
        serde_json::to_value(f.handoffs.cancel(future)).unwrap()["state"],
        "cancelled"
    );
    assert_eq!(
        value(&f, start(&scope, 3, TEXT, json!({"kind":"file","line":1}))).await["state"],
        "cancelled"
    );
}

#[tokio::test]
async fn handoff_expired_handoff_cannot_be_extended_by_polling_or_replay() {
    let f = fixture(
        false,
        Limits {
            deadline: Duration::from_millis(120),
            observe_wait: Duration::from_millis(5),
            ..Limits::default()
        },
    );
    let scope = f.handoffs.attach_window(&f.root).unwrap();
    let waiting = value(&f, start(&scope, 1, TEXT, json!({"kind":"file","line":1}))).await;
    entered(&f, 1).await;
    tokio::time::sleep(Duration::from_millis(140)).await;
    assert_eq!(
        value(&f, start(&scope, 1, TEXT, json!({"kind":"file","line":1}))).await["state"],
        "expired"
    );
    assert_eq!(finish(&f, &scope, 1, waiting).await["state"], "expired");
    assert!(f.host.application().status().pending);
}

#[tokio::test]
async fn handoff_scope_capacity_expiry_and_wrong_roots_are_explicit() {
    let f = fixture(
        true,
        Limits {
            windows: 1,
            window_idle: Duration::from_millis(50),
            ..Limits::default()
        },
    );
    assert!(
        f.handoffs
            .attach_window(std::path::Path::new("/other"))
            .is_err()
    );
    let scope = f.handoffs.attach_window(&f.root).unwrap();
    assert!(f.handoffs.attach_window(&f.root).is_err());
    tokio::time::sleep(Duration::from_millis(70)).await;
    let newer = f.handoffs.attach_window(&f.root).unwrap();
    assert_ne!(scope, newer);
    assert_eq!(
        value(&f, start(&scope, 1, TEXT, json!({"kind":"file","line":1}))).await["code"],
        "scope_expired"
    );
}

#[tokio::test]
async fn handoff_invalid_source_targets_and_reused_sequences_are_refused() {
    let f = fixture(true, Limits::default());
    let scope = f.handoffs.attach_window(&f.root).unwrap();
    let first = value(&f, start(&scope, 1, TEXT, json!({"kind":"file","line":1}))).await;
    assert_eq!(finish(&f, &scope, 1, first).await["state"], "ready");
    let reused = value(&f, start(&scope, 1, TEXT, json!({"kind":"file","line":2}))).await;
    assert_eq!(reused["code"], "sequence_reused");
    let bad = value(
        &f,
        start(
            &scope,
            2,
            TEXT,
            json!({"kind":"call","position":{"line":0,"character":0}}),
        ),
    )
    .await;
    let bad = finish(&f, &scope, 2, bad).await;
    assert_eq!(bad["state"], "refused");
    let changed = value(
        &f,
        start(&scope, 3, "not saved", json!({"kind":"file","line":1})),
    )
    .await;
    let changed = finish(&f, &scope, 3, changed).await;
    assert_eq!(changed["code"], "source_changed");
    let path:Request=serde_json::from_value(json!({"scope":scope,"sequence":4,"action":{"kind":"start","path":"../a.rs","text":TEXT,"target":{"kind":"file","line":1}}})).unwrap();
    assert_eq!(value(&f, path).await["code"], "invalid_source");
    for (sequence, length, code) in [(5, 4096, "not_captured"), (6, 4097, "invalid_source")] {
        let path = "x".repeat(length);
        let request=serde_json::from_value(json!({"scope":scope,"sequence":sequence,"action":{"kind":"start","path":path,"text":TEXT,"target":{"kind":"file","line":1}}})).unwrap();
        let result = value(&f, request).await;
        assert_eq!(finish(&f, &scope, sequence, result).await["code"], code);
    }
}

#[tokio::test]
async fn handoff_late_starts_invalid_handles_and_closed_updates_cannot_publish_a_destination() {
    let f = fixture(false, Limits::default());
    let scope = f.handoffs.attach_window(&f.root).unwrap();
    let current = value(&f, start(&scope, 2, TEXT, json!({"kind":"file","line":1}))).await;
    entered(&f, 1).await;
    let future = serde_json::from_value(
        json!({"scope":scope,"sequence":3,"action":{"kind":"continue","handle":current["handle"]}}),
    )
    .unwrap();
    assert_eq!(value(&f, future).await["code"], "invalid_request");
    assert_eq!(
        value(&f, start(&scope, 1, TEXT, json!({"kind":"file","line":2}))).await["state"],
        "superseded"
    );
    let bad = serde_json::from_value(
        json!({"scope":scope,"sequence":2,"action":{"kind":"continue","handle":"wrong"}}),
    )
    .unwrap();
    assert_eq!(value(&f, bad).await["code"], "invalid_request");
    f.host.application().close_updates();
    assert_eq!(finish(&f, &scope, 2, current).await["code"], "unavailable");
}

#[tokio::test]
async fn handoff_finite_wire_exchanges_keep_cancellation_and_status_available_while_capture_waits()
{
    use vizzi::{
        companion::Companion,
        editor::VsCode,
        review_application::ReviewApplication,
        review_source::CaptureLimits,
        server::{diagnostic_request, router_with_companion},
    };
    let f = fixture(false, Limits::default());
    let companion = Arc::new(
        Companion::new(
            f.host.clone(),
            None,
            Arc::new(VsCode::new("not-invoked".into())),
        )
        .unwrap(),
    );
    let reviews =
        Arc::new(ReviewApplication::start(f.host.clone(), CaptureLimits::default()).unwrap());
    let router = router_with_companion(f.host.clone(), reviews, companion);
    let request = |method: &'static str, uri: &'static str, body: Option<Value>| {
        let router = router.clone();
        async move {
            let response = tokio::time::timeout(
                Duration::from_secs(1),
                diagnostic_request(router, json!(1), method, uri, body),
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(response.status, 200, "{}", response.body);
            serde_json::from_str::<Value>(&response.body).unwrap()
        }
    };
    let scope = request(
        "POST",
        "/api/editor/handoff/window",
        Some(json!({"root":f.root})),
    )
    .await["scope"]
        .as_str()
        .unwrap()
        .to_owned();
    let pending=request("POST","/api/editor/handoff",Some(json!({"scope":scope,"sequence":1,"action":{"kind":"start","path":"a.rs","text":TEXT,"target":{"kind":"file","line":1}}}))).await;
    assert_eq!(pending["state"], "pending");
    entered(&f, 1).await;
    assert_eq!(request("GET", "/api/status", None).await["pending"], true);
    assert_eq!(
        request(
            "POST",
            "/api/editor/handoff/cancel",
            Some(json!({"scope":scope,"sequence":1}))
        )
        .await["state"],
        "cancelled"
    );
    assert!(f.host.application().status().pending);
    f.released.store(true, Ordering::SeqCst);
}

#[tokio::test]
async fn handoff_wire_decoding_and_source_allowances_are_independent_of_the_generic_body_limit() {
    use vizzi::{
        companion::Companion,
        editor::VsCode,
        review_application::ReviewApplication,
        review_source::CaptureLimits,
        server::{diagnostic_request, router_with_companion},
    };
    let f = fixture(true, Limits::default());
    let companion = Arc::new(
        Companion::new(
            f.host.clone(),
            None,
            Arc::new(VsCode::new("not-invoked".into())),
        )
        .unwrap(),
    );
    let reviews =
        Arc::new(ReviewApplication::start(f.host.clone(), CaptureLimits::default()).unwrap());
    let router = router_with_companion(f.host.clone(), reviews, companion);
    let attach = diagnostic_request(
        router.clone(),
        json!(1),
        "POST",
        "/api/editor/handoff/window",
        Some(json!({"root":f.root})),
    )
    .await
    .unwrap();
    let scope = serde_json::from_str::<Value>(&attach.body).unwrap()["scope"]
        .as_str()
        .unwrap()
        .to_owned();
    let maximum = " ".repeat(4 * 1024 * 1024);
    std::fs::write(f.root.join("a.rs"), &maximum).unwrap();
    let body = json!({"scope":scope,"sequence":1,"action":{"kind":"start","path":"a.rs","text":maximum,"target":{"kind":"file","line":1}}});
    let response = diagnostic_request(
        router.clone(),
        json!(2),
        "POST",
        "/api/editor/handoff",
        Some(body.clone()),
    )
    .await
    .unwrap();
    assert_eq!(response.status, 200);
    let mut accepted: Value = serde_json::from_str(&response.body).unwrap();
    let until = std::time::Instant::now() + Duration::from_secs(3);
    while accepted["state"] == "pending" {
        assert!(std::time::Instant::now() < until, "{accepted}");
        let next=diagnostic_request(router.clone(),json!(2),"POST","/api/editor/handoff",Some(json!({"scope":scope,"sequence":1,"action":{"kind":"continue","handle":accepted["handle"]}}))).await.unwrap();
        accepted = serde_json::from_str(&next.body).unwrap();
    }
    assert_eq!(
        accepted["state"], "ready",
        "maximum-size saved source: {accepted}"
    );
    let mut oversized = body.clone();
    oversized["sequence"] = json!(2);
    oversized["action"]["text"] = json!(" ".repeat(4 * 1024 * 1024 + 1));
    let response = diagnostic_request(
        router.clone(),
        json!(3),
        "POST",
        "/api/editor/handoff",
        Some(oversized),
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&response.body).unwrap()["code"],
        "invalid_source"
    );
    for malformed in [
        json!({"scope":scope,"sequence":3,"action":{"kind":"continue","handle":"x","executable":"code"}}),
        json!({"scope":scope,"sequence":3,"action":{"kind":"start","path":"a.rs","text":TEXT,"target":{"kind":"file","line":1,"build":true}}}),
    ] {
        assert_eq!(
            diagnostic_request(
                router.clone(),
                json!(4),
                "POST",
                "/api/editor/handoff",
                Some(malformed)
            )
            .await
            .unwrap()
            .status,
            422
        );
    }
}

#[tokio::test]
async fn handoff_cancelling_before_worker_start_preserves_deferred_capture() {
    let f = fixture(
        false,
        Limits {
            observe_wait: Duration::ZERO,
            ..Limits::default()
        },
    );
    let scope = f.handoffs.attach_window(&f.root).unwrap();
    let _ = value(&f, start(&scope, 1, TEXT, json!({"kind":"file","line":1}))).await;
    f.handoffs
        .cancel(serde_json::from_value(json!({"scope":scope,"sequence":1})).unwrap());
    tokio::task::yield_now().await;
    assert_eq!(
        f.host.application().status().readiness,
        vizzi::application::Readiness::NotStarted
    );
}

#[tokio::test]
async fn handoff_sequence_bounds_and_out_of_scope_files_do_not_poison_window_ordering_or_refresh() {
    let f = fixture(true, Limits::default());
    let scope = f.handoffs.attach_window(&f.root).unwrap();
    let initial = value(&f, start(&scope, 1, TEXT, json!({"kind":"file","line":1}))).await;
    assert_eq!(finish(&f, &scope, 1, initial).await["state"], "ready");
    for sequence in [0, 9_007_199_254_740_992] {
        assert_eq!(
            value(
                &f,
                start(&scope, sequence, TEXT, json!({"kind":"file","line":1}))
            )
            .await["code"],
            "invalid_request"
        );
        assert_eq!(
            serde_json::to_value(f.handoffs.cancel(
                serde_json::from_value(json!({"scope":scope,"sequence":sequence})).unwrap()
            ))
            .unwrap()["code"],
            "invalid_request"
        );
    }
    let before = f.host.application().desired();
    std::fs::write(f.root.join("notes"), "unclassified text").unwrap();
    let unknown=serde_json::from_value(json!({"scope":scope,"sequence":2,"action":{"kind":"start","path":"notes","text":"unclassified text","target":{"kind":"file","line":1}}})).unwrap();
    let answer = value(&f, unknown).await;
    assert_eq!(finish(&f, &scope, 2, answer).await["code"], "not_captured");
    assert_eq!(
        f.host.application().desired(),
        before,
        "file admission must not use directory-event relevance"
    );
    let maximum = 9_007_199_254_740_991;
    let accepted = value(
        &f,
        start(&scope, maximum, TEXT, json!({"kind":"file","line":1})),
    )
    .await;
    assert_eq!(
        finish(&f, &scope, maximum, accepted).await["state"],
        "ready"
    );
    let cancelled = serde_json::to_value(
        f.handoffs
            .cancel(serde_json::from_value(json!({"scope":scope,"sequence":maximum})).unwrap()),
    )
    .unwrap();
    assert_eq!(cancelled["state"], "cancelled");
}

#[tokio::test]
async fn handoff_newly_saved_files_join_one_capture_demand_across_editor_windows() {
    let f = fixture(true, Limits::default());
    let one = f.handoffs.attach_window(&f.root).unwrap();
    let two = f.handoffs.attach_window(&f.root).unwrap();
    let initial = value(&f, start(&one, 1, TEXT, json!({"kind":"file","line":1}))).await;
    assert_eq!(finish(&f, &one, 1, initial).await["state"], "ready");
    let before = f.host.application().desired();
    let text = "pub fn newly_saved() {}\n";
    f.released.store(false, Ordering::SeqCst);
    std::fs::write(f.root.join("new.rs"), text).unwrap();
    let request = |scope: &str, sequence| {
        serde_json::from_value(json!({"scope":scope,"sequence":sequence,"action":{"kind":"start","path":"new.rs","text":text,"target":{"kind":"file","line":1}}})).unwrap()
    };
    let first = value(&f, request(&one, 2)).await;
    entered(&f, 2).await;
    let second = value(&f, request(&two, 1)).await;
    assert_eq!(f.host.application().desired(), before + 1);
    f.released.store(true, Ordering::SeqCst);
    let first = finish(&f, &one, 2, first).await;
    let second = finish(&f, &two, 1, second).await;
    assert_eq!(first["state"], "ready");
    assert_eq!(second["destination"], first["destination"]);
    assert_eq!(f.entered.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn handoff_cancelling_an_inflight_submission_reports_cancelled_to_the_waiting_caller() {
    let f = fixture(
        false,
        Limits {
            observe_wait: Duration::from_secs(1),
            ..Limits::default()
        },
    );
    let scope = f.handoffs.attach_window(&f.root).unwrap();
    tokio::join!(
        async {
            let result = value(&f, start(&scope, 1, TEXT, json!({"kind":"file","line":1}))).await;
            assert_eq!(result["state"], "cancelled");
        },
        async {
            entered(&f, 1).await;
            let result = serde_json::to_value(
                f.handoffs
                    .cancel(serde_json::from_value(json!({"scope":scope,"sequence":1})).unwrap()),
            )
            .unwrap();
            assert_eq!(result["state"], "cancelled");
            assert!(f.host.application().status().pending);
            f.released.store(true, Ordering::SeqCst);
        }
    );
}

#[tokio::test]
async fn handoff_dropping_the_handoff_owner_releases_its_wait_without_leaking_the_source_host() {
    let Fixture {
        _temp: temp,
        root,
        host,
        handoffs,
        released,
        ..
    } = fixture(false, Limits::default());
    let scope = handoffs.attach_window(&root).unwrap();
    let result = serde_json::to_value(
        handoffs
            .handoff(start(&scope, 1, TEXT, json!({"kind":"file","line":1})))
            .await,
    )
    .unwrap();
    assert_eq!(result["state"], "pending");
    let weak = Arc::downgrade(&host);
    drop(host);
    drop(handoffs);
    let until = std::time::Instant::now() + Duration::from_secs(1);
    while weak.upgrade().is_some() && std::time::Instant::now() < until {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    released.store(true, Ordering::SeqCst);
    assert!(
        weak.upgrade().is_none(),
        "handoff wait retained a closed source workspace"
    );
    drop(temp);
}

#[tokio::test]
async fn handoff_failed_initial_capture_is_reported_and_a_new_intent_can_retry_it() {
    let f = fixture(true, Limits::default());
    f.fail.store(true, Ordering::SeqCst);
    let scope = f.handoffs.attach_window(&f.root).unwrap();
    let first = value(&f, start(&scope, 1, TEXT, json!({"kind":"file","line":1}))).await;
    let refused = finish(&f, &scope, 1, first).await;
    assert_eq!(refused["code"], "capture_failed");
    f.fail.store(false, Ordering::SeqCst);
    let retry = value(&f, start(&scope, 2, TEXT, json!({"kind":"file","line":1}))).await;
    assert_eq!(finish(&f, &scope, 2, retry).await["state"], "ready");
    assert_eq!(f.entered.load(Ordering::SeqCst), 2);
}
