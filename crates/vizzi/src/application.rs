//! Short-lived publication locks, with queries running on immutable handles.

use crate::{
    changes::{Changes, Subscription},
    engine::Refresh,
};
use serde::Serialize;
use std::sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicU64, Ordering},
};
use vizzi_core::{
    model::Snapshot,
    publication::{Outcome, Publication},
};

struct Inner {
    publication: Publication,
    measurements: Option<crate::engine::Measurements>,
    error: Option<String>,
    observation: String,
}
/// Transport-independent application operations for a single workspace.
pub struct Application {
    name: String,
    inner: Mutex<Inner>,
    desired: AtomicU64,
    changes: Changes,
}
/// A small status response; polling never reads source or runs analysis.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Status {
    /// Version of the host response schema.
    pub schema: u32,
    /// Workspace display name.
    pub name: String,
    /// Number of changed observations published.
    pub generation: u64,
    /// Current opaque snapshot identity.
    pub snapshot: Option<String>,
    /// Whether a refresh is outstanding.
    pub pending: bool,
    /// Latest completed capture and analysis measurements.
    pub measurements: Option<crate::engine::Measurements>,
    /// Most recent refresh failure, retaining the previous observation.
    pub error: Option<String>,
    /// Native observation mode and any watcher limitation.
    pub observation: String,
}
impl Application {
    /// Creates an empty application before initial capture.
    /// Returns an error when the system cannot supply a fresh stream identity.
    pub fn new(name: String) -> Result<Self, getrandom::Error> {
        let inner = Inner {
            publication: Publication::default(),
            measurements: None,
            error: None,
            observation: "initializing".into(),
        };
        let changes = Changes::new(status(&name, &inner))?;
        Ok(Self {
            name,
            inner: Mutex::new(inner),
            desired: AtomicU64::new(0),
            changes,
        })
    }
    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    /// Requests work under the same lock as publication.
    pub fn request(&self) -> u64 {
        let mut inner = self.inner();
        let ticket = inner.publication.request();
        self.desired.store(ticket, Ordering::Release);
        self.changes.update(status(&self.name, &inner));
        ticket
    }
    /// Returns the newest ticket without taking a lock in parser workers.
    pub fn desired(&self) -> u64 {
        self.desired.load(Ordering::Acquire)
    }
    /// Clones a coherent snapshot handle under a short lock.
    pub fn snapshot(&self) -> Option<Arc<Snapshot>> {
        self.inner().publication.current()
    }
    /// Retrieves a retained observation, never rereading mutable source.
    pub fn pinned(&self, id: &str) -> Option<Arc<Snapshot>> {
        self.inner().publication.pinned(id)
    }
    /// Publishes validated work only for the latest request.
    pub fn publish(&self, ticket: u64, refresh: Refresh) -> Outcome {
        let mut inner = self.inner();
        let outcome = inner.publication.publish(ticket, refresh.snapshot);
        if outcome != Outcome::Stale {
            inner.measurements = Some(refresh.measurements);
            inner.error = None;
            self.changes.update(status(&self.name, &inner));
        }
        outcome
    }
    /// Records failure of the latest request without clearing the current snapshot.
    pub fn fail(&self, ticket: u64, error: String) {
        let mut inner = self.inner();
        if inner.publication.ticket() == ticket {
            inner.publication.failed(ticket);
            inner.error = Some(error);
            self.changes.update(status(&self.name, &inner));
        }
    }
    /// Records native observation availability independently of analysis failures.
    pub fn observation(&self, mode: String) {
        let mut inner = self.inner();
        inner.observation = mode;
        self.changes.update(status(&self.name, &inner));
    }
    /// Reads small publication metadata.
    pub fn status(&self) -> Status {
        status(&self.name, &self.inner())
    }
    /// Subscribes to latest status metadata, with immediate initial delivery.
    pub fn subscribe(&self) -> Subscription {
        self.changes.subscribe()
    }
    /// Ends all current and future subscriptions without changing captured source.
    pub fn close_updates(&self) {
        self.changes.close();
    }
}

