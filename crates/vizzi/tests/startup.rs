//! Deferred native startup and explicit live-index activation.

use std::{sync::Arc, time::Duration};
use vizzi::{
    cache::Cache,
    engine::Engine,
    host::{Host, HostOptions},
    source::Directory,
};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use vizzi_core::{
    analysis::{Analyzer, LanguageAdapter, Languages},
    model::{Analysis, Context, ContextId, ModelError},
    rust::RustAnalyzer,
};

struct ControlledRust {
    entered: mpsc::Sender<()>,
    released: Arc<AtomicBool>,
    fail: Arc<AtomicBool>,
}
impl LanguageAdapter for ControlledRust {
    fn context(&self) -> Context {
        Context {
            id: ContextId::derive(&[b"startup-controlled-rust"]),
            language: "rust".into(),
            description: "Controlled startup parser".into(),
        }
    }
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        Ok(Box::new(ControlledParser {
            context: self.context(),
            entered: self.entered.clone(),
            released: self.released.clone(),
            fail: self.fail.clone(),
            inner: RustAnalyzer::new()?,
        }))
    }
}
struct ControlledParser {
    context: Context,
    entered: mpsc::Sender<()>,
    released: Arc<AtomicBool>,
    fail: Arc<AtomicBool>,
    inner: RustAnalyzer,
}
impl Analyzer for ControlledParser {
    fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError> {
        let mut result = self.inner.analyze(bytes)?;
        result.context = self.context.clone();
        Ok(result)
    }
    fn analyze_cancellable(
        &mut self,
        bytes: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Analysis>, ModelError> {
        let _ = self.entered.send(());
        while !self.released.load(Ordering::Acquire) && !cancelled() {
            std::thread::sleep(Duration::from_millis(1));
        }
        if cancelled() {
            return Ok(None);
        }
        if self.fail.load(Ordering::Acquire) {
            return Err(ModelError("initial startup fixture failure".into()));
        }
        self.analyze(bytes).map(Some)
    }
}
fn controlled(
    root: &std::path::Path,
    cache: &std::path::Path,
    released: Arc<AtomicBool>,
    fail: Arc<AtomicBool>,
) -> (Host, mpsc::Receiver<()>) {
    let (entered, receive) = mpsc::channel();
    let languages = Languages::new(vec![
        Arc::new(ControlledRust {
            entered,
            released,
            fail,
        }),
        Arc::new(vizzi_core::python::PythonAdapter),
    ])
    .unwrap();
    let directory = Directory::with_languages(root, Arc::new(languages)).unwrap();
    let engine = Engine::new(directory, Cache::new(cache.into()), 1).unwrap();
    (
        Host::prepare_with_options(
            engine,
            HostOptions {
                watch: false,
                reconcile_interval: Duration::from_secs(3600),
                idle_maintenance: false,
            },
        )
        .unwrap(),
        receive,
    )
}
async fn complete(app: &vizzi::application::Application) -> vizzi::application::Status {
    let mut updates = app.subscribe();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let s = updates.next().await.unwrap().status.clone();
            if !s.pending {
                return s;
            }
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn initializing_metadata_is_available_duplicates_do_not_restart_and_drop_cancels() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn held() {}\n").unwrap();
    let (host, entered) = controlled(
        root.path(),
        cache.path(),
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );
    let app = host.application();
    assert!(entered.try_recv().is_err());
    host.activate();
    entered.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(
        app.status().readiness,
        vizzi::application::Readiness::Initializing
    );
    assert!(app.snapshot().is_none());
    assert!(app.status().measurements.is_none());
    let ticket = app.desired();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| host.activate());
        }
    });
    assert_eq!(app.desired(), ticket);
    assert_eq!(host.analysis_workers().active, 1);
    let mut subscription = app.subscribe();
    assert!(subscription.next().await.unwrap().status.pending);
    drop(host);
    assert!(subscription.next().await.is_none());
    assert!(app.snapshot().is_none());
}

#[tokio::test]
async fn initial_failure_has_no_empty_observation_and_explicit_activation_retries() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn retried() {}\n").unwrap();
    let fail = Arc::new(AtomicBool::new(true));
    let (host, _) = controlled(
        root.path(),
        cache.path(),
        Arc::new(AtomicBool::new(true)),
        fail.clone(),
    );
    let app = host.application();
    host.activate();
    let failed = complete(&app).await;
    assert_eq!(failed.readiness, vizzi::application::Readiness::Failed);
    assert!(failed.snapshot.is_none());
    assert!(
        failed
            .error
            .unwrap()
            .contains("initial startup fixture failure")
    );
    fail.store(false, Ordering::Release);
    host.activate();
    let ready = complete(&app).await;
    assert_eq!(ready.readiness, vizzi::application::Readiness::Ready);
    assert!(ready.error.is_none());
    assert_eq!(app.snapshot().unwrap().documents().len(), 1);
}

