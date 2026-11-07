//! Question results use current captured documents and existing investigation/editor bindings.
use std::sync::Arc;
use vizzi::{
    cache::Cache,
    engine::Engine,
    host::{Host, HostOptions},
    question_search::{QuestionSearch, SearchRequest, Selector},
    source::Directory,
};
fn setup() -> (tempfile::TempDir, tempfile::TempDir, Arc<Host>) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("item.rs"),"struct Parcel; impl Parcel { fn new() -> Self { Self } }\nfn open_attempt() { let parcel = Parcel::new(); }\n").unwrap();
    start(root)
}
fn start(root: tempfile::TempDir) -> (tempfile::TempDir, tempfile::TempDir, Arc<Host>) {
    let cache = tempfile::tempdir().unwrap();
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().join("analysis")),
        1,
    )
    .unwrap();
    let host = Arc::new(
        Host::start_with_options(
            engine,
            HostOptions {
                watch: false,
                reconcile_interval: std::time::Duration::from_secs(3600),
                idle_maintenance: false,
            },
        )
        .unwrap(),
    );
    (root, cache, host)
}
#[test]
fn focused_results_and_original_matches_keep_native_document_locations() {
    let (_root, _cache, host) = setup();
    let snapshot = host.application().snapshot().unwrap();
    let trial = QuestionSearch::new(host.clone());
    let request = SearchRequest {
        snapshot: snapshot.id().as_str().into(),
        question: "where are new parcels created?".into(),
        selector: Selector::Focused,
    };
    let found = trial.search(&request).unwrap();
    assert!(!found.results.is_empty());
    assert_eq!(found.snapshot, request.snapshot);
    assert_eq!(found.searched_files, 1);
    assert_eq!(found.available_files, 1);
    for hit in &found.results {
        let doc = snapshot.file(&hit.file).unwrap();
        assert_eq!(doc.version().as_str(), hit.version);
        assert_eq!(doc.path(), hit.path);
        assert!(hit.line > 0);
        assert!(hit.suggested_role.is_some());
    }
    assert!(found.results.iter().map(|e| e.excerpt.len()).sum::<usize>() <= 4000);
    let original = trial
        .search(&SearchRequest {
            selector: Selector::Original,
            ..request
        })
        .unwrap();
    assert!(original.results.iter().all(|e| e.suggested_role.is_none()));
}
#[test]
fn invalid_or_stale_questions_do_not_publish_current_results() {
    let (_root, _cache, host) = setup();
    let snapshot = host.application().snapshot().unwrap();
    let trial = QuestionSearch::new(host);
    for (version, question) in [
        ("stale".to_owned(), "where are parcels created?".into()),
        (snapshot.id().as_str().into(), " \n".into()),
        (snapshot.id().as_str().into(), "x".repeat(2049)),
    ] {
        assert!(
            trial
                .search(&SearchRequest {
                    snapshot: version,
                    question,
                    selector: Selector::Focused
                })
                .is_err()
        );
    }
    let empty = trial
        .search(&SearchRequest {
            snapshot: snapshot.id().as_str().into(),
            question: "where are new missingwidgets created?".into(),
            selector: Selector::Focused,
        })
        .unwrap();
    assert!(empty.results.is_empty());
}

