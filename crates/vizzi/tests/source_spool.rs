//! Immutable spool capture keeps old source independent of workspace and host lifetime.
use std::sync::Arc;
use vizzi::{cache::Cache, engine::Engine, source::Directory, source_spool::SourceSpool};
use vizzi_core::{
    model::BULK_SOURCE_LIMIT,
    projection::WindowQuery,
    query::{Projection, ViewQuery},
};
#[test]
fn spooled_source_survives_project_deletion_and_loads_one_verified_analysis_input() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let spool = Arc::new(SourceSpool::new(external.path(), 1024 * 1024).unwrap());
    std::fs::write(root.path().join("a.rs"), "// α\r\nfn café() {}\n").unwrap();
    let directory = Directory::open(root.path())
        .unwrap()
        .with_spool(spool.clone())
        .unwrap();
    let engine = Engine::new(directory, Cache::new(cache.path().into()), 2).unwrap();
    let first = engine.refresh(None, &|| false).unwrap().unwrap();
    let file = first.snapshot.file_by_path("a.rs").unwrap();
    assert_eq!(file.captured().storage().byte_capacity, 0);
    assert_eq!(first.measurements.parsed, 1);
    assert_eq!(spool.stats().stores, 1);
    assert_eq!(spool.stats().reads, 1);
    assert_eq!(spool.stats().read_bytes, "// α\r\nfn café() {}\n".len());
    let repeat = engine
        .refresh(Some(&first.snapshot), &|| false)
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(
        file.captured(),
        repeat.snapshot.documents()[0].captured()
    ));
    assert_eq!(spool.stats().written_bytes, "// α\r\nfn café() {}\n".len());
    let window = file.window(&WindowQuery::default(), None).unwrap();
    assert_eq!(window.lines[0].text, "// α");
    assert_eq!(window.lines[1].text, "fn café() {}");
    let view = first
        .snapshot
        .view(&ViewQuery {
            projection: Some(Projection::Summary),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(view.source.unwrap().text, None);
    std::fs::write(root.path().join("a.rs"), "fn changed() {}\n").unwrap();
    let next = engine
        .refresh(Some(&first.snapshot), &|| false)
        .unwrap()
        .unwrap();
    assert_ne!(next.snapshot.id(), first.snapshot.id());
    drop(engine);
    drop(spool);
    drop(root);
    assert_eq!(
        file.captured()
            .read_text(BULK_SOURCE_LIMIT)
            .unwrap()
            .as_deref(),
        Some("// α\r\nfn café() {}\n")
    );
    assert_eq!(file.analyses()[0].facts[0].name, "café");
}
#[test]
fn spool_capacity_failure_keeps_the_last_published_observation_readable() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let spool = Arc::new(SourceSpool::new(external.path(), 20).unwrap());
    let directory = Directory::open(root.path())
        .unwrap()
        .with_spool(spool.clone())
        .unwrap();
    std::fs::write(root.path().join("a.rs"), "fn first() {}").unwrap();
    let first = directory.capture().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn much_larger_replacement() {}").unwrap();
    assert!(directory.capture().is_err());
    assert_eq!(
        first.files[0].source.read_text(100).unwrap().as_deref(),
        Some("fn first() {}")
    );
    assert_eq!(spool.stats().written_bytes, "fn first() {}".len());
    assert!(SourceSpool::new(external.path(), 0).is_err());
}

#[test]
fn concurrently_reading_pins_does_not_share_a_mutable_file_cursor() {
    let root = tempfile::tempdir().unwrap();
    let spool = SourceSpool::new(root.path(), 1024 * 1024).unwrap();
    let a = spool
        .persist(vizzi_core::model::CapturedSource::capture(
            "α".repeat(100000).into_bytes(),
            None,
        ))
        .unwrap();
    let b = spool
        .persist(vizzi_core::model::CapturedSource::capture(
            "🙂".repeat(100000).into_bytes(),
            None,
        ))
        .unwrap();
    std::thread::scope(|scope| {
        for index in 0..8 {
            let a = a.clone();
            let b = b.clone();
            scope.spawn(move || {
                for _ in 0..8 {
                    if index % 2 == 0 {
                        assert_eq!(
                            a.reader().read(65534..65538, 4).unwrap().as_ref(),
                            "αα".as_bytes()
                        );
                    } else {
                        assert_eq!(
                            b.reader().read(65532..65540, 8).unwrap().as_ref(),
                            "🙂🙂".as_bytes()
                        );
                    }
                }
            });
        }
    });
    assert_eq!(spool.stats().reads, 128);
    assert_eq!(spool.stats().read_bytes, 128 * 65536);
}

