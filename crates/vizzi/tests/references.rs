//! Semantic reference results and explicitly weaker caller candidates.
use serde_json::json;
use std::sync::Arc;
use vizzi::{
    cache::Cache,
    definitions::{Answer, Location, Position, Provider},
    engine::Engine,
    host::{Host, HostOptions},
    source::Directory,
    structural::references::ReferenceRequest,
    structural::{Request, StructureHost},
};
use vizzi_core::{
    model::Snapshot,
    structure::{CallLookup, Symbol},
};
struct Semantic;
impl Provider for Semantic {
    fn resolve(&self, _: Arc<Snapshot>, _: &CallLookup) -> Result<Answer, String> {
        Err("unused".into())
    }
    fn references(&self, _: Arc<Snapshot>, _: &Symbol, _: usize) -> Result<Answer, String> {
        Ok(Answer {
            locations: vec![
                Location {
                    capture: None,
                    path: "lib.rs".into(),
                    position: Position {
                        line: 1,
                        character: 13,
                    },
                },
                Location {
                    capture: None,
                    path: "lib.rs".into(),
                    position: Position {
                        line: 1,
                        character: 33,
                    },
                },
            ],
            outside: 0,
            context: "a".repeat(64),
        })
    }
}
struct AllCalls;
impl Provider for AllCalls {
    fn resolve(&self, _: Arc<Snapshot>, _: &CallLookup) -> Result<Answer, String> {
        Err("unused".into())
    }
    fn references(&self, snapshot: Arc<Snapshot>, _: &Symbol, _: usize) -> Result<Answer, String> {
        let doc = snapshot.file_by_path("lib.rs").unwrap();
        let text = doc.resident_source().unwrap();
        let locations = text
            .match_indices("target")
            .skip(1)
            .map(|(byte, _)| Location {
                capture: None,
                path: "lib.rs".into(),
                position: vizzi::definitions::position(text, byte).unwrap(),
            })
            .collect();
        Ok(Answer {
            locations,
            outside: 0,
            context: "b".repeat(64),
        })
    }
}
struct RawAnswer(Answer);
impl Provider for RawAnswer {
    fn resolve(&self, _: Arc<Snapshot>, _: &CallLookup) -> Result<Answer, String> {
        Err("unused".into())
    }
    fn references(&self, _: Arc<Snapshot>, _: &Symbol, _: usize) -> Result<Answer, String> {
        Ok(self.0.clone())
    }
}
#[test]
fn bounded_reference_queries_report_overflow_for_both_capabilities() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let text = format!(
        "fn target(){{}}\n{}",
        (0..263)
            .map(|n| format!("fn caller{n}(){{target();}}\n"))
            .collect::<String>()
    );
    std::fs::write(root.path().join("lib.rs"), text).unwrap();
    let host = Arc::new(
        Host::start_with_options(
            Engine::new(
                Directory::open(root.path()).unwrap(),
                Cache::new(cache.path().into()),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                idle_maintenance: false,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    for semantic in [false, true] {
        let app = if semantic {
            StructureHost::with_definitions(host.clone(), Arc::new(AllCalls))
        } else {
            StructureHost::new(host.clone())
        };
        let snapshot = host.application().snapshot().unwrap();
        let inventory = app.query(&serde_json::from_value(json!({
            "snapshot":snapshot.id().as_str(),
            "query":{"kind":"inventory","area":"lib.rs","search":"target","offset":0,"functions_only":true}
        })).unwrap()).unwrap();
        let request = json!({"snapshot":snapshot.id().as_str(),"symbol":inventory["items"][0]["symbol"]["id"],"kind":"callers"});
        let first = app
            .references(&serde_json::from_value(request.clone()).unwrap())
            .unwrap();
        assert_eq!(first.matching, 256);
        assert_eq!(first.omitted, 7);
        let mut continuation = request;
        continuation["offset"] = 248.into();
        continuation["query"] = first.query.into();
        let last = app
            .references(&serde_json::from_value(continuation).unwrap())
            .unwrap();
        assert_eq!(last.items.len(), 8);
        assert_eq!(last.next, None);
    }
}

struct ChangedDuringQuery {
    host: Arc<Host>,
    root: std::path::PathBuf,
}
struct DependencyReferences {
    catalog: vizzi::dependency_sources::Catalog,
    answer: Answer,
}
impl Provider for DependencyReferences {
    fn dependencies(&self) -> Option<&vizzi::dependency_sources::Catalog> {
        Some(&self.catalog)
    }
    fn resolve(&self, _: Arc<Snapshot>, _: &CallLookup) -> Result<Answer, String> {
        Err("unused".into())
    }
    fn references(&self, _: Arc<Snapshot>, _: &Symbol, _: usize) -> Result<Answer, String> {
        Ok(self.answer.clone())
    }
}
#[test]
fn reference_targets_and_callers_can_keep_independent_dependency_provenance() {
    let root = tempfile::tempdir().unwrap();
    let dependency = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("lib.rs"), "fn local(){}\n").unwrap();
    std::fs::write(
        dependency.path().join("lib.rs"),
        "pub fn target(){}\nfn caller(){target();}\n",
    )
    .unwrap();
    let catalog = vizzi::dependency_sources::Catalog::new(cache.path().join("dependencies.json"));
    let captured = catalog
        .capture(vizzi::dependency_sources::Input {
            root: dependency.path().canonicalize().unwrap(),
            path: "lib.rs".into(),
            language: "rust".into(),
            package: "helper".into(),
            version: Some("1.0.0".into()),
            configuration: "a".repeat(64),
        })
        .unwrap();
    let target = captured
        .index
        .functions("", "target", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let capture = captured.origin.capture.clone();
    let host = Arc::new(
        Host::start_with_options(
            Engine::new(
                Directory::open(root.path()).unwrap(),
                Cache::new(cache.path().join("analysis")),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                idle_maintenance: false,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let app = StructureHost::with_definitions(
        host.clone(),
        Arc::new(DependencyReferences {
            catalog,
            answer: Answer {
                locations: vec![Location {
                    capture: Some(capture.clone()),
                    path: "lib.rs".into(),
                    position: Position {
                        line: 1,
                        character: 12,
                    },
                }],
                outside: 0,
                context: "b".repeat(64),
            },
        }),
    );
    let request = json!({"snapshot":host.application().snapshot().unwrap().id().as_str(),"symbol":target.id,"capture":capture,"kind":"callers"});
    let page = app
        .references(&serde_json::from_value(request.clone()).unwrap())
        .unwrap();
    assert_eq!(page.matching, 1);
    assert_eq!(page.target.origin.unwrap().capture, capture);
    let row = &page.items[0];
    assert_eq!(row.origin.as_ref().unwrap().capture, capture);
    assert_eq!(
        row.symbol
            .as_ref()
            .unwrap()
            .origin
            .as_ref()
            .unwrap()
            .capture,
        capture
    );
    assert_eq!(
        row.site.as_ref().unwrap().candidates[0]
            .origin
            .as_ref()
            .unwrap()
            .capture,
        capture
    );
    let mut expired = request;
    expired["capture"] = "f".repeat(64).into();
    assert!(matches!(
        app.references(&serde_json::from_value(expired).unwrap()),
        Err(vizzi::companion::CompanionError::Conflict(_))
    ));
}
impl Provider for ChangedDuringQuery {
    fn resolve(&self, _: Arc<Snapshot>, _: &CallLookup) -> Result<Answer, String> {
        Err("unused".into())
    }
    fn references(&self, snapshot: Arc<Snapshot>, _: &Symbol, _: usize) -> Result<Answer, String> {
        std::fs::write(self.root.join("lib.rs"), "fn target(){}\n// changed\n").unwrap();
        self.host.request(vizzi::source::CaptureScope::Reconcile);
        let started = std::time::Instant::now();
        while self.host.application().snapshot().unwrap().id() == snapshot.id() {
            assert!(started.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(Answer {
            locations: vec![],
            outside: 0,
            context: "a".repeat(64),
        })
    }
}
#[test]
fn source_changes_during_analysis_reject_the_late_reference_answer() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("lib.rs"), "fn target(){}\n").unwrap();
    let host = Arc::new(
        Host::start_with_options(
            Engine::new(
                Directory::open(root.path()).unwrap(),
                Cache::new(cache.path().into()),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                idle_maintenance: false,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let app = StructureHost::with_definitions(
        host.clone(),
        Arc::new(ChangedDuringQuery {
            host: host.clone(),
            root: root.path().into(),
        }),
    );
    let snapshot = host.application().snapshot().unwrap();
    let inventory = app.query(&serde_json::from_value(json!({
        "snapshot":snapshot.id().as_str(),
        "query":{"kind":"inventory","area":"lib.rs","search":"target","offset":0,"functions_only":true}
    })).unwrap()).unwrap();
    let request = serde_json::from_value(json!({"snapshot":snapshot.id().as_str(),"symbol":inventory["items"][0]["symbol"]["id"],"kind":"references"})).unwrap();
    assert!(matches!(
        app.references(&request),
        Err(vizzi::companion::CompanionError::Conflict(_))
    ));
}
#[test]
fn reference_rows_keep_their_line_context_and_report_unavailable_locations_without_false_uniqueness()
 {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("lib.rs"),
        "fn target(){}\nfn caller(){ target(); }\n",
    )
    .unwrap();
    let host = Arc::new(
        Host::start_with_options(
            Engine::new(
                Directory::open(root.path()).unwrap(),
                Cache::new(cache.path().into()),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                idle_maintenance: false,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let plain = StructureHost::new(host.clone());
    let data=plain.query(&serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"lib.rs","search":"target","offset":0,"functions_only":true}})).unwrap()).unwrap();
    let request:ReferenceRequest=serde_json::from_value(json!({"snapshot":snapshot,"symbol":data["items"][0]["symbol"]["id"],"kind":"callers","offset":0})).unwrap();
    let location = Location {
        capture: None,
        path: "lib.rs".into(),
        position: Position {
            line: 1,
            character: 13,
        },
    };
    let app = StructureHost::with_definitions(
        host.clone(),
        Arc::new(RawAnswer(Answer {
            locations: vec![
                location.clone(),
                location,
                Location {
                    capture: None,
                    path: "missing.rs".into(),
                    position: Position {
                        line: 0,
                        character: 0,
                    },
                },
            ],
            outside: 1,
            context: "a".repeat(64),
        })),
    );
    let page = app.references(&request).unwrap();
    assert_eq!(page.matching, 1);
    assert_eq!(page.outside, 2);
    assert_eq!(page.items[0].source.line, 2);
    assert!(page.items[0].snippet.contains("fn caller"));
    assert!(page.items[0].snippet.contains("target()"));
    for answer in [
        Answer {
            locations: vec![],
            outside: 0,
            context: "invalid".into(),
        },
        Answer {
            locations: vec![],
            outside: 4097,
            context: "a".repeat(64),
        },
    ] {
        let app = StructureHost::with_definitions(host.clone(), Arc::new(RawAnswer(answer)));
        assert!(app.references(&request).is_err());
    }
}
#[test]
fn candidate_callers_exclude_calls_bound_to_a_different_same_named_method() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("lib.rs"),"struct A; impl A {fn run(&self){} fn caller(&self){self.run();}} struct B; impl B {fn run(&self){}}\n").unwrap();
    let host = Arc::new(
        Host::start_with_options(
            Engine::new(
                Directory::open(root.path()).unwrap(),
                Cache::new(cache.path().into()),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                idle_maintenance: false,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let app = StructureHost::new(host.clone());
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let data=app.query(&serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"lib.rs","search":"B::run","offset":0,"functions_only":true}})).unwrap()).unwrap();
    let request=serde_json::from_value(json!({"snapshot":snapshot,"symbol":data["items"][0]["symbol"]["id"],"kind":"callers","offset":0})).unwrap();
    assert_eq!(app.references(&request).unwrap().matching, 0);
}
#[test]
fn reference_continuations_cover_each_page_and_reject_malformed_identity_tokens() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let text = format!(
        "fn target(){{}}\n{}",
        (0..17)
            .map(|n| format!("fn caller{n}(){{target();}}\n"))
            .collect::<String>()
    );
    std::fs::write(root.path().join("lib.rs"), text).unwrap();
    let host = Arc::new(
        Host::start_with_options(
            Engine::new(
                Directory::open(root.path()).unwrap(),
                Cache::new(cache.path().into()),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                idle_maintenance: false,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let app = StructureHost::with_definitions(host.clone(), Arc::new(AllCalls));
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let data=app.query(&serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"lib.rs","search":"target","offset":0,"functions_only":true}})).unwrap()).unwrap();
    let symbol = data["items"][0]["symbol"]["id"].clone();
    let request = json!({"snapshot":snapshot,"symbol":symbol,"kind":"callers","offset":0});
    let first = app
        .references(&serde_json::from_value(request.clone()).unwrap())
        .unwrap();
    assert_eq!(first.matching, 17);
    assert_eq!(first.items.len(), 8);
    assert_eq!(first.next, Some(8));
    let mut continuation = request.clone();
    continuation["query"] = first.query.into();
    let mut seen = first
        .items
        .iter()
        .map(|r| r.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for (offset, count, next) in [(8, 8, Some(16)), (16, 1, None)] {
        continuation["offset"] = offset.into();
        let page = app
            .references(&serde_json::from_value(continuation.clone()).unwrap())
            .unwrap();
        assert_eq!(page.items.len(), count);
        assert_eq!(page.next, next);
        seen.extend(page.items.into_iter().map(|r| r.id));
    }
    assert_eq!(seen.len(), 17);
    continuation["offset"] = 256.into();
    let empty = app
        .references(&serde_json::from_value(continuation).unwrap())
        .unwrap();
    assert!(empty.items.is_empty());
    assert_eq!(empty.next, None);
    for field in [
        "snapshot", "symbol", "query", "capture", "tracking", "file", "context",
    ] {
        for token in ["a".into(), "z".repeat(64), "A".repeat(64)] {
            let mut bad = request.clone();
            if field == "file" {
                bad["context"] = "a".repeat(64).into();
            }
            if field == "context" {
                bad["file"] = "a".repeat(64).into();
            }
            bad[field] = token.into();
            assert!(
                matches!(
                    app.references(&serde_json::from_value(bad).unwrap()),
                    Err(vizzi::companion::CompanionError::Invalid(_))
                ),
                "{field}"
            );
        }
    }
}
#[test]
fn references_distinguish_semantic_callers_from_value_uses_and_name_candidates() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("lib.rs"),
        "fn target(){}\nfn caller(){ target(); let saved=target; }\n",
    )
    .unwrap();
    let host = Arc::new(
        Host::start_with_options(
            Engine::new(
                Directory::open(root.path()).unwrap(),
                Cache::new(cache.path().into()),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                idle_maintenance: false,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let app = StructureHost::with_definitions(host.clone(), Arc::new(Semantic));
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let symbols=app.query(&serde_json::from_value::<Request>(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"lib.rs","search":"target","offset":0,"functions_only":true}})).unwrap()).unwrap();
    let target = &symbols["items"][0]["symbol"];
    for (kind, count) in [("callers", 1), ("references", 2)] {
        let request: ReferenceRequest = serde_json::from_value(
            json!({"snapshot":snapshot,"symbol":target["id"],"kind":kind,"offset":0}),
        )
        .unwrap();
        let result = serde_json::to_value(app.references(&request).unwrap()).unwrap();
        assert_eq!(result["precision"], "semantic");
        assert_eq!(result["matching"], count);
        assert_eq!(result["items"][0]["symbol"]["name"], "caller");
        if kind == "callers" {
            assert_eq!(
                result["items"][0]["site"]["binding"],
                "language_server_definition"
            );
            assert_eq!(
                result["items"][0]["site"]["candidates"][0]["id"],
                target["id"]
            );
        }
    }
    let fallback = StructureHost::new(host);
    let request = serde_json::from_value(
        json!({"snapshot":snapshot,"symbol":target["id"],"kind":"callers","offset":0}),
    )
    .unwrap();
    let result = serde_json::to_value(fallback.references(&request).unwrap()).unwrap();
    assert_eq!(result["precision"], "name_candidates");
    assert_eq!(result["matching"], 1);
}
#[test]
fn reference_pages_are_bound_to_their_target_query_and_source_observation() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("lib.rs"),
        "fn target(){}\nfn caller(){ target(); let saved=target; }\n",
    )
    .unwrap();
    let host = Arc::new(
        Host::start_with_options(
            Engine::new(
                Directory::open(root.path()).unwrap(),
                Cache::new(cache.path().into()),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                idle_maintenance: false,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let app = StructureHost::with_definitions(host.clone(), Arc::new(Semantic));
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let data=app.query(&serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"lib.rs","search":"target","offset":0,"functions_only":true}})).unwrap()).unwrap();
    let symbol = data["items"][0]["symbol"]["id"].clone();
    let request = json!({"snapshot":snapshot,"symbol":symbol,"kind":"references","offset":0});
    let page = app
        .references(&serde_json::from_value(request.clone()).unwrap())
        .unwrap();
    let mut continuation = request.clone();
    continuation["query"] = page.query.clone().into();
    let repeat = app
        .references(&serde_json::from_value(continuation.clone()).unwrap())
        .unwrap();
    assert_eq!(repeat.items.len(), 2);
    let mut other = request.clone();
    other["kind"] = "callers".into();
    let callers = app
        .references(&serde_json::from_value(other).unwrap())
        .unwrap();
    assert_ne!(callers.query, page.query);
    let retained = app
        .references(&serde_json::from_value(continuation.clone()).unwrap())
        .unwrap();
    assert_eq!(retained.items.len(), 2);
    for (field, value) in [
        ("snapshot", json!("0".repeat(64))),
        ("symbol", json!("0".repeat(64))),
        ("offset", json!(257)),
        ("query", json!("0".repeat(64))),
        ("capture", json!("invalid")),
        ("context", json!("0".repeat(64))),
    ] {
        let mut invalid = continuation.clone();
        invalid[field] = value;
        assert!(
            app.references(&serde_json::from_value(invalid).unwrap())
                .is_err(),
            "{field}"
        );
    }
    let mut missing = request;
    missing["offset"] = 1.into();
    assert!(
        app.references(&serde_json::from_value(missing).unwrap())
            .is_err()
    );
}
#[test]
#[ignore = "requires installed rust-analyzer; local project only"]
fn rust_reference_provider_returns_real_calls_and_function_value_uses() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("Cargo.toml"),"[package]\nname='reference_fixture'\nversion='0.1.0'\nedition='2024'\n[lib]\npath='lib.rs'\n").unwrap();
    std::fs::write(
        root.path().join("lib.rs"),
        "pub fn target(){}\npub fn caller(){ target(); let _saved=target; }\n",
    )
    .unwrap();
    let host = Arc::new(
        Host::start_with_options(
            Engine::new(
                Directory::open(root.path()).unwrap(),
                Cache::new(cache.path().join("analysis")),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                idle_maintenance: false,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let provider = Arc::new(vizzi::rust_definitions::RustAnalyzer::new(
        root.path().into(),
        cache.path().join("semantic"),
        "rust-analyzer".into(),
    ));
    let app = StructureHost::with_definitions(host.clone(), provider);
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let data=app.query(&serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"lib.rs","search":"target","offset":0,"functions_only":true}})).unwrap()).unwrap();
    let symbol = data["items"][0]["symbol"]["id"].as_str().unwrap();
    for (kind, count) in [("callers", 1), ("references", 2)] {
        let request = serde_json::from_value(
            json!({"snapshot":snapshot,"symbol":symbol,"kind":kind,"offset":0}),
        )
        .unwrap();
        let result = app.references(&request).unwrap();
        assert_eq!(result.precision, "semantic", "{:?}", result.detail);
        assert_eq!(result.matching, count);
    }
}