#[test]
fn changed_capture_invalidates_results_and_current_only_pins() {
    let (root, _cache, host) = setup();
    let trial = QuestionSearch::new(host.clone());
    let old = host.application().snapshot().unwrap();
    let request = SearchRequest {
        snapshot: old.id().as_str().into(),
        question: "where are new parcels created?".into(),
        selector: Selector::Focused,
    };
    let result = trial.search(&request).unwrap();
    let hit = &result.results[0];
    std::fs::write(root.path().join("item.rs"), "fn replacement() {}\n").unwrap();
    host.request(vizzi::source::CaptureScope::Reconcile);
    let start = std::time::Instant::now();
    while host.application().snapshot().unwrap().id() == old.id() {
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(trial.search(&request).is_err());
    let next = trial
        .search(&SearchRequest {
            snapshot: host.application().snapshot().unwrap().id().as_str().into(),
            ..request
        })
        .unwrap();
    assert!(next.results.is_empty());
    let app = vizzi::companion::Companion::new(
        host,
        None,
        Arc::new(vizzi::editor::VsCode::new("not-invoked".into())),
    )
    .unwrap();
    assert!(
        app.pin_current(vizzi::companion::PinEvidence {
            revision: 0,
            snapshot: result.snapshot,
            file: hit.file.clone(),
            version: hit.version.clone(),
            line: hit.line,
            title: hit.title.clone()
        })
        .is_err()
    );
    assert!(app.view().board.evidence.is_empty());
}

#[test]
fn unsupported_source_is_counted_as_an_omission() {
    let (root, _cache, host) = setup();
    std::fs::write(root.path().join("binary.rs"), [255]).unwrap();
    let before = host.application().snapshot().unwrap().id().clone();
    host.request(vizzi::source::CaptureScope::Reconcile);
    let start = std::time::Instant::now();
    while host.application().snapshot().unwrap().id() == &before {
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let trial = QuestionSearch::new(host.clone());
    let result = trial
        .search(&SearchRequest {
            snapshot: host.application().snapshot().unwrap().id().as_str().into(),
            question: "where are new parcels created?".into(),
            selector: Selector::Focused,
        })
        .unwrap();
    assert_eq!(result.available_files, 2);
    assert_eq!(result.searched_files, 1);
    assert_eq!(result.omitted_files, 1);
}

#[tokio::test]
async fn http_search_checks_schema_bounds_staleness_and_native_pin_bindings() {
    use serde_json::{Value, json};
    use vizzi::{
        companion::Companion,
        editor::VsCode,
        review_application::ReviewApplication,
        review_source::CaptureLimits,
        server::{diagnostic, diagnostic_request, router_with_companion},
    };
    let (_root, _cache, host) = setup();
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let app = Arc::new(
        Companion::new(
            host.clone(),
            None,
            Arc::new(VsCode::new("not-invoked".into())),
        )
        .unwrap(),
    );
    let reviews =
        Arc::new(ReviewApplication::start(host.clone(), CaptureLimits::default()).unwrap());
    let router = router_with_companion(host, reviews, app.clone());
    let capability = diagnostic(router.clone(), json!(1), "/api/search")
        .await
        .unwrap();
    assert_eq!(capability.status, 200);
    assert_eq!(
        serde_json::from_str::<Value>(&capability.body).unwrap()["result_limit"],
        8
    );
    let query = json!({"snapshot":snapshot,"question":"where are new parcels created?","selector":"focused"});
    for (body, expected) in [
        (
            json!({"snapshot":"stale","question":"where?","selector":"focused"}),
            409,
        ),
        (
            json!({"snapshot":snapshot,"question":"","selector":"focused"}),
            400,
        ),
        (
            json!({"snapshot":snapshot,"question":"q","selector":"oracle"}),
            422,
        ),
        (
            json!({"snapshot":snapshot,"question":"q","selector":"focused","execute":"code"}),
            422,
        ),
        (
            json!({"snapshot":snapshot,"question":"q".repeat(20000),"selector":"focused"}),
            413,
        ),
    ] {
        let response =
            diagnostic_request(router.clone(), json!(1), "POST", "/api/search", Some(body))
                .await
                .unwrap();
        assert_eq!(response.status, expected, "{}", response.body);
    }
    let response = diagnostic_request(router.clone(), json!(1), "POST", "/api/search", Some(query))
        .await
        .unwrap();
    assert_eq!(response.status, 200, "{}", response.body);
    let results: Value = serde_json::from_str(&response.body).unwrap();
    let hit = &results["results"][0];
    let pin = json!({"revision":0,"snapshot":snapshot,"file":hit["file"],"version":hit["version"],"line":hit["line"],"title":hit["title"]});
    let response = diagnostic_request(
        router.clone(),
        json!(1),
        "POST",
        "/api/search/evidence",
        Some(pin.clone()),
    )
    .await
    .unwrap();
    assert_eq!(response.status, 200, "{}", response.body);
    let evidence = app.view().board.evidence;
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].source.path, hit["path"].as_str().unwrap());
    assert_eq!(
        evidence[0].source.version.as_str(),
        hit["version"].as_str().unwrap()
    );
    let response = diagnostic_request(router, json!(1), "POST", "/api/search/evidence", Some(pin))
        .await
        .unwrap();
    assert_eq!(response.status, 409);
    assert_eq!(app.view().board.evidence.len(), 1);
}

#[test]
fn the_exact_question_byte_allowance_is_accepted_but_larger_input_is_refused() {
    let (_root, _cache, host) = setup();
    let trial = QuestionSearch::new(host.clone());
    let mut request = SearchRequest {
        snapshot: host.application().snapshot().unwrap().id().as_str().into(),
        question: "é".repeat(1024),
        selector: Selector::Focused,
    };
    assert!(trial.search(&request).is_ok());
    request.question.push('!');
    assert!(trial.search(&request).is_err());
}

#[test]
fn per_file_and_total_text_allowances_are_inclusive_and_omissions_are_visible() {
    let root = tempfile::tempdir().unwrap();
    let full = format!("//{}", "x".repeat(2 * 1024 * 1024 - 2));
    for n in 0..16 {
        std::fs::write(root.path().join(format!("a{n:02}.rs")), &full).unwrap();
    }
    std::fs::write(root.path().join("00-oversize.rs"), format!("{full}! ")).unwrap();
    std::fs::write(root.path().join("z.rs"), "fn omitted() {}\n").unwrap();
    let (_root, _cache, host) = start(root);
    let results = QuestionSearch::new(host.clone())
        .search(&SearchRequest {
            snapshot: host.application().snapshot().unwrap().id().as_str().into(),
            question: "omitted".into(),
            selector: Selector::Focused,
        })
        .unwrap();
    assert_eq!(results.available_files, 18);
    assert_eq!(results.searched_files, 16);
    assert_eq!(results.omitted_files, 2);
    assert!(results.results.is_empty());
}

#[test]
fn the_document_count_allowance_is_bounded_even_for_empty_files() {
    let root = tempfile::tempdir().unwrap();
    for n in 0..4097 {
        std::fs::write(root.path().join(format!("a{n:04}.rs")), "").unwrap();
    }
    let (_root, _cache, host) = start(root);
    let results = QuestionSearch::new(host.clone())
        .search(&SearchRequest {
            snapshot: host.application().snapshot().unwrap().id().as_str().into(),
            question: "missing_topic".into(),
            selector: Selector::Focused,
        })
        .unwrap();
    assert_eq!(results.available_files, 4097);
    assert_eq!(results.searched_files, 4096);
    assert_eq!(results.omitted_files, 1);
}
