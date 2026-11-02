//! Fresh disk verification may share immutable source, independently of facts.

use std::{collections::BTreeSet, sync::Arc};
use vizzi::{
    cache::Cache,
    engine::Engine,
    source::{CaptureScope, Directory},
};

#[test]
fn capture_verifies_changed_bytes_and_shares_unchanged_sources() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn alpha() {}").unwrap();
    std::fs::write(root.path().join("b.rs"), "fn bravo() {}").unwrap();
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().into()),
        2,
    )
    .unwrap();
    let first = engine.refresh(None, &|| false).unwrap().unwrap().snapshot;
    let same = engine.refresh(Some(&first), &|| false).unwrap().unwrap();
    assert_eq!(same.measurements.read_files, 2);
    assert_eq!(same.measurements.shared_sources, 2);
    for path in ["a.rs", "b.rs"] {
        assert!(Arc::ptr_eq(
            first.file_by_path(path).unwrap().captured(),
            same.snapshot.file_by_path(path).unwrap().captured()
        ));
    }
    std::fs::write(root.path().join("a.rs"), "fn delta() {}").unwrap();
    let changed = engine.refresh(Some(&first), &|| false).unwrap().unwrap();
    assert_eq!(changed.measurements.shared_sources, 1);
    assert_eq!(
        changed.snapshot.file_by_path("a.rs").unwrap().source(),
        Some("fn delta() {}")
    );
    assert_eq!(
        first.file_by_path("a.rs").unwrap().source(),
        Some("fn alpha() {}")
    );
    assert_eq!(
        changed.snapshot.id(),
        engine
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
    let directory = Directory::open(root.path()).unwrap();
    let capture = directory
        .capture_reusing(
            Some(&changed.snapshot),
            &CaptureScope::Paths(BTreeSet::from(["a.rs".into()])),
            &|| false,
        )
        .unwrap()
        .unwrap();
    assert_eq!(capture.files.len(), 1);
    assert!(Arc::ptr_eq(
        &capture.files[0].source,
        changed.snapshot.file_by_path("a.rs").unwrap().captured()
    ));
    assert!(
        directory
            .capture_reusing(Some(&first), &CaptureScope::Reconcile, &|| true)
            .unwrap()
            .is_none()
    );
}

#[test]
fn cancellation_during_capture_discards_partial_work_and_allows_a_clean_retry() {
    use std::cell::Cell;
    let root = tempfile::tempdir().unwrap();
    for index in 0..200 {
        std::fs::write(root.path().join(format!("{index}.rs")), "fn example() {}\n").unwrap();
    }
    let directory = Directory::open(root.path()).unwrap();
    for scope in [
        CaptureScope::Reconcile,
        CaptureScope::Paths((0..200).map(|index| format!("{index}.rs").into()).collect()),
    ] {
        let checks = Cell::new(0);
        let cancelled = directory
            .capture_reusing(None, &scope, &|| {
                checks.set(checks.get() + 1);
                checks.get() >= 70
            })
            .unwrap();
        assert!(cancelled.is_none());
        assert!(
            checks.get() <= 71,
            "capture continued polling after observing cancellation"
        );
        let completed = directory
            .capture_reusing(None, &scope, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(completed.files.len(), 200);
    }
}
