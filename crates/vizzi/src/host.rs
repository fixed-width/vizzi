//! Background refresh coordination and native directory observation.

use crate::{
    application::Application,
    engine::{Engine, EngineError, Refresh},
    invalidation::{PendingRefresh, RecoveryToken, classify},
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
    queue: Arc<Mutex<PendingRefresh>>,
}
/// Native observation and periodic verification settings.
#[derive(Clone, Debug)]
pub struct HostOptions {
    /// Use native filesystem notifications; false deliberately selects polling.
    pub watch: bool,
    /// Delay between accepted verification and the next periodic request.
    pub reconcile_interval: Duration,
}
impl Default for HostOptions {
    fn default() -> Self {
        Self {
            watch: true,
            reconcile_interval: Duration::from_secs(2),
        }
    }
}
struct RecoveryTask {
    token: RecoveryToken,
    task: Option<JoinHandle<Result<Option<Refresh>, EngineError>>>,
    ready: Option<Refresh>,
}
impl RecoveryTask {
    fn running(&self) -> bool {
        self.task.is_some()
    }
}
impl Host {
    /// Captures the initial observation, then watches with periodic reconciliation.
    pub fn start(engine: Engine) -> Result<Self, EngineError> {
        Self::start_with_options(engine, HostOptions::default())
    }
    /// Starts a host with explicit observation policy; analyzed projects are never run.
    pub fn start_with_options(engine: Engine, options: HostOptions) -> Result<Self, EngineError> {
        if options.reconcile_interval.is_zero()
            || Instant::now()
                .checked_add(options.reconcile_interval)
                .is_none()
        {
            return Err(EngineError::Configuration(
                "reconciliation interval must be positive and representable".into(),
            ));
        }
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
        let mut watcher = if options.watch {
            notify::recommended_watcher(move |event: Result<notify::Event, notify::Error>| {
                if let Some(scope) = classify(callback_engine.directory(), event) {
                    pending(&callback_queue).invalidate(&callback_app, scope);
                    let _ = callback_signal.try_send(());
                }
            })
            .ok()
        } else {
            None
        };
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
        let worker_queue = queue.clone();
        let task = std::thread::spawn(move || {
            let queue = worker_queue;
            let mut schedule = Reconciliation::new(Instant::now());
            schedule.interval = options.reconcile_interval;
            let mut recovery: Option<RecoveryTask> = None;
            loop {
                if worker_stop.load(Ordering::Acquire) {
                    break;
                }
                if schedule.due(Instant::now()) {
                    if engine.workers() == 1 || worker_app.status().error.is_some() {
                        pending(&queue).invalidate(&worker_app, CaptureScope::Reconcile);
                    } else if recovery.is_none() {
                        let token = pending(&queue).begin_recovery(&worker_app);
                        if let Some(token) = token
                            && let Some(base) = worker_app.snapshot()
                        {
                            worker_app.recovery_started();
                            let background_engine = engine.clone();
                            let background_stop = worker_stop.clone();
                            let background_token = token.clone();
                            let task = std::thread::spawn(move || {
                                background_engine.refresh_budget(
                                    Some(&base),
                                    &CaptureScope::Reconcile,
                                    &|| {
                                        background_token
                                            .cancelled(background_stop.load(Ordering::Acquire))
                                    },
                                    1,
                                )
                            });
                            recovery = Some(RecoveryTask {
                                token,
                                task: Some(task),
                                ready: None,
                            });
                        }
                    }
                }
                let finished = recovery
                    .as_ref()
                    .is_some_and(|work| work.task.as_ref().is_some_and(|task| task.is_finished()));
                if finished
                    && let Some(work) = &mut recovery
                    && let Some(task) = work.task.take()
                {
                    match join_recovery(task) {
                        Ok(Some(refresh)) => work.ready = Some(refresh),
                        result => {
                            let error = result.err().map(|error| error.to_string());
                            worker_app.recovery_discarded(error);
                            pending(&queue).cancel_recovery();
                            recovery = None;
                        }
                    }
                }
                if recovery
                    .as_ref()
                    .is_some_and(|work| !work.running() && work.token.cancelled(false))
                {
                    worker_app.recovery_discarded(None);
                    recovery = None;
                }
                if worker_app.status().pending {
                    while receive.try_recv().is_ok() {}
                    std::thread::sleep(Duration::from_millis(25));
                    if worker_stop.load(Ordering::Acquire) {
                        break;
                    }
                    let plan = pending(&queue).plan(&worker_app);
                    if matches!(plan.scope, CaptureScope::Reconcile) {
                        pending(&queue).cancel_recovery();
                    }
                    let previous = worker_app.snapshot();
                    let cancelled =
                        || plan.cancelled(&worker_app, worker_stop.load(Ordering::Acquire));
                    let workers = engine.workers()
                        - usize::from(recovery.as_ref().is_some_and(RecoveryTask::running));
                    match engine.refresh_budget(
                        previous.as_deref(),
                        &plan.scope,
                        &cancelled,
                        workers,
                    ) {
                        Ok(Some(refresh)) => {
                            let directories = refresh.directories.clone();
                            let outcome =
                                pending(&queue).publish(&worker_app, plan.ticket, refresh);
                            if outcome != vizzi_core::publication::Outcome::Stale
                                && let Some(directories) = directories
                            {
                                schedule.completed(Instant::now());
                                if observe(&mut watcher, &mut watched, &directories, &worker_app) {
                                    pending(&queue)
                                        .invalidate(&worker_app, CaptureScope::Reconcile);
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(error) => worker_app.fail(plan.ticket, error.to_string()),
                    }
                }
                if let Some(work) = &recovery
                    && let Some(candidate) = &work.ready
                {
                    let plan = pending(&queue).recovery_plan(&worker_app, &work.token);
                    if let Some(plan) = plan {
                        let overlay_needed =
                            matches!(&plan.scope,CaptureScope::Paths(paths) if !paths.is_empty());
                        let result = if overlay_needed {
                            engine.refresh_budget(
                                Some(&candidate.snapshot),
                                &plan.scope,
                                &|| {
                                    plan.cancelled(&worker_app, worker_stop.load(Ordering::Acquire))
                                        || work.token.cancelled(false)
                                },
                                engine.workers(),
                            )
                        } else {
                            Ok(Some(candidate.clone()))
                        };
                        match result {
                            Ok(Some(mut refresh)) => {
                                let overlay = overlay_needed.then(|| refresh.measurements.clone());
                                if refresh.directories.is_none() {
                                    refresh.directories = candidate.directories.clone();
                                }
                                let directories = refresh.directories.clone();
                                let outcome = pending(&queue).publish_recovery(
                                    &worker_app,
                                    &work.token,
                                    plan.ticket,
                                    refresh,
                                    candidate.measurements.clone(),
                                    overlay,
                                );
                                if outcome != vizzi_core::publication::Outcome::Stale {
                                    recovery = None;
                                    schedule.completed(Instant::now());
                                    if let Some(directories) = directories
                                        && observe(
                                            &mut watcher,
                                            &mut watched,
                                            &directories,
                                            &worker_app,
                                        )
                                    {
                                        pending(&queue)
                                            .invalidate(&worker_app, CaptureScope::Reconcile);
                                    }
                                }
                            }
                            Ok(None) => {}
                            Err(error) => {
                                worker_app.recovery_discarded(Some(error.to_string()));
                                pending(&queue).cancel_recovery();
                                recovery = None;
                            }
                        }
                    }
                }
                let wait = schedule.wait(
                    Instant::now(),
                    worker_app.status().pending,
                    recovery.is_some(),
                );
                match receive.recv_timeout(wait) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            pending(&queue).cancel_recovery();
            if let Some(mut work) = recovery {
                if let Some(task) = work.task.take() {
                    let _ = task.join();
                }
                worker_app.recovery_discarded(None);
            }
        });
        Ok(Self {
            application,
            stop,
            signal,
            task: Some(task),
            queue,
        })
    }
    /// Supplies source hints or requests discovery through the same invalidation path.
    pub fn request(&self, scope: CaptureScope) {
        pending(&self.queue).invalidate(&self.application, scope);
        let _ = self.signal.try_send(());
    }
    /// Returns the same application used by browser and diagnostic hosts.
    pub fn application(&self) -> Arc<Application> {
        self.application.clone()
    }
}
fn join_recovery(
    task: JoinHandle<Result<Option<Refresh>, EngineError>>,
) -> Result<Option<Refresh>, EngineError> {
    task.join().unwrap_or_else(|_| {
        Err(EngineError::Configuration(
            "recovery worker panicked".into(),
        ))
    })
}
fn pending(queue: &Mutex<PendingRefresh>) -> MutexGuard<'_, PendingRefresh> {
    queue
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
struct Reconciliation {
    next: Instant,
    interval: Duration,
}
impl Reconciliation {
    fn new(now: Instant) -> Self {
        Self {
            next: now,
            interval: Duration::from_secs(2),
        }
    }
    fn due(&mut self, now: Instant) -> bool {
        if now < self.next {
            return false;
        }
        self.next = now.checked_add(self.interval).unwrap_or(self.next);
        true
    }
    fn wait(&self, now: Instant, foreground: bool, recovery: bool) -> Duration {
        if foreground {
            return Duration::ZERO;
        }
        let remaining = self.next.saturating_duration_since(now);
        if recovery {
            remaining.min(Duration::from_millis(10))
        } else {
            remaining
        }
    }
    fn completed(&mut self, now: Instant) {
        self.next = now.checked_add(self.interval).unwrap_or(self.next);
    }
}
fn observe<W: Watcher>(
    watcher: &mut Option<W>,
    watched: &mut BTreeSet<PathBuf>,
    directories: &[PathBuf],
    app: &Application,
) -> bool {
    let Some(watcher) = watcher else {
        app.observation("polling; native watches disabled or unavailable".into());
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
        "native directory watches + periodic reconciliation".into()
    } else {
        format!("periodic reconciliation; {failures} directory watches unavailable")
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
            "periodic reconciliation; 1 directory watches unavailable"
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
            "polling; native watches disabled or unavailable"
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
        assert_eq!(schedule.wait(now, false, false), Duration::from_secs(2));
        assert_eq!(schedule.wait(now, true, false), Duration::ZERO);
        assert_eq!(schedule.wait(now, false, true), Duration::from_millis(10));
        assert_eq!(
            schedule.wait(now + Duration::from_millis(1995), false, true),
            Duration::from_millis(5)
        );
        assert!(!schedule.due(now + Duration::from_secs(1)));
        assert!(schedule.due(now + Duration::from_secs(2)));
        assert!(!schedule.due(now + Duration::from_secs(3)));
        schedule.completed(now + Duration::from_secs(10));
        assert_eq!(
            schedule.wait(now + Duration::from_secs(10), false, false),
            Duration::from_secs(2)
        );
        assert!(!schedule.due(now + Duration::from_secs(11)));
        assert!(schedule.due(now + Duration::from_secs(12)));
    }
    #[test]
    fn private_worker_panics_and_normal_cancellation_have_distinct_results() {
        let failure = join_recovery(std::thread::spawn(|| {
            panic!("deliberate private worker panic")
        }))
        .err()
        .unwrap();
        assert_eq!(failure.to_string(), "recovery worker panicked");
        assert!(
            join_recovery(std::thread::spawn(|| Ok(None)))
                .unwrap()
                .is_none()
        );
    }
}
