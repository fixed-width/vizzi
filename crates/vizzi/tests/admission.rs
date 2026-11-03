//! Shared analysis input admission and source-preserving deferral.

use std::sync::Arc;
use vizzi::admission::{Admission, Priority, Refusal};

#[test]
fn foreground_headroom_and_exact_limits_are_shared_and_release_on_drop() {
    let budget = Arc::new(Admission::new(100, 75, 25).unwrap());
    let recovery = budget.acquire(75, Priority::Recovery).unwrap();
    assert_eq!(
        budget.acquire(1, Priority::Recovery).err(),
        Some(Refusal::Busy)
    );
    let foreground = budget.acquire(25, Priority::Foreground).unwrap();
    assert_eq!(budget.stats().active_bytes, 100);
    assert_eq!(
        budget.acquire(1, Priority::Foreground).err(),
        Some(Refusal::Busy)
    );
    assert_eq!(
        budget.acquire(76, Priority::Foreground).err(),
        Some(Refusal::FileLimit)
    );
    assert_eq!(budget.stats().oversized, 1);
    assert_eq!(budget.stats().busy, 2);
    drop(recovery);
    assert_eq!(budget.stats().active_bytes, 25);
    assert_eq!(budget.stats().recovery_bytes, 0);
    let shared = budget.clone();
    let another = shared.acquire(75, Priority::Foreground).unwrap();
    assert_eq!(budget.stats().active_bytes, 100);
    assert_eq!(budget.stats().active_jobs, 2);
    drop(another);
    drop(foreground);
    assert_eq!(budget.stats().active_bytes, 0);
    assert_eq!(budget.stats().active_jobs, 0);
    assert_eq!(budget.stats().peak_bytes, 100);
}

#[test]
fn empty_inputs_use_a_slot_and_invalid_limits_are_rejected() {
    for (total, file, reserve) in [(0, 1, 0), (1, 0, 0), (100, 76, 25), (100, 1, 100)] {
        assert!(Admission::new(total, file, reserve).is_err());
    }
    let budget = Admission::new(1, 1, 0).unwrap();
    let empty = budget.acquire(0, Priority::Recovery).unwrap();
    assert_eq!(budget.stats().active_bytes, 1);
    assert!(budget.acquire(0, Priority::Foreground).is_err());
    drop(empty);
    assert!(budget.acquire(1, Priority::Recovery).is_ok());
}

