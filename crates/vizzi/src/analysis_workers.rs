//! Shared aggregate parser allocation with one background attempt at a time.

use crate::admission::Priority;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

/// Aggregate analysis-worker observations, independently of source/input byte admission.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct WorkerStats {
    /// Configured maximum active parser workers.
    pub capacity: usize,
    /// Currently allocated parser workers.
    pub active: usize,
    /// Whether review or recovery owns the single background allocation.
    pub background: bool,
    /// Maximum simultaneous allocation observed.
    pub peak: usize,
}
#[derive(Default)]
struct Active {
    workers: usize,
    background: bool,
    peak: usize,
}
pub(crate) struct Workers {
    capacity: usize,
    state: Mutex<Active>,
    wake: Condvar,
}
pub(crate) struct Allocation {
    pool: Arc<Workers>,
    pub workers: usize,
    background: bool,
}

impl Workers {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            state: Mutex::new(Active::default()),
            wake: Condvar::new(),
        }
    }
    pub(crate) fn acquire(
        self: &Arc<Self>,
        requested: usize,
        priority: Priority,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Option<Allocation> {
        loop {
            if cancelled() {
                return None;
            }
            let mut state = lock(&self.state);
            let available = self.capacity - state.workers;
            let background = priority == Priority::Recovery && requested > 0;
            if requested == 0 || (available > 0 && (!background || !state.background)) {
                let count = if background {
                    1
                } else {
                    requested.min(available)
                };
                state.workers += count;
                state.background |= background;
                state.peak = state.peak.max(state.workers);
                return Some(Allocation {
                    pool: self.clone(),
                    workers: count,
                    background,
                });
            }
            let (state, _) = self
                .wake
                .wait_timeout(state, Duration::from_millis(20))
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            drop(state);
        }
    }
    pub(crate) fn stats(&self) -> WorkerStats {
        let state = lock(&self.state);
        WorkerStats {
            capacity: self.capacity,
            active: state.workers,
            background: state.background,
            peak: state.peak,
        }
    }
}
impl Drop for Allocation {
    fn drop(&mut self) {
        let mut state = lock(&self.pool.state);
        state.workers -= self.workers;
        if self.background {
            state.background = false;
        }
        drop(state);
        self.pool.wake.notify_all();
    }
}
fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    };

    #[test]
    fn one_background_allocation_leaves_remaining_capacity_for_foreground() {
        let pool = Arc::new(Workers::with_capacity(3));
        let background = pool
            .acquire(usize::MAX, Priority::Recovery, &|| false)
            .unwrap();
        assert_eq!(background.workers, 1);
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread_pool = pool.clone();
        let waiting = std::thread::spawn(move || {
            let first = AtomicBool::new(true);
            let value = thread_pool.acquire(1, Priority::Recovery, &|| {
                if first.swap(false, Ordering::SeqCst) {
                    started_tx.send(()).unwrap();
                }
                thread_stop.load(Ordering::SeqCst)
            });
            done_tx
                .send(value.map(|allocation| allocation.workers))
                .unwrap();
        });
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(done_rx.recv_timeout(Duration::from_millis(50)).is_err());
        let foreground = pool
            .acquire(usize::MAX, Priority::Foreground, &|| false)
            .unwrap();
        assert_eq!(foreground.workers, 2);
        assert_eq!(pool.stats().active, 3);
        assert_eq!(pool.stats().peak, 3);
        stop.store(true, Ordering::SeqCst);
        assert_eq!(done_rx.recv_timeout(Duration::from_secs(2)).unwrap(), None);
        waiting.join().unwrap();
        drop(foreground);
        drop(background);
        assert_eq!(pool.stats().active, 0);
        assert!(!pool.stats().background);
    }

    #[test]
    fn single_worker_waits_and_zero_job_attempts_do_not_hold_background_capacity() {
        let pool = Arc::new(Workers::with_capacity(1));
        let zero = pool.acquire(0, Priority::Recovery, &|| false).unwrap();
        assert_eq!(zero.workers, 0);
        assert!(!pool.stats().background);
        drop(zero);
        let background = pool.acquire(1, Priority::Recovery, &|| false).unwrap();
        let (tx, rx) = mpsc::sync_channel(1);
        let worker_pool = pool.clone();
        let thread = std::thread::spawn(move || {
            let allocation = worker_pool
                .acquire(2, Priority::Foreground, &|| false)
                .unwrap();
            tx.send(allocation).unwrap();
        });
        assert!(rx.recv_timeout(Duration::from_millis(50)).is_err());
        drop(background);
        let foreground = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(foreground.workers, 1);
        drop(foreground);
        thread.join().unwrap();
        assert_eq!(pool.stats().active, 0);
        assert_eq!(pool.stats().peak, 1);
        assert!(pool.acquire(1, Priority::Foreground, &|| true).is_none());
    }
}
