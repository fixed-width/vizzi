//! Ordered publication with bounded immutable snapshot retention.

use crate::model::Snapshot;
use std::{collections::VecDeque, sync::Arc};

/// The outcome of attempting to publish completed background work.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A different observation was published.
    Published,
    /// The observation was already current.
    Unchanged,
    /// A newer refresh request superseded the completed work.
    Stale,
}

/// Metadata to protect with a short host-owned lock.
#[derive(Default)]
pub struct Publication {
    ticket: u64,
    generation: u64,
    pending: bool,
    current: Option<Arc<Snapshot>>,
    history: VecDeque<Arc<Snapshot>>,
}
impl Publication {
    /// Allocates a refresh ticket, invalidating earlier work.
    pub fn request(&mut self) -> u64 {
        self.ticket += 1;
        self.pending = true;
        self.ticket
    }
    /// Returns the latest requested ticket.
    pub fn ticket(&self) -> u64 {
        self.ticket
    }
    /// Returns the number of distinct observations published.
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Reports whether the latest request is still outstanding.
    pub fn pending(&self) -> bool {
        self.pending
    }
    /// Clones a handle to the latest immutable snapshot.
    pub fn current(&self) -> Option<Arc<Snapshot>> {
        self.current.clone()
    }
    /// Finds a current or retained snapshot; callers must handle expiration.
    pub fn pinned(&self, id: &str) -> Option<Arc<Snapshot>> {
        self.current
            .iter()
            .chain(self.history.iter())
            .find(|snapshot| snapshot.id().as_str() == id)
            .cloned()
    }
    /// Clears a failed latest request without discarding the published observation.
    pub fn failed(&mut self, ticket: u64) {
        if ticket == self.ticket {
            self.pending = false;
        }
    }
    /// Publishes only when no newer request has arrived.
    pub fn publish(&mut self, ticket: u64, snapshot: Arc<Snapshot>) -> Outcome {
        if ticket != self.ticket {
            return Outcome::Stale;
        }
        self.pending = false;
        if self
            .current
            .as_ref()
            .is_some_and(|old| old.id() == snapshot.id())
        {
            return Outcome::Unchanged;
        }
        if let Some(old) = self.current.replace(snapshot) {
            self.history.push_back(old);
        }
        if self.history.len() > 3 {
            self.history.pop_front();
        }
        self.generation += 1;
        Outcome::Published
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WorkspaceId;
    fn snapshot(text: &str) -> Arc<Snapshot> {
        let workspace = WorkspaceId::derive(&[b"test"]);
        let file = crate::model::fixture_document(
            workspace.clone(),
            "a.rs".into(),
            text.as_bytes(),
            vec![],
        )
        .unwrap();
        Arc::new(crate::model::fixture_snapshot(workspace, vec![Arc::new(file)]).unwrap())
    }
    #[test]
    fn obsolete_work_cannot_replace_newer_request() {
        let mut publication = Publication::default();
        let old = publication.request();
        let newer = publication.request();
        assert_eq!(publication.publish(old, snapshot("old")), Outcome::Stale);
        assert!(publication.current().is_none());
        assert!(publication.pending());
        assert_eq!(
            publication.publish(newer, snapshot("new")),
            Outcome::Published
        );
        assert!(!publication.pending());
        assert_eq!(publication.generation(), 1);
        let ticket = publication.request();
        assert_eq!(
            publication.publish(ticket, snapshot("new")),
            Outcome::Unchanged
        );
        assert_eq!(publication.generation(), 1);
    }
    #[test]
    fn pinned_source_survives_edits_and_expires_explicitly() {
        let mut publication = Publication::default();
        let first = snapshot("first");
        let ticket = publication.request();
        publication.publish(ticket, first.clone());
        for text in ["second", "third", "fourth"] {
            let ticket = publication.request();
            publication.publish(ticket, snapshot(text));
        }
        assert_eq!(
            publication.pinned(first.id().as_str()).unwrap().documents()[0].source(),
            Some("first")
        );
        let ticket = publication.request();
        publication.publish(ticket, snapshot("fifth"));
        assert!(publication.pinned(first.id().as_str()).is_none());
        assert_eq!(first.documents()[0].source(), Some("first"));
    }
    #[test]
    fn failures_clear_only_the_latest_request_and_preserve_the_generation() {
        let mut p = Publication::default();
        assert_eq!(p.ticket(), 0);
        assert_eq!(p.generation(), 0);
        let first = p.request();
        let latest = p.request();
        assert_eq!(p.ticket(), latest);
        assert!(latest > first);
        p.failed(first);
        assert!(p.pending());
        p.failed(latest);
        assert!(!p.pending());
        assert!(p.current().is_none());
        let next = p.request();
        p.publish(next, snapshot("ok"));
        let next = p.request();
        p.failed(next);
        assert_eq!(p.generation(), 1);
        assert!(p.current().is_some());
    }
}
