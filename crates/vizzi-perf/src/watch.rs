//! Event classification for the native watcher experiment.

use serde::Serialize;
use std::{collections::BTreeSet, path::PathBuf};

/// Candidate paths and an explicit request for a full reconciliation.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct ChangeBatch {
    /// Relative paths whose current contents should be checked.
    pub paths: BTreeSet<PathBuf>,
    /// Event loss or an ambiguous subtree change requires reconciliation.
    pub reconcile: bool,
}

/// Collect notifications for one source root without treating them as content proof.
pub struct PendingChanges {
    root: PathBuf,
    batch: ChangeBatch,
}

impl PendingChanges {
    /// Construct a collector for the canonical source root used by the watcher.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            batch: ChangeBatch::default(),
        }
    }

    /// Record a notification, ignoring read-only access and paths outside the root.
    pub fn record(&mut self, event: notify::Result<notify::Event>) {
        use notify::{
            EventKind,
            event::{AccessKind, AccessMode, CreateKind, RemoveKind},
        };
        let event = match event {
            Ok(event) => event,
            Err(_) => {
                self.batch.reconcile = true;
                return;
            }
        };
        self.batch.reconcile |= event.need_rescan();
        match event.kind {
            EventKind::Access(AccessKind::Close(AccessMode::Write)) => {}
            EventKind::Access(_) => return,
            _ => {}
        }
        if event.paths.is_empty() {
            self.batch.reconcile = true;
        }
        for path in event.paths {
            let Ok(relative) = path.strip_prefix(&self.root) else {
                continue;
            };
            if relative.as_os_str().is_empty() {
                self.batch.reconcile = true;
                continue;
            }
            self.batch.paths.insert(relative.to_path_buf());
            if matches!(
                event.kind,
                EventKind::Create(CreateKind::Folder) | EventKind::Remove(RemoveKind::Folder)
            ) {
                self.batch.reconcile = true;
            }
        }
    }

    /// Drain the candidates and reset the reconciliation request.
    pub fn take(&mut self) -> ChangeBatch {
        std::mem::take(&mut self.batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::{
        Event, EventKind,
        event::{AccessKind, AccessMode, CreateKind, Flag, ModifyKind, RenameMode},
    };

    #[test]
    fn deduplicates_relative_paths_and_ignores_outside_notifications() {
        let root = PathBuf::from("/workspace");
        let mut pending = PendingChanges::new(root.clone());
        for _ in 0..2 {
            pending.record(Ok(
                Event::new(EventKind::Create(CreateKind::File)).add_path(root.join("file.rs"))
            ));
        }
        pending.record(Ok(
            Event::new(EventKind::Any).add_path(PathBuf::from("/outside/file.rs"))
        ));
        assert_eq!(
            pending.take(),
            ChangeBatch {
                paths: BTreeSet::from([PathBuf::from("file.rs")]),
                reconcile: false
            }
        );
        assert_eq!(pending.take(), ChangeBatch::default());
    }

    #[test]
    fn rename_keeps_both_paths_and_write_close_counts_as_a_candidate() {
        let root = PathBuf::from("/workspace");
        let mut pending = PendingChanges::new(root.clone());
        pending.record(Ok(Event::new(EventKind::Modify(ModifyKind::Name(
            RenameMode::Both,
        )))
        .add_path(root.join("old.rs"))
        .add_path(root.join("new.rs"))));
        pending.record(Ok(Event::new(EventKind::Access(AccessKind::Close(
            AccessMode::Write,
        )))
        .add_path(root.join("saved.rs"))));
        assert_eq!(
            pending.take().paths,
            BTreeSet::from(["old.rs".into(), "new.rs".into(), "saved.rs".into()])
        );
    }

    #[test]
    fn reading_the_fingerprint_does_not_create_a_new_dirty_path() {
        let mut pending = PendingChanges::new("/workspace".into());
        pending
            .record(Ok(Event::new(EventKind::Access(AccessKind::Read))
                .add_path("/workspace/file.rs".into())));
        assert_eq!(pending.take(), ChangeBatch::default());
    }

    #[test]
    fn watcher_errors_and_rescan_flags_request_reconciliation() {
        let mut pending = PendingChanges::new("/workspace".into());
        pending.record(Err(notify::Error::generic("dropped events")));
        assert!(pending.take().reconcile);
        pending.record(Ok(Event::new(EventKind::Any)
            .set_flag(Flag::Rescan)
            .add_path("/workspace/file.rs".into())));
        assert!(pending.take().reconcile);
        assert!(!pending.take().reconcile);
    }

    #[test]
    fn ambiguous_and_directory_events_request_reconciliation() {
        let mut pending = PendingChanges::new("/workspace".into());
        pending.record(Ok(Event::new(EventKind::Any)));
        assert!(pending.take().reconcile);
        pending
            .record(Ok(Event::new(EventKind::Create(CreateKind::Folder))
                .add_path("/workspace/new-dir".into())));
        assert!(pending.take().reconcile);
        pending.record(Ok(
            Event::new(EventKind::Modify(ModifyKind::Any)).add_path("/workspace".into())
        ));
        assert!(pending.take().reconcile);
    }
}