#[tokio::test]
async fn dormant_drop_closes_subscriptions_without_parsing_or_publishing() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn untouched() {}\n").unwrap();
    let (host, entered) = controlled(
        root.path(),
        cache.path(),
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );
    let app = host.application();
    let mut updates = app.subscribe();
    assert_eq!(
        updates.next().await.unwrap().status.readiness,
        vizzi::application::Readiness::NotStarted
    );
    drop(host);
    assert!(entered.try_recv().is_err());
    assert!(app.snapshot().is_none());
    assert!(updates.next().await.is_none());
}

fn git(root: &std::path::Path, args: &[&str]) {
    let result = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[tokio::test]
async fn scoped_review_finishes_with_the_live_index_dormant_and_stays_readable_during_initialization()
 {
    use vizzi::review_application::{CreateComparison, ReviewApplication, ReviewPageQuery};
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    git(
        root.path(),
        &["init", "--quiet", "--initial-branch=main", "--template="],
    );
    std::fs::create_dir(root.path().join("focus")).unwrap();
    std::fs::write(root.path().join("outside.rs"), "fn blocked() {}\n").unwrap();
    std::fs::write(root.path().join("focus/a.py"), "def before(): pass\n").unwrap();
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "--quiet", "-m", "fixture"]);
    std::fs::write(root.path().join("focus/a.py"), "def after(): pass\n").unwrap();
    let (host, entered) = controlled(
        root.path(),
        cache.path(),
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );
    let host = Arc::new(host);
    let app = host.application();
    let review =
        ReviewApplication::start(host.clone(), vizzi::review_source::CaptureLimits::default())
            .unwrap();
    review
        .create(CreateComparison {
            target_revision: None,
            provider: "git".into(),
            revision: "HEAD".into(),
            scope: Some("focus".into()),
        })
        .unwrap();
    let mut updates = review.subscribe();
    let metadata = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let s = updates.next().await.unwrap().status.clone();
            if !s.pending {
                assert!(s.error.is_none());
                break s.active.unwrap();
            }
        }
    })
    .await
    .unwrap();
    assert!(entered.try_recv().is_err());
    assert_eq!(
        app.status().readiness,
        vizzi::application::Readiness::NotStarted
    );
    assert!(app.snapshot().is_none());
    assert_eq!(host.analysis_workers().peak, 1);
    let query = review.query(&metadata.token).unwrap();
    let files = query.files(&ReviewPageQuery::default()).unwrap();
    assert_eq!(files.files[0].path, "focus/a.py");
    host.activate();
    entered.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(
        app.status().readiness,
        vizzi::application::Readiness::Initializing
    );
    let row = &files.files[0];
    let window = query
        .window(
            &row.file,
            vizzi_core::comparison::Side::Working,
            row.working_version.as_deref().unwrap(),
            &vizzi_core::projection::WindowQuery::default(),
        )
        .unwrap();
    assert_eq!(window.window.lines[0].text, "def after(): pass");
    drop(query);
    drop(review);
    drop(host);
}

#[tokio::test]
async fn http_metadata_is_passive_and_activation_requires_json() {
    use serde_json::json;
    use tower::ServiceExt;
    use vizzi::server::{diagnostic, diagnostic_request, router_with_host};
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn held() {}\n").unwrap();
    let (host, entered) = controlled(
        root.path(),
        cache.path(),
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );
    let host = Arc::new(host);
    let app = host.application();
    let review = Arc::new(
        vizzi::review_application::ReviewApplication::start(
            host.clone(),
            vizzi::review_source::CaptureLimits::default(),
        )
        .unwrap(),
    );
    let router = router_with_host(host.clone(), review.clone());
    for uri in ["/", "/review", "/api/status", "/api/comparisons/status"] {
        assert_eq!(
            diagnostic(router.clone(), json!(1), uri)
                .await
                .unwrap()
                .status,
            200
        );
    }
    assert!(entered.try_recv().is_err());
    assert_eq!(
        app.status().readiness,
        vizzi::application::Readiness::NotStarted
    );
    assert_eq!(
        diagnostic(router.clone(), json!(1), "/api/view")
            .await
            .unwrap()
            .status,
        503
    );
    let bad = diagnostic_request(
        router.clone(),
        json!(1),
        "POST",
        "/api/workspace/start",
        Some(json!({"root":"elsewhere"})),
    )
    .await
    .unwrap();
    assert_eq!(bad.status, 422);
    for (content_type, body, expected) in [
        ("text/plain", "{}".into(), 415),
        ("application/json", format!("{{}}{}", " ".repeat(1024)), 413),
    ] {
        let response = router
            .clone()
            .oneshot(
                axum::http::Request::post("/api/workspace/start")
                    .header("content-type", content_type)
                    .body(axum::body::Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    assert!(entered.try_recv().is_err());
    let response = router
        .clone()
        .oneshot(
            axum::http::Request::post("/api/workspace/start")
                .header("content-type", "application/json")
                .body(axum::body::Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 202);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    entered.recv_timeout(Duration::from_secs(3)).unwrap();
    let status = diagnostic(router.clone(), json!(1), "/api/status")
        .await
        .unwrap();
    let status: serde_json::Value = serde_json::from_str(&status.body).unwrap();
    assert_eq!(status["readiness"], "initializing");
    assert_eq!(status["snapshot"], serde_json::Value::Null);
    drop(router);
    drop(review);
    drop(host);
}

#[tokio::test]
async fn deferred_host_has_no_observation_until_explicit_activation() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn first() {}\n").unwrap();
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().into()),
        1,
    )
    .unwrap();
    let host = Arc::new(
        Host::prepare_with_options(
            engine,
            HostOptions {
                watch: false,
                reconcile_interval: Duration::from_secs(3600),
                idle_maintenance: false,
            },
        )
        .unwrap(),
    );
    let app = host.application();
    assert_eq!(
        serde_json::to_value(app.status()).unwrap()["readiness"],
        "not_started"
    );
    assert!(app.snapshot().is_none());
    assert_eq!(host.analysis_workers().peak, 0);
    let mut updates = app.subscribe();
    assert_eq!(updates.next().await.unwrap().status.snapshot, None);
    host.activate();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let next = updates.next().await.unwrap();
            if next.status.snapshot.is_some() {
                break;
            }
        }
    })
    .await
    .unwrap();
    let ready = app.status();
    assert_eq!(serde_json::to_value(&ready).unwrap()["readiness"], "ready");
    let desired = app.desired();
    host.activate();
    assert_eq!(app.desired(), desired);
    assert_eq!(app.status().snapshot, ready.snapshot);
    assert_eq!(host.analysis_workers().peak, 1);
}

