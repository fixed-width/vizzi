//! Filesystem hints and acknowledgement of refresh work.

use crate::{
    application::Application,
    engine::Refresh,
    source::{CaptureScope, Directory, MAX_PATH_HINTS},
};
use std::collections::BTreeSet;
use vizzi_core::publication::Outcome;

pub(crate) fn classify(
    directory: &Directory,
    event: notify::Result<notify::Event>,
) -> Option<CaptureScope> {
    use notify::{
        EventKind,
        event::{AccessKind, AccessMode, CreateKind, ModifyKind, RemoveKind, RenameMode},
    };
    let event = match event {
        Ok(event) => event,
        Err(_) => return Some(CaptureScope::Reconcile),
    };
    if event.need_rescan() {
        return Some(CaptureScope::Reconcile);
    }
    if matches!(event.kind, EventKind::Access(_))
        && !matches!(
            event.kind,
            EventKind::Access(AccessKind::Close(AccessMode::Write))
        )
    {
        return None;
    }
    if event.paths.is_empty() {
        return Some(CaptureScope::Reconcile);
    }
    let mut paths: BTreeSet<_> = event
        .paths
        .iter()
        .filter_map(|path| directory.event_path(path))
        .collect();
    if paths.is_empty() {
        return None;
    }
    if paths.iter().any(|path| path.as_os_str().is_empty()) {
        return Some(CaptureScope::Reconcile);
    }
    match event.kind {
        EventKind::Create(CreateKind::File)
        | EventKind::Remove(RemoveKind::File)
        | EventKind::Modify(ModifyKind::Data(_))
        | EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
            paths.retain(|path| directory.source_path(path))
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if event.paths.len() == 2 => {}
        EventKind::Modify(ModifyKind::Name(RenameMode::From | RenameMode::To))
            if event.paths.len() == 1 => {}
        EventKind::Modify(ModifyKind::Metadata(_)) => {}
        _ => return Some(CaptureScope::Reconcile),
    }
    if paths.is_empty() {
        None
    } else {
        Some(CaptureScope::Paths(paths))
    }
}

