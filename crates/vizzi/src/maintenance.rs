//! Idle admission of one blocking source-maintenance worker.
use serde::Serialize;
use std::{
    sync::{Arc, Mutex, MutexGuard},
    thread::JoinHandle,
    time::{Duration, Instant},
};

pub(crate) trait SourceMaintenance: Send + Sync {
    /// In-memory hint only; never waits for physical cleanup or reads file metadata.
    fn pending(&self) -> bool;
    fn run(&self) -> Result<(), String>;
}
/// Native source-maintenance progress, separate from foreground/recovery publication.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct MaintenanceStatus {
    /// Admitted attempts, including thread creation failures.
    pub attempts: u64,
    /// A cleanup worker is outstanding.
    pub running: bool,
    /// Successful maintenance operations, including metadata-only retirement batches.
    pub completed: u64,
    /// IO, panic or thread creation failures.
    pub failed: u64,
    /// Last failure, cleared by successful cleanup.
    pub error: Option<String>,
}
pub(crate) const QUIET: Duration = Duration::from_millis(250);
const RETRY: Duration = Duration::from_secs(1);
pub(crate) const POLL: Duration = Duration::from_millis(100);
pub(crate) struct Idle {
    source: Option<Arc<dyn SourceMaintenance>>,
    status: Arc<Mutex<MaintenanceStatus>>,
    quiet_since: Option<Instant>,
    next_attempt: Instant,
    task: Option<JoinHandle<Result<(), String>>>,
    stopped: bool,
}
impl Idle {
    pub fn eligible(pending: bool, recovering: bool) -> bool {
        !pending && !recovering
    }
    pub fn new(
        source: Option<Arc<dyn SourceMaintenance>>,
        status: Arc<Mutex<MaintenanceStatus>>,
        now: Instant,
    ) -> Self {
        Self {
            source,
            status,
            quiet_since: None,
            next_attempt: now,
            task: None,
            stopped: false,
        }
    }
    pub fn status(status: &Mutex<MaintenanceStatus>) -> MutexGuard<'_, MaintenanceStatus> {
        status
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub fn busy(&mut self) {
        self.quiet_since = None;
    }
    pub fn poll_wait(&self) -> Option<Duration> {
        if self.stopped {
            None
        } else {
            self.source.as_ref().map(|_| POLL)
        }
    }
    fn complete(&mut self, result: std::thread::Result<Result<(), String>>, now: Instant) {
        let result = result.unwrap_or_else(|_| Err("source maintenance worker panicked".into()));
        let mut status = Self::status(&self.status);
        status.running = false;
        match result {
            Ok(()) => {
                status.completed = status.completed.saturating_add(1);
                status.error = None;
            }
            Err(error) => {
                status.failed = status.failed.saturating_add(1);
                status.error = Some(error);
                self.next_attempt = now + RETRY;
            }
        }
    }
    pub fn poll(&mut self, now: Instant, idle: bool) {
        if self.stopped {
            return;
        }
        if self.task.as_ref().is_some_and(|task| task.is_finished())
            && let Some(task) = self.task.take()
        {
            self.complete(task.join(), now);
        }
        if !idle {
            self.busy();
            return;
        }
        let since = *self.quiet_since.get_or_insert(now);
        if self.task.is_some()
            || now.saturating_duration_since(since) < QUIET
            || now < self.next_attempt
        {
            return;
        }
        let Some(source) = &self.source else {
            return;
        };
        if !source.pending() {
            return;
        }
        let source = source.clone();
        {
            let mut status = Self::status(&self.status);
            status.attempts = status.attempts.saturating_add(1);
        }
        match std::thread::Builder::new()
            .name("vizzi-source-maintenance".into())
            .spawn(move || source.run())
        {
            Ok(task) => {
                self.task = Some(task);
                Self::status(&self.status).running = true;
            }
            Err(error) => self.complete(
                Ok(Err(format!("cannot start source maintenance: {error}"))),
                now,
            ),
        }
    }
    pub fn shutdown(&mut self) {
        self.stopped = true;
        if let Some(task) = self.task.take() {
            self.complete(task.join(), Instant::now());
        }
    }
}
impl Drop for Idle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    };

    struct Controlled {
        pending: AtomicBool,
        calls: AtomicUsize,
        action: Box<dyn Fn() -> Result<(), String> + Send + Sync>,
    }
    impl SourceMaintenance for Controlled {
        fn pending(&self) -> bool {
            self.pending.load(Ordering::SeqCst)
        }
        fn run(&self) -> Result<(), String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            (self.action)()
        }
    }
    fn drain(idle: &mut Idle, now: Instant) {
        let started = Instant::now();
        while Idle::status(&idle.status).running && started.elapsed() < Duration::from_secs(2) {
            idle.poll(now, false);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!Idle::status(&idle.status).running);
    }
    #[test]
    fn quiet_boundary_busy_reset_and_single_worker_admission() {
        let (entered, started) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        let resume = Mutex::new(resume);
        let work = Arc::new(Controlled {
            pending: AtomicBool::new(true),
            calls: AtomicUsize::new(0),
            action: Box::new(move || {
                entered.send(()).unwrap();
                resume.lock().unwrap().recv().unwrap();
                Ok(())
            }),
        });
        let status = Arc::new(Mutex::new(MaintenanceStatus::default()));
        let now = Instant::now();
        let mut idle = Idle::new(Some(work.clone()), status.clone(), now);
        assert_eq!(idle.poll_wait(), Some(POLL));
        idle.poll(now, true);
        idle.poll(now + QUIET - Duration::from_nanos(1), true);
        assert_eq!(Idle::status(&status).attempts, 0);
        idle.busy();
        idle.poll(now + QUIET, true);
        idle.poll(now + QUIET + QUIET - Duration::from_nanos(1), true);
        assert_eq!(Idle::status(&status).attempts, 0);
        idle.poll(now + QUIET + QUIET, true);
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        for offset in 1..10 {
            idle.poll(now + Duration::from_secs(offset), true);
        }
        assert_eq!(work.calls.load(Ordering::SeqCst), 1);
        assert_eq!(Idle::status(&status).attempts, 1);
        release.send(()).unwrap();
        drain(&mut idle, now + Duration::from_secs(10));
        assert_eq!(Idle::status(&status).completed, 1);
        work.pending.store(false, Ordering::SeqCst);
        idle.poll(now + Duration::from_secs(20), true);
        idle.poll(now + Duration::from_secs(21), true);
        assert_eq!(Idle::status(&status).attempts, 1);
        idle.shutdown();
        assert_eq!(idle.poll_wait(), None);
        work.pending.store(true, Ordering::SeqCst);
        idle.poll(now + Duration::from_secs(30), true);
        assert_eq!(Idle::status(&status).attempts, 1);
    }
    #[test]
    fn no_provider_no_work_or_foreground_activity_admits_nothing() {
        assert!(Idle::eligible(false, false));
        assert!(!Idle::eligible(true, false));
        assert!(!Idle::eligible(false, true));
        assert!(!Idle::eligible(true, true));
        let now = Instant::now();
        let status = Arc::new(Mutex::new(MaintenanceStatus::default()));
        let mut absent = Idle::new(None, status.clone(), now);
        absent.poll(now, true);
        absent.poll(now + Duration::from_secs(10), true);
        assert_eq!(absent.poll_wait(), None);
        assert_eq!(Idle::status(&status).attempts, 0);
        let work = Arc::new(Controlled {
            pending: AtomicBool::new(false),
            calls: AtomicUsize::new(0),
            action: Box::new(|| Ok(())),
        });
        let mut idle = Idle::new(Some(work.clone()), status.clone(), now);
        idle.poll(now, true);
        idle.poll(now + QUIET, true);
        assert_eq!(Idle::status(&status).attempts, 0);
        work.pending.store(true, Ordering::SeqCst);
        idle.poll(now + Duration::from_secs(1), false);
        idle.poll(now + Duration::from_secs(2), true);
        assert_eq!(Idle::status(&status).attempts, 0);
        idle.poll(now + Duration::from_secs(2) + QUIET, true);
        drain(&mut idle, now + Duration::from_secs(3));
        assert_eq!(Idle::status(&status).completed, 1);
    }
    #[test]
    fn failures_and_panics_back_off_and_success_clears_the_error() {
        for panic in [false, true] {
            let fail = Arc::new(AtomicBool::new(true));
            let behavior = fail.clone();
            let work = Arc::new(Controlled {
                pending: AtomicBool::new(true),
                calls: AtomicUsize::new(0),
                action: Box::new(move || {
                    if behavior.load(Ordering::SeqCst) {
                        if panic {
                            panic!("maintenance fixture");
                        }
                        Err("IO fixture".into())
                    } else {
                        Ok(())
                    }
                }),
            });
            let now = Instant::now();
            let status = Arc::new(Mutex::new(MaintenanceStatus::default()));
            let mut idle = Idle::new(Some(work.clone()), status.clone(), now);
            idle.poll(now, true);
            idle.poll(now + QUIET, true);
            drain(&mut idle, now + QUIET);
            assert_eq!(Idle::status(&status).failed, 1);
            assert!(Idle::status(&status).error.is_some());
            fail.store(false, Ordering::SeqCst);
            idle.poll(now + QUIET, true);
            idle.poll(now + QUIET + RETRY - Duration::from_nanos(1), true);
            assert_eq!(Idle::status(&status).attempts, 1);
            idle.poll(now + QUIET + RETRY, true);
            drain(&mut idle, now + QUIET + RETRY);
            assert_eq!(Idle::status(&status).completed, 1);
            assert_eq!(Idle::status(&status).failed, 1);
            assert!(Idle::status(&status).error.is_none());
        }
    }
    #[test]
    fn shutdown_joins_one_outstanding_worker_once() {
        for explicit in [false, true] {
            let (entered, started) = mpsc::channel();
            let (release, resume) = mpsc::channel();
            let resume = Mutex::new(resume);
            let work = Arc::new(Controlled {
                pending: AtomicBool::new(true),
                calls: AtomicUsize::new(0),
                action: Box::new(move || {
                    entered.send(()).unwrap();
                    resume.lock().unwrap().recv().unwrap();
                    Ok(())
                }),
            });
            let now = Instant::now();
            let status = Arc::new(Mutex::new(MaintenanceStatus::default()));
            let mut idle = Idle::new(Some(work), status.clone(), now);
            idle.poll(now, true);
            idle.poll(now + QUIET, true);
            started.recv_timeout(Duration::from_secs(2)).unwrap();
            let (done, finished) = mpsc::channel();
            let join = std::thread::spawn(move || {
                if explicit {
                    idle.shutdown();
                    idle.shutdown();
                }
                drop(idle);
                done.send(()).unwrap();
            });
            let premature = finished.recv_timeout(Duration::from_millis(20));
            release.send(()).unwrap();
            join.join().unwrap();
            assert!(premature.is_err());
            assert_eq!(Idle::status(&status).completed, 1);
            assert!(!Idle::status(&status).running);
        }
    }
}
