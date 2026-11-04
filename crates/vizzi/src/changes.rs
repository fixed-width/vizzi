//! Bounded latest-state delivery, independent of any host transport.

use crate::application::Status;
use serde::{Serialize, Serializer};
use std::sync::Arc;
use tokio::sync::watch;

/// A complete status observation, versioned within one application lifetime.
#[derive(Clone, Debug, Serialize)]
pub struct Change {
    /// Version of the notification envelope.
    pub schema: u32,
    /// Opaque identity, replaced on application restart.
    pub stream: String,
    /// Monotonic status revision; serialised as a decimal string.
    #[serde(serialize_with = "decimal")]
    pub revision: u64,
    /// Latest coherent application metadata, without source or fact payloads.
    pub status: Status,
}
impl Change {
    /// Returns the transport cursor for this observation.
    pub fn id(&self) -> String {
        format!("{}:{}", self.stream, self.revision)
    }
}
fn decimal<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.to_string())
}

pub(crate) struct Changes {
    sender: watch::Sender<Option<Arc<Change>>>,
}
impl Changes {
    pub(crate) fn new(status: Status) -> Result<Self, getrandom::Error> {
        let mut nonce = [0; 32];
        getrandom::fill(&mut nonce)?;
        let stream = blake3::Hash::from_bytes(nonce).to_hex().to_string();
        let (sender, _) = watch::channel(Some(Arc::new(Change {
            schema: 1,
            stream,
            revision: 0,
            status,
        })));
        Ok(Self { sender })
    }
    pub(crate) fn update(&self, status: Status) {
        self.sender.send_if_modified(|slot| {
            let Some(previous) = slot else {
                return false;
            };
            if previous.status == status {
                return false;
            }
            *slot = Some(Arc::new(Change {
                schema: 1,
                stream: previous.stream.clone(),
                revision: previous.revision + 1,
                status,
            }));
            true
        });
    }
    pub(crate) fn subscribe(&self) -> Subscription {
        Subscription {
            receiver: self.sender.subscribe(),
            first: true,
            closed: false,
        }
    }
    pub(crate) fn close(&self) {
        self.sender.send_replace(None);
    }
}

/// An independent observer retaining at most the latest state.
pub struct Subscription {
    receiver: watch::Receiver<Option<Arc<Change>>>,
    first: bool,
    closed: bool,
}
impl Subscription {
    /// Returns current state immediately, then waits for newer state or closure.
    pub async fn next(&mut self) -> Option<Arc<Change>> {
        if self.closed {
            return None;
        }
        if self.first {
            self.first = false;
        } else if self.receiver.changed().await.is_err() {
            self.closed = true;
            return None;
        }
        let state = self.receiver.borrow_and_update().clone();
        self.closed = state.is_none();
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::FutureExt;
    fn status() -> Status {
        Status {
            readiness: crate::application::Readiness::NotStarted,
            recovery: crate::application::RecoveryStatus::default(),
            schema: 1,
            name: "fixture".into(),
            generation: 0,
            snapshot: None,
            pending: false,
            measurements: None,
            error: None,
            observation: "initializing".into(),
        }
    }
    #[tokio::test]
    async fn initial_state_is_immediate_and_cursors_are_restart_scoped() {
        let changes = Changes::new(status()).unwrap();
        let first = changes
            .subscribe()
            .next()
            .now_or_never()
            .expect("notification should be ready")
            .unwrap();
        assert_eq!(first.schema, 1);
        assert_eq!(first.revision, 0);
        assert_eq!(first.status, status());
        assert_eq!(first.id(), format!("{}:0", first.stream));
        let other = Changes::new(status())
            .unwrap()
            .subscribe()
            .next()
            .await
            .unwrap();
        assert_ne!(first.stream, other.stream);
        let maximum = Change {
            revision: u64::MAX,
            ..(*first).clone()
        };
        assert_eq!(
            serde_json::to_value(maximum).unwrap()["revision"],
            u64::MAX.to_string()
        );
    }
    #[tokio::test]
    async fn slow_and_new_subscribers_get_latest_without_blocking_or_replaying() {
        let changes = Changes::new(status()).unwrap();
        let mut slow = changes.subscribe();
        let mut fast = changes.subscribe();
        let initial = fast
            .next()
            .now_or_never()
            .expect("notification should be ready")
            .unwrap();
        for generation in 1..=100 {
            changes.update(Status {
                generation,
                ..status()
            });
            assert_eq!(
                fast.next()
                    .now_or_never()
                    .expect("notification should be ready")
                    .unwrap()
                    .status
                    .generation,
                generation
            );
        }
        let latest = slow
            .next()
            .now_or_never()
            .expect("notification should be ready")
            .unwrap();
        assert_eq!(latest.revision, 100);
        assert_eq!(latest.status.generation, 100);
        assert_eq!(initial.status.generation, 0);
        assert_eq!(
            changes
                .subscribe()
                .next()
                .now_or_never()
                .expect("notification should be ready")
                .unwrap()
                .revision,
            100
        );
        assert!(slow.next().now_or_never().is_none());
    }
    #[tokio::test]
    async fn duplicate_state_is_silent_and_shutdown_is_terminal() {
        let changes = Changes::new(status()).unwrap();
        let mut subscriber = changes.subscribe();
        subscriber
            .next()
            .now_or_never()
            .expect("notification should be ready")
            .unwrap();
        changes.update(status());
        assert!(subscriber.next().now_or_never().is_none());
        changes.update(Status {
            pending: true,
            ..status()
        });
        assert_eq!(
            subscriber
                .next()
                .now_or_never()
                .expect("notification should be ready")
                .unwrap()
                .revision,
            1
        );
        changes.close();
        assert!(
            subscriber
                .next()
                .now_or_never()
                .expect("notification should be ready")
                .is_none()
        );
        assert!(
            subscriber
                .next()
                .now_or_never()
                .expect("notification should be ready")
                .is_none()
        );
        changes.update(Status {
            generation: 10,
            ..status()
        });
        assert!(
            changes
                .subscribe()
                .next()
                .now_or_never()
                .expect("notification should be ready")
                .is_none()
        );
    }
    #[tokio::test]
    async fn updates_survive_zero_receivers_and_sender_drop_ends_subscription() {
        let changes = Changes::new(status()).unwrap();
        changes.update(Status {
            error: Some("capture failed".into()),
            ..status()
        });
        let mut subscriber = changes.subscribe();
        assert_eq!(
            subscriber
                .next()
                .now_or_never()
                .expect("notification should be ready")
                .unwrap()
                .status
                .error
                .as_deref(),
            Some("capture failed")
        );
        drop(changes);
        assert!(
            subscriber
                .next()
                .now_or_never()
                .expect("notification should be ready")
                .is_none()
        );
    }

    #[tokio::test]
    async fn a_waiting_subscriber_wakes_for_cross_task_publication() {
        let changes = Changes::new(status()).unwrap();
        let mut subscriber = changes.subscribe();
        subscriber.next().await.unwrap();
        let publisher = tokio::spawn(async move {
            tokio::task::yield_now().await;
            changes.update(Status {
                generation: 1,
                ..status()
            });
        });
        let next = tokio::time::timeout(std::time::Duration::from_secs(1), subscriber.next())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(next.status.generation, 1);
        publisher.await.unwrap();
    }
}