fn status(name: &str, inner: &Inner) -> Status {
    Status {
        schema: 1,
        name: name.into(),
        generation: inner.publication.generation(),
        snapshot: inner
            .publication
            .current()
            .map(|snapshot| snapshot.id().as_str().into()),
        pending: inner.publication.pending(),
        measurements: inner.measurements.clone(),
        error: inner.error.clone(),
        observation: inner.observation.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Measurements;
    use vizzi_core::model::WorkspaceId;
    fn refresh(parsed: usize) -> Refresh {
        Refresh {
            snapshot: Arc::new(
                Snapshot::new(
                    WorkspaceId::derive(&[b"test"]),
                    vec![],
                    vizzi_core::model::CapturePolicyId::derive(&[b"test"]),
                )
                .unwrap(),
            ),
            directories: Some(vec![]),
            measurements: Measurements {
                parsed,
                ..Default::default()
            },
        }
    }
    #[test]
    fn stale_work_and_failures_cannot_replace_latest_status() {
        let app = Application::new("fixture".into()).unwrap();
        assert_eq!(app.desired(), 0);
        assert!(app.status().snapshot.is_none());
        let first = app.request();
        let latest = app.request();
        assert_eq!(app.desired(), latest);
        assert!(latest > first);
        assert_eq!(app.publish(first, refresh(100)), Outcome::Stale);
        assert!(app.status().measurements.is_none());
        app.fail(first, "old error".into());
        assert!(app.status().pending);
        assert!(app.status().error.is_none());
        app.fail(latest, "current error".into());
        assert!(!app.status().pending);
        assert_eq!(app.status().error.as_deref(), Some("current error"));
        let next = app.request();
        app.publish(next, refresh(2));
        assert_eq!(app.status().measurements.unwrap().parsed, 2);
        assert!(app.status().error.is_none());
        app.observation("polling".into());
        assert_eq!(app.status().observation, "polling");
        assert_eq!(app.status().name, "fixture");
        assert_eq!(app.status().generation, 1);
    }

    #[tokio::test]
    async fn subscription_tracks_only_current_observable_state() {
        use futures_util::FutureExt;
        let app = Application::new("fixture".into()).unwrap();
        let mut updates = app.subscribe();
        let initial = updates
            .next()
            .now_or_never()
            .expect("notification should be ready")
            .unwrap();
        let first = app.request();
        let pending = updates
            .next()
            .now_or_never()
            .expect("request must notify synchronously")
            .unwrap();
        assert!(pending.status.pending);
        let latest = app.request();
        app.publish(first, refresh(100));
        app.fail(first, "obsolete".into());
        assert!(updates.next().now_or_never().is_none());
        app.publish(latest, refresh(2));
        let published = updates
            .next()
            .now_or_never()
            .expect("notification should be ready")
            .unwrap();
        assert_eq!(published.status.measurements.as_ref().unwrap().parsed, 2);
        assert_eq!(published.status.generation, 1);
        assert!(!published.status.pending);
        assert_eq!(published.revision, pending.revision + 1);
        let next = app.request();
        updates
            .next()
            .now_or_never()
            .expect("notification should be ready")
            .unwrap();
        app.fail(next, "read failed".into());
        assert_eq!(
            updates
                .next()
                .now_or_never()
                .expect("notification should be ready")
                .unwrap()
                .status
                .error
                .as_deref(),
            Some("read failed")
        );
        app.observation("fallback".into());
        assert_eq!(
            updates
                .next()
                .now_or_never()
                .expect("notification should be ready")
                .unwrap()
                .status
                .observation,
            "fallback"
        );
        app.observation("fallback".into());
        assert!(updates.next().now_or_never().is_none());
        assert!(!initial.status.pending);
        assert!(initial.status.snapshot.is_none());
        app.close_updates();
        assert!(
            updates
                .next()
                .now_or_never()
                .expect("notification should be ready")
                .is_none()
        );
    }
}