#[derive(Default)]
pub(crate) struct PendingRefresh {
    scope: Option<CaptureScope>,
    ticket: u64,
}
pub(crate) struct Plan {
    pub ticket: u64,
    pub scope: CaptureScope,
}
impl Plan {
    pub(crate) fn cancelled(&self, app: &Application, stopped: bool) -> bool {
        stopped || app.desired() != self.ticket
    }
}
impl PendingRefresh {
    pub(crate) fn invalidate(&mut self, app: &Application, scope: CaptureScope) {
        let scope = match scope {
            CaptureScope::Paths(paths) if paths.len() > MAX_PATH_HINTS => CaptureScope::Reconcile,
            other => other,
        };
        match &mut self.scope {
            Some(current) => current.merge(scope),
            None => self.scope = Some(scope),
        }
        self.ticket = app.request();
    }
    pub(crate) fn plan(&self, app: &Application) -> Plan {
        let ticket = app.desired();
        Plan {
            ticket,
            scope: if ticket == self.ticket {
                self.scope.clone().unwrap_or_default()
            } else {
                CaptureScope::Reconcile
            },
        }
    }
    pub(crate) fn publish(&mut self, app: &Application, ticket: u64, refresh: Refresh) -> Outcome {
        let outcome = app.publish(ticket, refresh);
        if outcome != Outcome::Stale {
            self.scope = None;
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cache::Cache, engine::Engine, source::MAX_PATH_HINTS};
    use notify::{
        Event, EventKind,
        event::{AccessKind, AccessMode, CreateKind, DataChange, Flag, ModifyKind, RenameMode},
    };
    fn paths(names: &[&str]) -> CaptureScope {
        CaptureScope::Paths(names.iter().map(Into::into).collect())
    }
    #[test]
    fn writes_complete_renames_and_write_close_are_candidates() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Directory::open(temp.path()).unwrap();
        for kind in [
            EventKind::Modify(ModifyKind::Data(DataChange::Any)),
            EventKind::Access(AccessKind::Close(AccessMode::Write)),
            EventKind::Create(CreateKind::File),
        ] {
            assert_eq!(
                classify(&dir, Ok(Event::new(kind).add_path(dir.root().join("a.rs")))),
                Some(paths(&["a.rs"]))
            );
            assert_eq!(
                classify(&dir, Ok(Event::new(kind).add_path(dir.root().join("b.py")))),
                Some(paths(&["b.py"]))
            );
            assert!(
                classify(
                    &dir,
                    Ok(Event::new(kind).add_path(dir.root().join("notes.txt")))
                )
                .is_none()
            );
        }
        let rename = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(dir.root().join("tmp"))
            .add_path(dir.root().join("a.rs"));
        assert_eq!(classify(&dir, Ok(rename)), Some(paths(&["tmp", "a.rs"])));
        for mode in [RenameMode::From, RenameMode::To] {
            assert_eq!(
                classify(
                    &dir,
                    Ok(Event::new(EventKind::Modify(ModifyKind::Name(mode)))
                        .add_path(dir.root().join("a.rs")))
                ),
                Some(paths(&["a.rs"]))
            );
        }
    }
    #[test]
    fn ambiguous_and_directory_changes_require_reconciliation_but_reads_are_silent() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Directory::open(temp.path()).unwrap();
        for kind in [
            EventKind::Any,
            EventKind::Create(CreateKind::Folder),
            EventKind::Modify(ModifyKind::Name(RenameMode::Any)),
        ] {
            assert_eq!(
                classify(
                    &dir,
                    Ok(Event::new(kind).add_path(dir.root().join("dotted.folder")))
                ),
                Some(CaptureScope::Reconcile)
            );
        }
        assert_eq!(
            classify(&dir, Err(notify::Error::generic("overflow"))),
            Some(CaptureScope::Reconcile)
        );
        assert_eq!(
            classify(
                &dir,
                Ok(Event::new(EventKind::Access(AccessKind::Read)).set_flag(Flag::Rescan))
            ),
            Some(CaptureScope::Reconcile)
        );
        assert!(
            classify(
                &dir,
                Ok(Event::new(EventKind::Access(AccessKind::Read))
                    .add_path(dir.root().join("a.rs")))
            )
            .is_none()
        );
        assert!(
            classify(
                &dir,
                Ok(Event::new(EventKind::Any).add_path(dir.root().join("target/a.rs")))
            )
            .is_none()
        );
        assert!(
            classify(
                &dir,
                Ok(Event::new(EventKind::Any)
                    .add_path(temp.path().parent().unwrap().join("outside.rs")))
            )
            .is_none()
        );
    }
    #[test]
    fn cancelled_and_superseded_work_cannot_acknowledge_dirty_paths() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        for name in ["a", "b"] {
            std::fs::write(
                source.path().join(format!("{name}.rs")),
                format!("fn {name}() {{}}"),
            )
            .unwrap();
        }
        let engine = Engine::new(
            Directory::open(source.path()).unwrap(),
            Cache::new(cache.path().into()),
            1,
        )
        .unwrap();
        let app = Application::new("queue".into()).unwrap();
        let mut queue = PendingRefresh::default();
        queue.invalidate(&app, CaptureScope::Reconcile);
        let initial = queue.plan(&app);
        queue.publish(
            &app,
            initial.ticket,
            engine.refresh(None, &|| false).unwrap().unwrap(),
        );
        let base = app.snapshot().unwrap();
        std::fs::write(source.path().join("a.rs"), "fn changed_a() {}").unwrap();
        queue.invalidate(&app, paths(&["a.rs"]));
        let first = queue.plan(&app);
        assert!(
            engine
                .refresh_scoped(Some(&base), &first.scope, &|| true)
                .unwrap()
                .is_none()
        );
        let candidate = engine
            .refresh_scoped(Some(&base), &first.scope, &|| false)
            .unwrap()
            .unwrap();
        std::fs::write(source.path().join("b.rs"), "fn changed_b() {}").unwrap();
        queue.invalidate(&app, paths(&["b.rs"]));
        assert_eq!(queue.publish(&app, first.ticket, candidate), Outcome::Stale);
        let latest = queue.plan(&app);
        assert_eq!(latest.scope, paths(&["a.rs", "b.rs"]));
        let candidate = engine
            .refresh_scoped(Some(&base), &latest.scope, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(
            queue.publish(&app, latest.ticket, candidate),
            Outcome::Published
        );
        let expected = engine.refresh(None, &|| false).unwrap().unwrap();
        assert_eq!(app.snapshot().unwrap().id(), expected.snapshot.id());
        assert_eq!(
            app.snapshot().unwrap().documents()[0].source(),
            Some("fn changed_a() {}")
        );
        assert_eq!(queue.plan(&app).scope, CaptureScope::Reconcile);
    }
    #[test]
    fn duplicate_events_invalidate_work_overflow_is_bounded_and_manual_refresh_is_full() {
        let app = Application::new("queue".into()).unwrap();
        let mut queue = PendingRefresh::default();
        queue.invalidate(&app, paths(&["a.rs"]));
        let first = queue.plan(&app).ticket;
        queue.invalidate(&app, paths(&["a.rs"]));
        assert!(queue.plan(&app).ticket > first);
        assert_eq!(queue.plan(&app).scope, paths(&["a.rs"]));
        app.request();
        assert_eq!(queue.plan(&app).scope, CaptureScope::Reconcile);
        queue.invalidate(
            &app,
            CaptureScope::Paths(
                (0..MAX_PATH_HINTS)
                    .map(|i| format!("f{i}.rs").into())
                    .collect(),
            ),
        );
        assert_eq!(queue.plan(&app).scope, CaptureScope::Reconcile);
        queue.invalidate(&app, paths(&["more.rs"]));
        assert_eq!(queue.plan(&app).scope, CaptureScope::Reconcile);
    }

    #[test]
    fn failed_work_retains_candidates_and_an_initial_large_batch_is_bounded() {
        let app = Application::new("queue".into()).unwrap();
        let mut queue = PendingRefresh::default();
        queue.invalidate(&app, paths(&["a.rs"]));
        let ticket = queue.plan(&app).ticket;
        app.fail(ticket, "capture failed".into());
        assert_eq!(queue.plan(&app).scope, paths(&["a.rs"]));
        queue.invalidate(&app, paths(&["b.rs"]));
        assert_eq!(queue.plan(&app).scope, paths(&["a.rs", "b.rs"]));
        let mut queue = PendingRefresh::default();
        queue.invalidate(
            &app,
            CaptureScope::Paths(
                (0..=MAX_PATH_HINTS)
                    .map(|i| format!("f{i}.rs").into())
                    .collect(),
            ),
        );
        assert_eq!(queue.plan(&app).scope, CaptureScope::Reconcile);
    }

    #[test]
    fn exact_hint_capacity_is_allowed_and_plans_expire_on_stop_or_new_requests() {
        let app = Application::new("queue".into()).unwrap();
        let mut queue = PendingRefresh::default();
        queue.invalidate(
            &app,
            CaptureScope::Paths(
                (0..MAX_PATH_HINTS)
                    .map(|i| format!("f{i}.rs").into())
                    .collect(),
            ),
        );
        let plan = queue.plan(&app);
        assert!(matches!(plan.scope, CaptureScope::Paths(ref p) if p.len() == MAX_PATH_HINTS));
        assert!(!plan.cancelled(&app, false));
        assert!(plan.cancelled(&app, true));
        app.request();
        assert!(plan.cancelled(&app, false));
    }
    #[test]
    fn metadata_is_a_candidate_but_malformed_rename_shapes_reconcile() {
        let root = tempfile::tempdir().unwrap();
        let directory = Directory::open(root.path()).unwrap();
        let metadata = Event::new(EventKind::Modify(ModifyKind::Metadata(
            notify::event::MetadataKind::Any,
        )))
        .add_path(root.path().join("a.rs"));
        assert_eq!(classify(&directory, Ok(metadata)), Some(paths(&["a.rs"])));
        let both = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(root.path().join("a.rs"));
        assert_eq!(
            classify(&directory, Ok(both)),
            Some(CaptureScope::Reconcile)
        );
        let from = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::From)))
            .add_path(root.path().join("a.rs"))
            .add_path(root.path().join("b.rs"));
        assert_eq!(
            classify(&directory, Ok(from)),
            Some(CaptureScope::Reconcile)
        );
    }
}
