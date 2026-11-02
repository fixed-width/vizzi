//! Foreground progress and recovery publication through the public native host.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use vizzi::{
    cache::Cache,
    engine::Engine,
    host::{Host, HostOptions},
    source::{CaptureScope, Directory},
};
use vizzi_core::{
    analysis::{Analyzer, LanguageAdapter, Languages},
    model::{Analysis, Context, ModelError},
    rust::RustAdapter,
};

#[derive(Default)]
struct Activity {
    hold: AtomicBool,
    started: AtomicBool,
    overlay_hold: AtomicBool,
    overlay_started: AtomicBool,
    foreground_hold: AtomicBool,
    foreground_started: AtomicBool,
    active: AtomicUsize,
    peak: AtomicUsize,
}
struct Adapter(Arc<Activity>);
impl LanguageAdapter for Adapter {
    fn context(&self) -> Context {
        RustAdapter.context()
    }
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        Ok(Box::new(Parser {
            inner: RustAdapter.create()?,
            activity: self.0.clone(),
        }))
    }
}
struct Parser {
    inner: Box<dyn Analyzer>,
    activity: Arc<Activity>,
}
impl Analyzer for Parser {
    fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError> {
        self.inner.analyze(bytes)
    }
    fn analyze_cancellable(
        &mut self,
        bytes: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Analysis>, ModelError> {
        let active = self.activity.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.activity.peak.fetch_max(active, Ordering::SeqCst);
        if bytes.starts_with(b"fn slow_recovered") {
            self.activity.started.store(true, Ordering::Release);
            while self.activity.hold.load(Ordering::Acquire) && !cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        if bytes.starts_with(b"fn slow_overlay") {
            self.activity.overlay_started.store(true, Ordering::Release);
            while self.activity.overlay_hold.load(Ordering::Acquire) && !cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        if bytes.starts_with(b"fn foreground_") {
            self.activity
                .foreground_started
                .store(true, Ordering::Release);
            while self.activity.foreground_hold.load(Ordering::Acquire) && !cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        let result = if bytes.starts_with(b"fn fail_recovery") {
            Err(ModelError("deliberate recovery failure".into()))
        } else {
            self.inner.analyze_cancellable(bytes, cancelled)
        };
        self.activity.active.fetch_sub(1, Ordering::SeqCst);
        result
    }
}
fn until(mut predicate: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(8) {
        if predicate() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    false
}
struct Fixture {
    root: tempfile::TempDir,
    cache: tempfile::TempDir,
    activity: Arc<Activity>,
}
impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
            cache: tempfile::tempdir().unwrap(),
            activity: Arc::new(Activity::default()),
        };
        for (name, source) in [
            ("a.rs", "fn original() {}"),
            ("b.rs", "fn slow_original() {}"),
            ("c.rs", "fn c() {}"),
        ] {
            std::fs::write(fixture.root.path().join(name), source).unwrap();
        }
        fixture
    }
    fn engine(&self, workers: usize) -> Engine {
        Engine::new(
            Directory::with_languages(
                self.root.path(),
                Arc::new(Languages::new(vec![Arc::new(Adapter(self.activity.clone()))]).unwrap()),
            )
            .unwrap(),
            Cache::new(self.cache.path().into()),
            workers,
        )
        .unwrap()
    }
    fn host(&self, workers: usize) -> Host {
        Host::start_with_options(
            self.engine(workers),
            HostOptions {
                watch: false,
                reconcile_interval: Duration::from_millis(20),
            },
        )
        .unwrap()
    }
}
#[test]
fn foreground_publishes_during_a_held_recovery_and_recovery_cannot_regress_it() {
    let fixture = Fixture::new();
    let root = &fixture.root;
    let activity = &fixture.activity;
    let host = fixture.host(2);
    let app = host.application();
    let original = app.snapshot().unwrap();
    activity.hold.store(true, Ordering::Release);
    std::fs::write(root.path().join("b.rs"), "fn slow_recovered() {}").unwrap();
    let started = until(|| activity.started.load(Ordering::Acquire));
    if !started {
        activity.hold.store(false, Ordering::Release);
    }
    assert!(started, "background recovery did not reach held input");
    activity.foreground_hold.store(true, Ordering::Release);
    for name in ["a", "c"] {
        std::fs::write(
            root.path().join(format!("{name}.rs")),
            format!("fn foreground_{name}() {{}}"),
        )
        .unwrap();
    }
    host.request(CaptureScope::Paths(["a.rs".into(), "c.rs".into()].into()));
    assert!(until(|| activity
        .foreground_started
        .load(Ordering::Acquire)));
    // A second foreground worker would overlap here with the held recovery parser.
    std::thread::sleep(Duration::from_millis(50));
    let peak = activity.peak.load(Ordering::SeqCst);
    activity.foreground_hold.store(false, Ordering::Release);
    assert_eq!(peak, 2);
    let published = until(|| {
        app.snapshot()
            .unwrap()
            .file_by_path("a.rs")
            .unwrap()
            .source()
            == Some("fn foreground_a() {}")
    });
    activity.hold.store(false, Ordering::Release);
    assert!(published, "foreground waited for the held recovery parse");
    assert!(until(|| app.status().recovery.completed > 0
        && app
            .snapshot()
            .unwrap()
            .file_by_path("b.rs")
            .unwrap()
            .source()
            == Some("fn slow_recovered() {}")));
    assert_eq!(
        app.snapshot()
            .unwrap()
            .file_by_path("a.rs")
            .unwrap()
            .source(),
        Some("fn foreground_a() {}")
    );
    assert_eq!(
        original.file_by_path("a.rs").unwrap().source(),
        Some("fn original() {}")
    );
    assert!(activity.peak.load(Ordering::SeqCst) <= 2);
    assert_eq!(
        app.snapshot().unwrap().id(),
        fixture
            .engine(2)
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
}

#[test]
fn hard_discovery_cancels_held_recovery_and_shutdown_joins_it() {
    let fixture = Fixture::new();
    let host = fixture.host(2);
    let app = host.application();
    fixture.activity.hold.store(true, Ordering::Release);
    std::fs::write(fixture.root.path().join("b.rs"), "fn slow_recovered() {}").unwrap();
    assert!(until(|| fixture.activity.started.load(Ordering::Acquire)));
    std::fs::write(fixture.root.path().join("b.rs"), "fn replacement() {}").unwrap();
    host.request(CaptureScope::Reconcile);
    assert!(until(|| app.status().recovery.discarded > 0
        && app
            .snapshot()
            .unwrap()
            .file_by_path("b.rs")
            .unwrap()
            .source()
            == Some("fn replacement() {}")));
    fixture.activity.started.store(false, Ordering::Release);
    std::fs::write(
        fixture.root.path().join("b.rs"),
        "fn slow_recovered_again() {}",
    )
    .unwrap();
    assert!(until(|| fixture.activity.started.load(Ordering::Acquire)));
    drop(host);
    assert_eq!(fixture.activity.active.load(Ordering::SeqCst), 0);
    assert!(!app.status().recovery.running);
}
#[test]
fn recovery_failure_retains_the_observation_and_retries_without_foreground_error() {
    let fixture = Fixture::new();
    let host = fixture.host(2);
    let app = host.application();
    let original = app.snapshot().unwrap();
    std::fs::write(fixture.root.path().join("b.rs"), "fn fail_recovery() {}").unwrap();
    assert!(until(|| app
        .status()
        .recovery
        .error
        .as_deref()
        .is_some_and(|e| e.contains("deliberate recovery failure"))));
    assert_eq!(app.snapshot().unwrap().id(), original.id());
    assert_eq!(app.status().error, None);
    assert!(!app.status().pending);
    std::fs::write(fixture.root.path().join("b.rs"), "fn restored() {}").unwrap();
    assert!(until(|| app.status().recovery.completed > 0
        && app
            .snapshot()
            .unwrap()
            .file_by_path("b.rs")
            .unwrap()
            .source()
            == Some("fn restored() {}")));
    assert_eq!(app.status().recovery.error, None);
}
#[test]
fn single_worker_recovers_silent_changes_serially_and_rejects_invalid_intervals() {
    let fixture = Fixture::new();
    for interval in [Duration::ZERO, Duration::MAX] {
        assert!(
            Host::start_with_options(
                fixture.engine(1),
                HostOptions {
                    watch: false,
                    reconcile_interval: interval
                }
            )
            .is_err()
        );
    }
    let host = fixture.host(1);
    let app = host.application();
    std::fs::write(
        fixture.root.path().join("b.rs"),
        "fn recovered_serially() {}",
    )
    .unwrap();
    assert!(until(|| app
        .snapshot()
        .unwrap()
        .file_by_path("b.rs")
        .unwrap()
        .source()
        == Some("fn recovered_serially() {}")));
    assert_eq!(app.status().recovery.completed, 0);
    assert!(!app.status().recovery.running);
    assert_eq!(fixture.activity.peak.load(Ordering::SeqCst), 1);
}

#[test]
fn a_finished_private_candidate_is_discarded_after_hard_invalidation() {
    let fixture = Fixture::new();
    let host = fixture.host(2);
    let app = host.application();
    let activity = &fixture.activity;
    activity.hold.store(true, Ordering::Release);
    std::fs::write(fixture.root.path().join("b.rs"), "fn slow_recovered() {}").unwrap();
    assert!(until(|| activity.started.load(Ordering::Acquire)));
    activity.foreground_hold.store(true, Ordering::Release);
    std::fs::write(fixture.root.path().join("a.rs"), "fn foreground_held() {}").unwrap();
    host.request(CaptureScope::Paths(["a.rs".into()].into()));
    assert!(until(|| activity
        .foreground_started
        .load(Ordering::Acquire)));
    activity.hold.store(false, Ordering::Release);
    assert!(until(|| activity.active.load(Ordering::SeqCst) == 1));
    // Let the private task finish while the foreground parser remains held.
    std::thread::sleep(Duration::from_millis(100));
    std::fs::write(fixture.root.path().join("a.rs"), "fn full_discovery() {}").unwrap();
    host.request(CaptureScope::Reconcile);
    assert!(until(
        || app.status().recovery.discarded > 0 && !app.status().pending
    ));
    std::fs::write(
        fixture.root.path().join("c.rs"),
        "fn silent_after_discard() {}",
    )
    .unwrap();
    assert!(until(|| app
        .snapshot()
        .unwrap()
        .file_by_path("c.rs")
        .unwrap()
        .source()
        == Some("fn silent_after_discard() {}")));
}

#[test]
fn new_hints_interrupt_an_overlay_and_the_fresh_retry_recovers_a_later_silent_edit() {
    let fixture = Fixture::new();
    let host = fixture.host(2);
    let app = host.application();
    let activity = &fixture.activity;
    activity.hold.store(true, Ordering::Release);
    std::fs::write(fixture.root.path().join("b.rs"), "fn slow_recovered() {}").unwrap();
    assert!(until(|| activity.started.load(Ordering::Acquire)));
    std::fs::write(fixture.root.path().join("a.rs"), "fn foreground_a() {}").unwrap();
    host.request(CaptureScope::Paths(["a.rs".into()].into()));
    assert!(until(|| app
        .snapshot()
        .unwrap()
        .file_by_path("a.rs")
        .unwrap()
        .source()
        == Some("fn foreground_a() {}")));
    activity.overlay_hold.store(true, Ordering::Release);
    std::fs::write(fixture.root.path().join("a.rs"), "fn slow_overlay() {}").unwrap();
    activity.hold.store(false, Ordering::Release);
    assert!(until(|| activity.overlay_started.load(Ordering::Acquire)));
    std::fs::write(fixture.root.path().join("c.rs"), "fn newest_c() {}").unwrap();
    host.request(CaptureScope::Paths(["c.rs".into()].into()));
    let published = until(|| {
        app.snapshot()
            .unwrap()
            .file_by_path("c.rs")
            .unwrap()
            .source()
            == Some("fn newest_c() {}")
    });
    activity.overlay_hold.store(false, Ordering::Release);
    assert!(
        published,
        "foreground hint waited for an obsolete held overlay"
    );
    assert!(until(|| app.status().recovery.completed > 0
        && app
            .snapshot()
            .unwrap()
            .file_by_path("a.rs")
            .unwrap()
            .source()
            == Some("fn slow_overlay() {}")));
    assert_eq!(
        app.snapshot().unwrap().id(),
        fixture
            .engine(2)
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
}

#[test]
fn periodic_noops_finish_without_discards_or_changed_generations() {
    let fixture = Fixture::new();
    let host = fixture.host(2);
    let app = host.application();
    let generation = app.status().generation;
    assert!(until(|| app.status().recovery.completed >= 2));
    let status = app.status();
    assert_eq!(status.generation, generation);
    assert_eq!(status.recovery.discarded, 0);
    assert_eq!(status.recovery.verification.unwrap().read_files, 3);
    assert_eq!(status.recovery.overlay, None);
}

#[test]
fn foreground_discovery_registers_new_directories_before_the_next_periodic_pass() {
    let fixture = Fixture::new();
    let host = Host::start_with_options(
        fixture.engine(2),
        HostOptions {
            watch: true,
            reconcile_interval: Duration::from_secs(60),
        },
    )
    .unwrap();
    let app = host.application();
    assert!(until(
        || app.status().recovery.completed > 0 && !app.status().pending
    ));
    let directory = fixture.root.path().join("new");
    std::fs::create_dir(&directory).unwrap();
    let file = directory.join("fresh.rs");
    std::fs::write(&file, "fn fresh() {}").unwrap();
    host.request(CaptureScope::Reconcile);
    assert!(until(|| !app.status().pending
        && app
            .snapshot()
            .unwrap()
            .file_by_path("new/fresh.rs")
            .is_some()));
    std::fs::write(&file, "fn changed_via_new_watch() {}").unwrap();
    assert!(until(|| app
        .snapshot()
        .unwrap()
        .file_by_path("new/fresh.rs")
        .unwrap()
        .source()
        == Some("fn changed_via_new_watch() {}")));
}