#[tokio::test]
async fn a_native_hint_activates_full_initial_discovery_before_targeted_updates() {
    use vizzi::source::CaptureScope;
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    for file in ["hinted.rs", "unhinted.rs"] {
        std::fs::write(root.path().join(file), "fn before() {}\n").unwrap();
    }
    let (host, _) = controlled(
        root.path(),
        cache.path(),
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(false)),
    );
    let app = host.application();
    std::fs::write(root.path().join("hinted.rs"), "fn changed() {}\n").unwrap();
    std::fs::write(root.path().join("new.rs"), "fn added() {}\n").unwrap();
    let hint = || {
        CaptureScope::Paths(
            [std::path::PathBuf::from("hinted.rs")]
                .into_iter()
                .collect(),
        )
    };
    host.request(hint());
    let initial = complete(&app).await;
    assert!(initial.error.is_none(), "{:?}", initial.error);
    assert_eq!(initial.measurements.unwrap().files, 3);
    let snapshot = app.snapshot().unwrap();
    assert_eq!(snapshot.documents().len(), 3);
    assert!(
        snapshot
            .documents()
            .iter()
            .any(|d| d.resident_source() == Some("fn added() {}\n"))
    );
    std::fs::write(root.path().join("hinted.rs"), "fn next() {}\n").unwrap();
    host.request(hint());
    assert!(complete(&app).await.error.is_none());
    let updated = app.snapshot().unwrap();
    assert_eq!(updated.documents().len(), 3);
    assert!(
        updated
            .documents()
            .iter()
            .any(|d| d.resident_source() == Some("fn next() {}\n"))
    );
    assert_eq!(snapshot.documents().len(), 3);
}

#[tokio::test]
async fn activation_installs_watches_and_discovers_new_nested_source() {
    async fn source(app: &vizzi::application::Application, path: &str, text: &str) {
        let mut updates = app.subscribe();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                updates.next().await.unwrap();
                if app.snapshot().is_some_and(|s| {
                    s.file_by_path(path)
                        .is_some_and(|d| d.resident_source() == Some(text))
                }) {
                    return;
                }
            }
        })
        .await
        .unwrap();
    }
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn first() {}\n").unwrap();
    let host = Host::prepare_with_options(
        Engine::new(
            Directory::open(root.path()).unwrap(),
            Cache::new(cache.path().into()),
            1,
        )
        .unwrap(),
        HostOptions {
            watch: true,
            reconcile_interval: Duration::from_secs(60),
            idle_maintenance: false,
        },
    )
    .unwrap();
    let app = host.application();
    host.activate();
    assert!(complete(&app).await.error.is_none());
    std::fs::write(root.path().join("a.rs"), "fn changed() {}\n").unwrap();
    source(&app, "a.rs", "fn changed() {}\n").await;
    std::fs::create_dir(root.path().join("new")).unwrap();
    std::fs::write(root.path().join("new/b.rs"), "fn nested() {}\n").unwrap();
    source(&app, "new/b.rs", "fn nested() {}\n").await;
    std::fs::write(root.path().join("new/b.rs"), "fn nested_changed() {}\n").unwrap();
    source(&app, "new/b.rs", "fn nested_changed() {}\n").await;
}