#[tokio::test]
async fn summary_needs_no_backing_io_and_http_reports_read_errors_explicitly() {
    use vizzi::{
        application::Application,
        engine::{Measurements, Refresh},
        server,
    };
    use vizzi_core::{
        model::{
            CapturePolicyId, CapturedSource, Document, Snapshot, SourceReadError, SourceReader,
            WorkspaceId,
        },
        policy::{Classification, Provenance, SourceOnlyReason},
    };
    #[derive(Debug)]
    struct Broken;
    impl SourceReader for Broken {
        fn read_at(&self, _: usize, _: &mut [u8]) -> Result<(), SourceReadError> {
            Err(SourceReadError::Io("fixture".into()))
        }
    }
    let source = CapturedSource::capture(b"readable captured text".to_vec(), None)
        .with_backing(Arc::new(Broken));
    let workspace = WorkspaceId::derive(&[b"broken fixture"]);
    let file = Arc::new(
        Document::capture_source(
            workspace.clone(),
            "a.h".into(),
            source,
            vec![],
            Classification::SourceOnly {
                language: None,
                provenance: Provenance::Extension {
                    extension: "h".into(),
                },
                reason: SourceOnlyReason::LanguageUnassigned,
            },
        )
        .unwrap(),
    );
    let snapshot = Arc::new(
        Snapshot::new(
            workspace,
            vec![file.clone()],
            CapturePolicyId::derive(&[b"fixture"]),
        )
        .unwrap(),
    );
    let app = Arc::new(Application::new("fixture".into()).unwrap());
    let ticket = app.request();
    app.publish(
        ticket,
        Refresh {
            snapshot: snapshot.clone(),
            directories: None,
            measurements: Measurements::default(),
        },
    );
    let router = server::router(app);
    let summary = server::diagnostic(router.clone(), 0.into(), "/api/view?projection=summary")
        .await
        .unwrap();
    assert_eq!(summary.status, 200);
    let uri = format!(
        "/api/window?snapshot={}&file={}&version={}",
        snapshot.id().as_str(),
        file.id().as_str(),
        file.version().as_str()
    );
    let failure = server::diagnostic(router.clone(), 1.into(), &uri)
        .await
        .unwrap();
    assert_eq!(failure.status, 503);
    assert!(failure.body.contains("IO"));
    let bulk = server::diagnostic(router, 2.into(), "/api/view")
        .await
        .unwrap();
    assert_eq!(bulk.status, 503);
}

#[tokio::test]
async fn bulk_source_capacity_is_an_http_error_while_summary_and_windows_still_work() {
    use vizzi::{
        application::Application,
        engine::{Measurements, Refresh},
        server,
    };
    use vizzi_core::{
        model::{CapturePolicyId, CapturedSource, Document, Snapshot, WorkspaceId},
        policy::{Classification, Provenance, SourceOnlyReason},
    };
    let workspace = WorkspaceId::derive(&[b"large bulk fixture"]);
    let source = CapturedSource::capture(vec![b'x'; 32 * 1024 * 1024 + 1], None);
    let file = Arc::new(
        Document::capture_source(
            workspace.clone(),
            "a.h".into(),
            source,
            vec![],
            Classification::SourceOnly {
                language: None,
                provenance: Provenance::Extension {
                    extension: "h".into(),
                },
                reason: SourceOnlyReason::LanguageUnassigned,
            },
        )
        .unwrap(),
    );
    let valid = Arc::new(
        Document::capture_source(
            workspace.clone(),
            "b.h".into(),
            CapturedSource::capture(vec![b'x'; 32 * 1024 * 1024], None),
            vec![],
            file.classification().clone(),
        )
        .unwrap(),
    );
    let snapshot = Arc::new(
        Snapshot::new(
            workspace,
            vec![file.clone(), valid.clone()],
            CapturePolicyId::derive(&[b"fixture"]),
        )
        .unwrap(),
    );
    let app = Arc::new(Application::new("fixture".into()).unwrap());
    let ticket = app.request();
    app.publish(
        ticket,
        Refresh {
            snapshot: snapshot.clone(),
            directories: None,
            measurements: Measurements::default(),
        },
    );
    let router = server::router(app);
    assert_eq!(
        server::diagnostic(router.clone(), 0.into(), "/api/view?projection=summary")
            .await
            .unwrap()
            .status,
        200
    );
    assert_eq!(
        server::diagnostic(router.clone(), 0.into(), "/api/view")
            .await
            .unwrap()
            .status,
        413
    );
    let accepted = server::diagnostic(
        router.clone(),
        0.into(),
        &format!(
            "/api/source?snapshot={}&file={}",
            snapshot.id().as_str(),
            valid.id().as_str()
        ),
    )
    .await
    .unwrap();
    assert_eq!(accepted.status, 200);
    assert!(accepted.body.len() > 32 * 1024 * 1024);
    let pin = format!(
        "snapshot={}&file={}",
        snapshot.id().as_str(),
        file.id().as_str()
    );
    assert_eq!(
        server::diagnostic(router.clone(), 0.into(), &format!("/api/source?{pin}"))
            .await
            .unwrap()
            .status,
        413
    );
    let window = server::diagnostic(
        router,
        0.into(),
        &format!("/api/window?{pin}&version={}", file.version().as_str()),
    )
    .await
    .unwrap();
    assert_eq!(window.status, 200);
    assert!(window.body.len() < 512 * 1024);
}
