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
    recovery: RecoveryStatus,
}
/// Periodic verification progress, separate from foreground pending work.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct RecoveryStatus {
    /// Whether a private verification or its overlay is outstanding.
    pub running: bool,
    /// Accepted complete periodic verifications, including unchanged observations.
    pub completed: u64,
    /// Cancelled/failed private candidates.
    pub discarded: u64,
    /// Complete verification phase of the latest accepted recovery.
    pub verification: Option<crate::engine::Measurements>,
    /// Fresh dirty-path overlay phase, when needed; counters are not added together.
    pub overlay: Option<crate::engine::Measurements>,
    /// Most recent maintenance failure without changing foreground error state.
    pub error: Option<String>,
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
    /// Whether a first complete live observation is available or being prepared.
    pub readiness: Readiness,
    /// Private periodic recovery progress and phase measurements.
    pub recovery: RecoveryStatus,
    /// Version of the host response schema.
    pub schema: u32,
    /// Workspace display name.
    pub name: String,
    /// Number of changed observations published.
    pub generation: u64,
    /// Current opaque snapshot identity.
    pub snapshot: Option<String>,
    /// Whether foreground work is outstanding; private recovery is separate.
    pub pending: bool,
    /// Latest completed capture and analysis measurements.
    pub measurements: Option<crate::engine::Measurements>,
    /// Most recent refresh failure, retaining the previous observation.
    pub error: Option<String>,
    /// Native observation mode and any watcher limitation.
    pub observation: String,
}
/// Live-index readiness, independent of review status and later refresh activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Readiness {
    /// No live indexing has been requested.
    NotStarted,
    /// The first complete capture is outstanding.
    Initializing,
    /// A complete observation exists, including an empty repository.
    Ready,
    /// The initial attempt failed without a complete observation.
    Failed,
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
            recovery: RecoveryStatus::default(),
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
        self.publish_as(ticket, refresh, None)
    }
    pub(crate) fn publish_recovery(
        &self,
        ticket: u64,
        refresh: Refresh,
        verification: crate::engine::Measurements,
        overlay: Option<crate::engine::Measurements>,
    ) -> Outcome {
        self.publish_as(ticket, refresh, Some((verification, overlay)))
    }
    fn publish_as(
        &self,
        ticket: u64,
        refresh: Refresh,
        recovery: Option<(
            crate::engine::Measurements,
            Option<crate::engine::Measurements>,
        )>,
    ) -> Outcome {
        let mut inner = self.inner();
        let outcome = inner.publication.publish(ticket, refresh.snapshot);
        if outcome != Outcome::Stale {
            inner.measurements = Some(refresh.measurements);
            inner.error = None;
            if let Some((verification, overlay)) = recovery {
                inner.recovery.running = false;
                inner.recovery.completed += 1;
                inner.recovery.verification = Some(verification);
                inner.recovery.overlay = overlay;
                inner.recovery.error = None;
            }
            self.changes.update(status(&self.name, &inner));
        }
        outcome
    }
    pub(crate) fn recovery_started(&self) {
        let mut inner = self.inner();
        inner.recovery.running = true;
        inner.recovery.error = None;
        self.changes.update(status(&self.name, &inner));
    }
    pub(crate) fn recovery_discarded(&self, error: Option<String>) {
        let mut inner = self.inner();
        inner.recovery.running = false;
        inner.recovery.discarded += 1;
        inner.recovery.error = error;
        self.changes.update(status(&self.name, &inner));
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
        readiness: if inner.publication.current().is_some() {
            Readiness::Ready
        } else if inner.publication.pending() {
            Readiness::Initializing
        } else if inner.error.is_some() {
            Readiness::Failed
        } else {
            Readiness::NotStarted
        },
        recovery: inner.recovery.clone(),
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
    #[test]
    fn readiness_tracks_initial_attempts_and_a_completed_empty_observation() {
        let app = Application::new("readiness".into()).unwrap();
        assert_eq!(app.status().readiness, Readiness::NotStarted);
        let ticket = app.request();
        assert_eq!(app.status().readiness, Readiness::Initializing);
        app.fail(ticket, "initial failure".into());
        assert_eq!(app.status().readiness, Readiness::Failed);
        assert!(app.snapshot().is_none());
        let retry = app.request();
        assert_eq!(app.status().readiness, Readiness::Initializing);
        app.publish(retry, refresh(0));
        assert_eq!(app.status().readiness, Readiness::Ready);
        assert!(app.snapshot().unwrap().documents().is_empty());
        let later = app.request();
        assert_eq!(app.status().readiness, Readiness::Ready);
        app.fail(later, "later failure".into());
        assert_eq!(app.status().readiness, Readiness::Ready);
        assert!(app.snapshot().is_some());
    }
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

    #[test]
    fn recovery_metadata_is_independent_of_foreground_failure_and_stale_publication() {
        let app = Application::new("recovery".into()).unwrap();
        let first = app.request();
        app.publish(first, refresh(1));
        app.recovery_started();
        assert!(app.status().recovery.running);
        assert!(!app.status().pending);
        let next = app.request();
        app.fail(next, "foreground failed".into());
        app.recovery_discarded(Some("maintenance failed".into()));
        let state = app.status();
        assert!(!state.recovery.running);
        assert_eq!(state.recovery.discarded, 1);
        assert_eq!(state.recovery.error.as_deref(), Some("maintenance failed"));
        assert_eq!(state.error.as_deref(), Some("foreground failed"));
        app.recovery_started();
        assert_eq!(app.status().recovery.error, None);
        let before = app.status();
        assert_eq!(
            app.publish_recovery(first, refresh(99), Measurements::default(), None),
            Outcome::Stale
        );
        assert_eq!(app.status(), before);
        let overlay = refresh(3).measurements;
        let verification = refresh(2).measurements;
        assert_eq!(
            app.publish_recovery(
                next,
                refresh(3),
                verification.clone(),
                Some(overlay.clone())
            ),
            Outcome::Unchanged
        );
        let state = app.status();
        assert_eq!(state.recovery.completed, 1);
        assert_eq!(state.recovery.discarded, 1);
        assert!(!state.recovery.running);
        assert_eq!(state.recovery.verification, Some(verification));
        assert_eq!(state.recovery.overlay, Some(overlay));
        assert_eq!(state.error, None);
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