#[test]
fn busy_analysis_publishes_truthful_source_and_retries_without_an_edit() {
    use vizzi::{cache::Cache, engine::Engine, source::Directory};
    use vizzi_core::model::Coverage;
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn original() {}").unwrap();
    let budget = Arc::new(Admission::new(128, 96, 32).unwrap());
    let first = budget.acquire(96, Priority::Foreground).unwrap();
    let second = budget.acquire(32, Priority::Foreground).unwrap();
    let make = || {
        Engine::new(
            Directory::open(root.path()).unwrap(),
            Cache::new(cache.path().into()),
            2,
        )
        .unwrap()
    };
    let limited = make().with_admission(budget.clone());
    let deferred = limited.refresh(None, &|| false).unwrap().unwrap();
    let file = deferred.snapshot.file_by_path("a.rs").unwrap();
    assert_eq!(file.resident_source(), Some("fn original() {}"));
    assert_eq!(file.analyses()[0].coverage, Coverage::Deferred);
    assert!(file.analyses()[0].facts.is_empty());
    assert_eq!(deferred.measurements.deferred, 1);
    assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 0);
    let view = deferred.snapshot.view(&Default::default()).unwrap();
    assert_eq!(view.files[0].coverage, Coverage::Deferred);
    assert_eq!(
        view.source.unwrap().text.as_deref(),
        Some("fn original() {}")
    );
    drop(first);
    drop(second);
    let completed = limited
        .refresh(Some(&deferred.snapshot), &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(completed.measurements.parsed, 1);
    assert_eq!(completed.measurements.deferred, 0);
    assert_eq!(
        completed.snapshot.file_by_path("a.rs").unwrap().analyses()[0].coverage,
        Coverage::Syntax
    );
    assert_eq!(
        completed.snapshot.id(),
        make()
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
    assert_eq!(
        file.analyses()[0].coverage,
        Coverage::Deferred,
        "old observations stay immutable"
    );
    assert_eq!(budget.stats().active_jobs, 0);
}

#[test]
fn oversized_inputs_keep_source_and_language_assignment_instead_of_stale_facts() {
    use vizzi::{cache::Cache, engine::Engine, source::Directory};
    use vizzi_core::model::Coverage;
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn original() {}").unwrap();
    let make = || {
        Engine::new(
            Directory::open(root.path()).unwrap(),
            Cache::new(cache.path().into()),
            1,
        )
        .unwrap()
    };
    let original = make().refresh(None, &|| false).unwrap().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn changed() {}").unwrap();
    let limited = make().with_admission(Arc::new(Admission::new(16, 8, 8).unwrap()));
    let result = limited
        .refresh(Some(&original.snapshot), &|| false)
        .unwrap()
        .unwrap();
    let file = result.snapshot.file_by_path("a.rs").unwrap();
    assert_eq!(file.resident_source(), Some("fn changed() {}"));
    assert_eq!(file.classification().assigned_language(), Some("rust"));
    assert_eq!(file.analyses()[0].coverage, Coverage::Deferred);
    assert!(file.analyses()[0].diagnostics[0].contains("per-file"));
    assert!(file.analyses()[0].facts.is_empty());
    assert_eq!(
        original.snapshot.file_by_path("a.rs").unwrap().analyses()[0].facts[0].name,
        "original"
    );
    let repeated = limited
        .refresh(Some(&result.snapshot), &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(repeated.snapshot.id(), result.snapshot.id());
    let complete = make()
        .refresh(Some(&result.snapshot), &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(complete.measurements.parsed, 1);
}

#[test]
fn parallel_permits_and_unwinding_cannot_leak_or_exceed_input_capacity() {
    use std::sync::Barrier;
    let budget = Arc::new(Admission::new(128, 96, 32).unwrap());
    let barrier = Arc::new(Barrier::new(8));
    std::thread::scope(|scope| {
        for index in 0..8 {
            let budget = budget.clone();
            let barrier = barrier.clone();
            scope.spawn(move || {
                barrier.wait();
                for _ in 0..40 {
                    let _permit = budget.acquire(
                        24,
                        if index % 2 == 0 {
                            Priority::Recovery
                        } else {
                            Priority::Foreground
                        },
                    );
                    let stats = budget.stats();
                    assert!(stats.active_bytes <= 128);
                    assert!(stats.recovery_bytes <= 96);
                }
            });
        }
    });
    let unwound = std::panic::catch_unwind(|| {
        let _permit = budget.acquire(96, Priority::Recovery).unwrap();
        panic!("controlled unwind");
    });
    assert!(unwound.is_err());
    assert_eq!(budget.stats().active_bytes, 0);
    assert_eq!(budget.stats().recovery_bytes, 0);
    assert_eq!(budget.stats().active_jobs, 0);
    assert!(budget.stats().peak_bytes <= 128);
    let configured = Admission::for_budget(100).unwrap().stats();
    assert_eq!(configured.capacity_bytes, 100);
    assert_eq!(configured.file_limit_bytes, 75);
    assert_eq!(configured.foreground_reserve_bytes, 25);
    assert!(Admission::for_budget(0).is_err());
}

#[test]
fn limits_apply_to_cached_work_but_complete_immutable_documents_need_no_permit() {
    use vizzi::{cache::Cache, engine::Engine, source::Directory};
    use vizzi_core::model::Coverage;
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn cached() {}").unwrap();
    let make = || {
        Engine::new(
            Directory::open(root.path()).unwrap(),
            Cache::new(cache.path().into()),
            1,
        )
        .unwrap()
    };
    let complete = make().refresh(None, &|| false).unwrap().unwrap();
    let limited = make().with_admission(Arc::new(Admission::new(16, 8, 8).unwrap()));
    let fresh = limited.refresh(None, &|| false).unwrap().unwrap();
    assert_eq!(
        fresh.snapshot.documents()[0].analyses()[0].coverage,
        Coverage::Deferred
    );
    assert_eq!(fresh.measurements.cache_hits, 0);
    let reused = limited
        .refresh(Some(&complete.snapshot), &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(reused.snapshot.id(), complete.snapshot.id());
    assert_eq!(reused.measurements.reused, 1);
    assert_eq!(reused.measurements.deferred, 0);
}
