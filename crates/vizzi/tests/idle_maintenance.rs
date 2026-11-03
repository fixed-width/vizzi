//! A real host notices retired backing during idle time without another source hint.
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use vizzi::{
    cache::Cache,
    engine::Engine,
    host::{Host, HostOptions},
    source::Directory,
    source_spool::SourceSpool,
};
use vizzi_core::model::CapturedSource;

#[test]
fn idle_host_reclaims_an_external_tail_without_changing_publication() {
    let owned = tempfile::tempdir().unwrap();
    let source = owned.path().join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("a.rs"), "fn original() {}\n").unwrap();
    let spool = Arc::new(SourceSpool::new(&owned.path().join("spool"), 2 * 1024 * 1024).unwrap());
    let engine = Engine::new(
        Directory::open(&source)
            .unwrap()
            .with_spool(spool.clone())
            .unwrap(),
        Cache::new(owned.path().join("cache")),
        4,
    )
    .unwrap();
    let host = Host::start_with_options(
        engine,
        HostOptions {
            watch: false,
            reconcile_interval: Duration::from_secs(3600),
            idle_maintenance: true,
        },
    )
    .unwrap();
    let app = host.application();
    let original = app.snapshot().unwrap();
    let tail = spool
        .persist(CapturedSource::capture(vec![b'x'; 1024 * 1024], None))
        .unwrap();
    drop(tail);
    let started = Instant::now();
    while host.maintenance_status().completed == 0 && started.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(host.maintenance_status().completed > 0);
    assert!(host.maintenance_status().error.is_none());
    assert_eq!(
        spool.stats().file_bytes,
        Some("fn original() {}\n".len() as u64)
    );
    assert_eq!(app.snapshot().unwrap().id(), original.id());
    assert!(app.status().error.is_none());
    drop(host);
    assert_eq!(
        original.documents()[0]
            .captured()
            .read_text(100)
            .unwrap()
            .as_deref(),
        Some("fn original() {}\n")
    );
}

#[test]
fn disabled_idle_maintenance_leaves_cleanup_to_the_caller() {
    let owned = tempfile::tempdir().unwrap();
    let source = owned.path().join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("a.rs"), "fn original() {}\n").unwrap();
    let spool = Arc::new(SourceSpool::new(&owned.path().join("spool"), 1024).unwrap());
    let engine = Engine::new(
        Directory::open(&source)
            .unwrap()
            .with_spool(spool.clone())
            .unwrap(),
        Cache::new(owned.path().join("cache")),
        4,
    )
    .unwrap();
    let host = Host::start_with_options(
        engine,
        HostOptions {
            watch: false,
            reconcile_interval: Duration::from_secs(3600),
            idle_maintenance: false,
        },
    )
    .unwrap();
    let tail = spool
        .persist(CapturedSource::capture(vec![b'x'; 100], None))
        .unwrap();
    drop(tail);
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(host.maintenance_status().attempts, 0);
    assert!(spool.stats().file_bytes.unwrap() > "fn original() {}\n".len() as u64);
    spool.reclaim().unwrap();
    assert_eq!(
        spool.stats().file_bytes,
        Some("fn original() {}\n".len() as u64)
    );
    drop(host);
}
