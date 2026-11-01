//! Background refresh coordination and native directory observation.

use crate::{
    application::Application,
    engine::{Engine, EngineError, Refresh},
    invalidation::{PendingRefresh, classify},
    source::CaptureScope,
};
use notify::{RecursiveMode, Watcher};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

/// Owns a live workspace and stops its coordinator when dropped.
pub struct Host {
    application: Arc<Application>,
    stop: Arc<AtomicBool>,
    signal: SyncSender<()>,
    task: Option<JoinHandle<()>>,
}
impl Host {
    /// Captures the initial observation, then watches with periodic reconciliation.
    pub fn start(engine: Engine) -> Result<Self, EngineError> {
        let engine = Arc::new(engine);
        let application = Arc::new(Application::new(
            engine
                .directory()
                .root()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
        )?);
        let (signal, receive) = mpsc::sync_channel(1);
        let queue = Arc::new(Mutex::new(PendingRefresh::default()));
        let callback_app = application.clone();
        let callback_signal = signal.clone();
        let callback_engine = engine.clone();
        let callback_queue = queue.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: Result<notify::Event, notify::Error>| {
                if let Some(scope) = classify(callback_engine.directory(), event) {
                    pending(&callback_queue).invalidate(&callback_app, scope);
                    let _ = callback_signal.try_send(());
                }
            })
            .ok();
        let mut watched = BTreeSet::new();
        observe(
            &mut watcher,
            &mut watched,
            &[engine.directory().root().to_path_buf()],
            &application,
        );
        loop {
            pending(&queue).invalidate(&application, CaptureScope::Reconcile);
            let plan = pending(&queue).plan(&application);
            let Some(mut refresh) = engine
                .refresh_scoped(None, &plan.scope, &|| plan.cancelled(&application, false))?
            else {
                continue;
            };
            let directories = refresh.directories.take();
            if pending(&queue).publish(&application, plan.ticket, refresh)
                != vizzi_core::publication::Outcome::Stale
            {
                if let Some(directories) = directories {
                    observe(&mut watcher, &mut watched, &directories, &application);
                }
                break;
            }
        }
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker_app = application.clone();
        let task = std::thread::spawn(move || {
            let mut schedule = Reconciliation::new(Instant::now());
            loop {
                if worker_stop.load(Ordering::Acquire) {
                    break;
                }
                if schedule.due(Instant::now()) {
                    pending(&queue).invalidate(&worker_app, CaptureScope::Reconcile);
                }
                if !worker_app.status().pending {
                    match receive.recv_timeout(schedule.wait(Instant::now())) {
                        Ok(()) => {}
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
                while receive.try_recv().is_ok() {}
                if !worker_app.status().pending {
                    continue;
                }
                std::thread::sleep(Duration::from_millis(25));
                if worker_stop.load(Ordering::Acquire) {
                    break;
                }
                if schedule.due(Instant::now()) {
                    pending(&queue).invalidate(&worker_app, CaptureScope::Reconcile);
                }
                let plan = pending(&queue).plan(&worker_app);
                let previous = worker_app.snapshot();
                let cancelled = || plan.cancelled(&worker_app, worker_stop.load(Ordering::Acquire));
                match engine.refresh_scoped(previous.as_deref(), &plan.scope, &cancelled) {
                    Ok(Some(mut refresh)) => {
                        let directories = refresh.directories.take();
                        let outcome = pending(&queue).publish(&worker_app, plan.ticket, refresh);
                        if outcome != vizzi_core::publication::Outcome::Stale
                            && let Some(directories) = directories
                        {
                            schedule.completed(Instant::now());
                            if observe(&mut watcher, &mut watched, &directories, &worker_app) {
                                pending(&queue).invalidate(&worker_app, CaptureScope::Reconcile);
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(error) => worker_app.fail(plan.ticket, error.to_string()),
                }
            }
        });
        Ok(Self {
            application,
            stop,
            signal,
            task: Some(task),
        })
    }
    /// Returns the same application used by browser and diagnostic hosts.
    pub fn application(&self) -> Arc<Application> {
        self.application.clone()
    }
}
fn pending(queue: &Mutex<PendingRefresh>) -> MutexGuard<'_, PendingRefresh> {
    queue
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
struct Reconciliation {
    next: Instant,
}
impl Reconciliation {
    fn new(now: Instant) -> Self {
        Self { next: now }
    }
    fn due(&mut self, now: Instant) -> bool {
        if now < self.next {
            return false;
        }
        self.next = now + Duration::from_secs(2);
        true
    }
    fn wait(&self, now: Instant) -> Duration {
        self.next.saturating_duration_since(now)
    }
    fn completed(&mut self, now: Instant) {
        self.next = now + Duration::from_secs(2);
    }
}
fn observe<W: Watcher>(
    watcher: &mut Option<W>,
    watched: &mut BTreeSet<PathBuf>,
    directories: &[PathBuf],
    app: &Application,
) -> bool {
    let Some(watcher) = watcher else {
        app.observation("polling; native watcher unavailable".into());
        return false;
    };
    let desired: BTreeSet<_> = directories.iter().cloned().collect();
    for removed in watched.difference(&desired).cloned().collect::<Vec<_>>() {
        let _ = watcher.unwatch(&removed);
        watched.remove(&removed);
    }
    let mut failures = 0;
    let mut added = false;
    for directory in directories {
        if watcher
            .watch(directory, RecursiveMode::NonRecursive)
            .is_ok()
        {
            added |= watched.insert(directory.clone());
        } else {
            failures += 1;
            watched.remove(directory);
        }
    }
    app.observation(if failures == 0 {
        "native directory watches + 2 s reconciliation".into()
    } else {
        format!("2 s reconciliation; {failures} directory watches unavailable")
    });
    added
}
impl Drop for Host {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.application.close_updates();
        let _ = self.signal.try_send(());
        if let Some(task) = self.task.take() {
            let _ = task.join();
        }
    }
}

/// Publishes a one-shot refresh without starting observation.
pub fn one_shot(engine: &Engine) -> Result<Arc<Application>, EngineError> {
    let app = Arc::new(Application::new(
        engine.directory().root().display().to_string(),
    )?);
    let ticket = app.request();
    let refresh: Refresh = engine
        .refresh(None, &|| false)?
        .ok_or_else(|| EngineError::Configuration("one-shot capture was cancelled".into()))?;
    app.publish(ticket, refresh);
    app.observation("one-shot capture".into());
    Ok(app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cache::Cache, source::Directory};
    use futures_util::FutureExt;
    #[derive(Default)]
    struct TestWatcher {
        active: BTreeSet<PathBuf>,
        fail: BTreeSet<PathBuf>,
    }
    impl Watcher for TestWatcher {
        fn new<F: notify::EventHandler>(_: F, _: notify::Config) -> notify::Result<Self> {
            Ok(Self::default())
        }
        fn watch(&mut self, path: &std::path::Path, _: RecursiveMode) -> notify::Result<()> {
            if self.fail.contains(path) {
                Err(notify::Error::generic("unavailable"))
            } else {
                self.active.insert(path.into());
                Ok(())
            }
        }
        fn unwatch(&mut self, path: &std::path::Path) -> notify::Result<()> {
            self.active.remove(path);
            Ok(())
        }
        fn kind() -> notify::WatcherKind {
            notify::WatcherKind::NullWatcher
        }
    }
    #[test]
    fn registration_reports_new_paths_repairs_missing_watches_and_reports_failures() {
        let app = Application::new("watches".into()).unwrap();
        let mut watcher = Some(TestWatcher::default());
        let mut watched = BTreeSet::new();
        let paths = vec![PathBuf::from("one"), PathBuf::from("two")];
        assert!(observe(&mut watcher, &mut watched, &paths, &app));
        assert!(!observe(&mut watcher, &mut watched, &paths, &app));
        watcher.as_mut().unwrap().active.clear();
        assert!(!observe(&mut watcher, &mut watched, &paths, &app));
        assert_eq!(watcher.as_ref().unwrap().active, watched);
        watcher.as_mut().unwrap().fail.insert("two".into());
        assert!(!observe(&mut watcher, &mut watched, &paths, &app));
        assert_eq!(
            app.status().observation,
            "2 s reconciliation; 1 directory watches unavailable"
        );
        watcher.as_mut().unwrap().fail.clear();
        assert!(observe(&mut watcher, &mut watched, &paths, &app));
        assert!(observe(&mut watcher, &mut watched, &["three".into()], &app));
        assert_eq!(
            watcher.as_ref().unwrap().active,
            BTreeSet::from(["three".into()])
        );
        let mut absent: Option<TestWatcher> = None;
        assert!(!observe(&mut absent, &mut watched, &paths, &app));
        assert_eq!(
            app.status().observation,
            "polling; native watcher unavailable"
        );
    }
    #[test]
    fn dropping_a_host_closes_subscriptions_but_keeps_captured_queries_valid() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("a.rs"), "fn a() {}").unwrap();
        let engine = Engine::new(
            Directory::open(source.path()).unwrap(),
            Cache::new(cache.path().into()),
            1,
        )
        .unwrap();
        let host = Host::start(engine).unwrap();
        let app = host.application();
        let mut updates = app.subscribe();
        updates.next().now_or_never().unwrap().unwrap();
        drop(host);
        assert!(
            updates
                .next()
                .now_or_never()
                .expect("shutdown must finish delivery")
                .is_none()
        );
        assert_eq!(
            app.snapshot().unwrap().documents()[0].source(),
            Some("fn a() {}")
        );
    }
    #[test]
    fn reconciliation_deadlines_advance_after_attempts_and_successful_slow_passes() {
        let now = Instant::now();
        let mut schedule = Reconciliation::new(now);
        assert!(schedule.due(now));
        assert_eq!(schedule.wait(now), Duration::from_secs(2));
        assert!(!schedule.due(now + Duration::from_secs(1)));
        assert!(schedule.due(now + Duration::from_secs(2)));
        assert!(!schedule.due(now + Duration::from_secs(3)));
        schedule.completed(now + Duration::from_secs(10));
        assert_eq!(
            schedule.wait(now + Duration::from_secs(10)),
            Duration::from_secs(2)
        );
        assert!(!schedule.due(now + Duration::from_secs(11)));
        assert!(schedule.due(now + Duration::from_secs(12)));
    }
}
