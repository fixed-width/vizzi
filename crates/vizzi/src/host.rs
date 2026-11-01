//! Background refresh coordination and native directory observation.

use crate::{
    application::Application,
    engine::{Engine, EngineError, Refresh},
};
use notify::{EventKind, RecursiveMode, Watcher};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::JoinHandle,
    time::Duration,
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
        ));
        let (signal, receive) = mpsc::sync_channel(1);
        let callback_app = application.clone();
        let callback_signal = signal.clone();
        let callback_engine = engine.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: Result<notify::Event, notify::Error>| {
                let relevant = match event {
                    Ok(event) => {
                        !matches!(event.kind, EventKind::Access(_))
                            && (event.need_rescan()
                                || event
                                    .paths
                                    .iter()
                                    .any(|path| callback_engine.directory().relevant(path)))
                    }
                    Err(_) => true,
                };
                if relevant {
                    callback_app.request();
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
            let ticket = application.request();
            let Some(refresh) = engine.refresh(None, &|| application.desired() != ticket)? else {
                continue;
            };
            observe(
                &mut watcher,
                &mut watched,
                &refresh.directories,
                &application,
            );
            if application.publish(ticket, refresh) != vizzi_core::publication::Outcome::Stale {
                break;
            }
        }
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker_app = application.clone();
        let task = std::thread::spawn(move || {
            // Reconcile after registering directory watches to close initial discovery gaps.
            worker_app.request();
            loop {
                if worker_stop.load(Ordering::Acquire) {
                    break;
                }
                if !worker_app.status().pending {
                    match receive.recv_timeout(Duration::from_secs(2)) {
                        Ok(()) => {}
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            worker_app.request();
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
                while receive.try_recv().is_ok() {}
                std::thread::sleep(Duration::from_millis(25));
                if worker_stop.load(Ordering::Acquire) {
                    break;
                }
                let ticket = worker_app.desired();
                let previous = worker_app.snapshot();
                let cancelled =
                    || worker_stop.load(Ordering::Acquire) || worker_app.desired() != ticket;
                match engine.refresh(previous.as_deref(), &cancelled) {
                    Ok(Some(refresh)) => {
                        observe(
                            &mut watcher,
                            &mut watched,
                            &refresh.directories,
                            &worker_app,
                        );
                        worker_app.publish(ticket, refresh);
                    }
                    Ok(None) => {}
                    Err(error) => worker_app.fail(ticket, error.to_string()),
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
fn observe(
    watcher: &mut Option<notify::RecommendedWatcher>,
    watched: &mut BTreeSet<PathBuf>,
    directories: &[PathBuf],
    app: &Application,
) {
    let Some(watcher) = watcher else {
        app.observation("polling; native watcher unavailable".into());
        return;
    };
    let desired: BTreeSet<_> = directories.iter().cloned().collect();
    for removed in watched.difference(&desired).cloned().collect::<Vec<_>>() {
        let _ = watcher.unwatch(&removed);
        watched.remove(&removed);
    }
    let mut failures = 0;
    for directory in directories {
        if !watched.contains(directory) {
            if watcher
                .watch(directory, RecursiveMode::NonRecursive)
                .is_ok()
            {
                watched.insert(directory.clone());
            } else {
                failures += 1;
            }
        }
    }
    app.observation(if failures == 0 {
        "native directory watches + 2 s reconciliation".into()
    } else {
        format!("2 s reconciliation; {failures} directory watches unavailable")
    });
}
impl Drop for Host {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
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
    ));
    let ticket = app.request();
    let refresh: Refresh = engine
        .refresh(None, &|| false)?
        .ok_or_else(|| EngineError::Configuration("one-shot capture was cancelled".into()))?;
    app.publish(ticket, refresh);
    app.observation("one-shot capture".into());
    Ok(app)
}
