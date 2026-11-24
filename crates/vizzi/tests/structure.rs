//! Native structural operations preserve current captured source bindings.
use serde_json::{Value, json};
use std::sync::Arc;
use vizzi::{
    cache::Cache,
    definitions::{Answer, Location, Position, Provider},
    engine::Engine,
    host::{Host, HostOptions},
    source::Directory,
    structural::{DefinitionRequest, Request, StructureHost},
};
use vizzi_core::{
    model::Snapshot,
    structure::{CallLookup, Symbol},
};
#[test]
fn native_operations_bind_symbols_to_the_current_capture_and_refuse_old_requests() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("pipe")).unwrap();
    std::fs::write(
        root.path().join("pipe/main.rs"),
        "fn begin() { relay(); } fn relay() {}\n",
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
                reconcile_interval: std::time::Duration::from_secs(3600),
                idle_maintenance: false,
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
    let request:Request=serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"pipe","search":"begin","offset":0}})).unwrap();
    let result = app.query(&request).unwrap();
    assert_eq!(result["matching"], 1);
    let symbol = &result["items"][0]["symbol"];
    let native = host.application().snapshot().unwrap();
    let doc = native
        .file(symbol["source"]["file"].as_str().unwrap())
        .unwrap();
    assert_eq!(symbol["source"]["version"], doc.version().as_str());
    let calls:Request=serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"calls","symbol":symbol["id"],"area":"pipe","offset":0}})).unwrap();
    assert_eq!(app.query(&calls).unwrap()["items"][0]["spelling"], "relay");
    let actions: Request = serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"actions","symbol":symbol["id"],"line_start":1,"line_count":1,"offset":0}})).unwrap();
    let annotations = app.query(&actions).unwrap();
    assert_eq!(annotations["snapshot"], snapshot);
    assert_eq!(
        annotations["items"][0]["id"],
        app.query(&calls).unwrap()["items"][0]["id"]
    );
    assert_eq!(
        annotations["items"][0]["position"],
        json!({"line":1,"start":13,"end":18})
    );
    assert!(serde_json::from_value::<Request>(json!({"snapshot":snapshot,"query":{"kind":"actions","symbol":symbol["id"],"line_start":1,"line_count":1,"offset":0,"text":"relay"}})).is_err());
    std::fs::write(root.path().join("pipe/main.rs"), "fn changed() {}\n").unwrap();
    host.request(vizzi::source::CaptureScope::Reconcile);
    let start = std::time::Instant::now();
    while host.application().snapshot().unwrap().id().as_str() == snapshot {
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(app.query(&calls).is_err());
    assert!(app.query(&actions).is_err());
    let current_snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let fresh:Request=serde_json::from_value(json!({"snapshot":current_snapshot,"query":{"kind":"inventory","area":"pipe","search":"changed","offset":0}})).unwrap();
    let current = app.query(&fresh).unwrap();
    assert_eq!(current["snapshot"], current_snapshot);
    assert_eq!(current["matching"], 1);
    assert_eq!(current["items"][0]["symbol"]["name"], "changed");
    assert!(serde_json::from_value::<Request>(json!({"snapshot":snapshot,"query":{"kind":"areas","search":"","offset":0,"execute":"code"}})).is_err());
}

#[tokio::test]
async fn structural_http_caps_schema_and_source_conflicts_are_bounded() {
    use vizzi::{
        companion::Companion,
        editor::VsCode,
        review_application::ReviewApplication,
        review_source::CaptureLimits,
        server::{diagnostic, diagnostic_request, router_with_companion},
    };
    use vizzi_core::{analysis::Languages, python::PythonAdapter, rust::RustAdapter};
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("main.rs"),
        "fn start() { run(); } fn run() {}\n",
    )
    .unwrap();
    let host = Arc::new(
        Host::start_with_options(
            Engine::new(
                Directory::with_languages(
                    root.path(),
                    Arc::new(
                        Languages::new(vec![Arc::new(RustAdapter), Arc::new(PythonAdapter)])
                            .unwrap(),
                    ),
                )
                .unwrap(),
                Cache::new(cache.path().join("analysis")),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                reconcile_interval: std::time::Duration::from_secs(3600),
                idle_maintenance: false,
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
    let router = router_with_companion(host, reviews, app);
    let caps = diagnostic(router.clone(), json!(1), "/api/structure")
        .await
        .unwrap();
    assert_eq!(caps.status, 200);
    assert!(caps.body.contains("candidate_only"));
    let capabilities: Value = serde_json::from_str(&caps.body).unwrap();
    assert_eq!(capabilities["call_languages"], json!(["rust", "python"]));
    for (body, status) in [
        (
            json!({"snapshot":snapshot,"query":{"kind":"areas","search":"","offset":0}}),
            200,
        ),
        (
            json!({"snapshot":"expired","query":{"kind":"areas","search":"","offset":0}}),
            409,
        ),
        (
            json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"../escape","search":"","offset":0}}),
            400,
        ),
        (
            json!({"snapshot":snapshot,"query":{"kind":"areas","search":"","offset":0,"executable":"code"}}),
            422,
        ),
        (
            json!({"snapshot":snapshot,"query":{"kind":"areas","search":"x".repeat(20000),"offset":0}}),
            413,
        ),
    ] {
        let response = diagnostic_request(
            router.clone(),
            json!(1),
            "POST",
            "/api/structure",
            Some(body),
        )
        .await
        .unwrap();
        assert_eq!(response.status, status, "{}", response.body);
    }
}

#[test]
fn function_search_requests_rank_and_filter_in_the_native_adapter() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let source = "struct process; struct LufsMeter; impl LufsMeter { fn process_sample() {} } fn process_sample() {} fn process() {}";
    std::fs::write(root.path().join("lib.rs"), source).unwrap();
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
                reconcile_interval: std::time::Duration::from_secs(3600),
                idle_maintenance: false,
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
    let app = StructureHost::new(host);
    let req:Request=serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"","search":"process","offset":0,"functions_only":true}})).unwrap();
    let results = app.query(&req).unwrap();
    assert_eq!(results["matching"], 3);
    assert_eq!(results["items"][0]["symbol"]["name"], "process");
    assert!(
        results["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["symbol"]["kind"] == "function")
    );
    let req:Request=serde_json::from_value(json!({"snapshot":snapshot,"query":{"kind":"inventory","area":"","search":"LufsMeter::process_sample","offset":0,"functions_only":true}})).unwrap();
    let results = app.query(&req).unwrap();
    assert_eq!(results["matching"], 1);
    assert_eq!(results["items"][0]["symbol"]["name"], "process_sample");
}

#[test]
fn exact_file_function_search_survives_global_declaration_exhaustion() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let padding = (0..65536)
        .map(|n| format!("struct Item{n};\n"))
        .collect::<String>();
    std::fs::write(root.path().join("a.rs"), padding).unwrap();
    std::fs::write(
        root.path().join("z.rs"),
        "struct Meter; impl Meter { fn start(&self) { self.finish(); } fn finish(&self) {} }",
    )
    .unwrap();
    std::fs::write(root.path().join("zz.rs"), "fn alternate() {}").unwrap();
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
                reconcile_interval: std::time::Duration::from_secs(3600),
                idle_maintenance: false,
            },
        )
        .unwrap(),
    );
    let snapshot = host.application().snapshot().unwrap();
    let doc = snapshot.file_by_path("z.rs").unwrap();
    let app = StructureHost::new(host);
    let query = |query| {
        app.query(
            &serde_json::from_value(json!({"snapshot":snapshot.id().as_str(),"query":query}))
                .unwrap(),
        )
        .unwrap()
    };
    let search = |area| {
        query(
            json!({"kind":"inventory","area":area,"search":"Meter::start","offset":0,"functions_only":true}),
        )
    };
    let global = search("");
    assert_eq!(global["matching"], 0);
    assert_eq!(global["coverage"]["available_files"], 3);
    assert_eq!(global["coverage"]["omitted_files"], 0);
    assert_eq!(global["coverage"]["omitted_symbols"], 5);
    let scoped = search("z.rs");
    assert_eq!(scoped["matching"], 1);
    assert_eq!(scoped["coverage"], global["coverage"]);
    assert_eq!(scoped["scope_coverage"]["available_files"], 1);
    assert_eq!(scoped["scope_coverage"]["omitted_symbols"], 0);
    let symbol = &scoped["items"][0]["symbol"];
    assert_eq!(symbol["source"]["file"], doc.id().as_str());
    assert_eq!(symbol["source"]["version"], doc.version().as_str());
    let alternate = query(json!({
        "kind":"inventory","area":"zz.rs","search":"alternate","offset":0,"functions_only":true
    }));
    assert_eq!(alternate["items"][0]["symbol"]["source"]["path"], "zz.rs");
    let calls = query(json!({"kind":"calls","area":"z.rs","symbol":symbol["id"],"offset":0}));
    assert_eq!(calls["items"][0]["name"], "finish");
    assert_eq!(calls["items"][0]["binding"], "rust_self_receiver");
    let refreshed = app
        .refresh(
            &serde_json::from_value(json!({
                "snapshot": snapshot.id().as_str(),
                "views": [{
                    "id": "v1", "tracking": symbol["tracking"], "symbol": symbol["id"],
                    "version": symbol["source"]["version"], "file": symbol["source"]["file"],
                    "context": symbol["source"]["context"], "area": "z.rs", "section_offset": 0,
                    "anchor": null
                }],
                "links": []
            }))
            .unwrap(),
        )
        .unwrap();
    assert_eq!(refreshed["views"][0]["state"], "current");
    assert_eq!(refreshed["views"][0]["symbol"], *symbol);
    assert_eq!(
        refreshed["views"][0]["actions"][0]["id"],
        calls["items"][0]["id"]
    );
    assert_eq!(search("")["matching"], 0);
}

struct ProjectedFiles {
    _root: tempfile::TempDir,
    _cache: tempfile::TempDir,
    host: Arc<Host>,
    snapshot: String,
}

impl ProjectedFiles {
    fn new(files: &[(&str, &str)], reserve: usize) -> Self {
        let root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let padding = (0..65536 - reserve)
            .map(|n| format!("struct Item{n};\n"))
            .collect::<String>();
        std::fs::write(root.path().join("a.rs"), padding).unwrap();
        for (path, text) in files {
            std::fs::write(root.path().join(path), text).unwrap();
        }
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
        let snapshot = host
            .application()
            .snapshot()
            .unwrap()
            .id()
            .as_str()
            .to_owned();
        Self {
            _root: root,
            _cache: cache,
            host,
            snapshot,
        }
    }

    fn query(&self, app: &StructureHost, path: Option<&str>, query: Value) -> Value {
        app.query(
            &serde_json::from_value(json!({
                "snapshot":self.snapshot, "path":path, "query":query
            }))
            .unwrap(),
        )
        .unwrap()
    }

    fn function(&self, app: &StructureHost, area: &str, search: &str) -> Value {
        self.query(
            app,
            None,
            json!({
                "kind":"inventory", "area":area, "search":search, "offset":0, "functions_only":true
            }),
        )["items"][0]["symbol"]
            .clone()
    }

    fn callers(&self, app: &StructureHost, target: &Value) -> Value {
        let request = serde_json::from_value(json!({
            "snapshot":self.snapshot, "symbol":target["id"], "file":target["source"]["file"],
            "context":target["source"]["context"], "kind":"callers", "offset":0
        }))
        .unwrap();
        serde_json::to_value(app.references(&request).unwrap()).unwrap()
    }

    fn calls(&self, app: &StructureHost, path: &str, owner: &Value) -> Value {
        self.query(
            app,
            Some(path),
            json!({
                "kind":"calls", "symbol":owner["id"], "area":path, "offset":0
            }),
        )
    }
}

const PARTIAL: &str = "struct A; impl A { fn start(&self) { self.run(); } fn run(&self) {} }\nstruct B; impl B { fn run(&self) {} }\n";

#[test]
fn partially_projected_callers_keep_the_file_binding_through_live_navigation() {
    let fixture = ProjectedFiles::new(&[("lib.rs", PARTIAL)], 3);
    let app = StructureHost::new(fixture.host.clone());
    let start = fixture.function(&app, "lib.rs", "A::start");
    let run = fixture.function(&app, "lib.rs", "A::run");
    assert_eq!(fixture.function(&app, "", "A::start"), start);
    assert!(fixture.function(&app, "", "A::run").is_null());
    let call = &fixture.calls(&app, "lib.rs", &start)["items"][0];
    assert_eq!(call["binding"], "rust_self_receiver");
    assert_eq!(call["candidates"][0]["id"], run["id"]);
    let actions = fixture.query(
        &app,
        Some("lib.rs"),
        json!({
            "kind":"actions", "symbol":start["id"], "line_start":1, "line_count":1, "offset":0
        }),
    );
    assert_eq!(actions["items"][0]["binding"], call["binding"]);
    let file = &start["source"]["file"];
    let source_actions = fixture.query(
        &app,
        None,
        json!({
            "kind":"source_actions", "file":file, "line_start":1, "line_count":1, "offset":0
        }),
    );
    assert!(
        source_actions["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| { item["kind"] == "function" && item["id"] == run["id"] })
    );
    let source_function = fixture.query(
        &app,
        None,
        json!({
            "kind":"function", "file":file, "symbol":run["id"]
        }),
    );
    assert_eq!(source_function["symbol"], run);
    let browser_call = app.editor_call(&serde_json::from_value(json!({
        "snapshot":fixture.snapshot, "query":{"kind":"invocation", "call":call["id"], "file":file}
    })).unwrap()).unwrap();
    assert_eq!(browser_call.invocation.id, call["id"]);
    assert_eq!(browser_call.caller.id, start["id"]);
    let definition = app
        .definition(&DefinitionRequest {
            snapshot: fixture.snapshot.clone(),
            path: None,
            call: call["id"].as_str().unwrap().into(),
        })
        .unwrap();
    assert_eq!(definition.invocation.candidates[0].id, run["id"]);
    assert!(definition.invocation.binding.is_some());
    let view = |id, symbol: &Value| {
        json!({
            "id":id, "symbol":symbol["id"], "tracking":symbol["tracking"], "version":symbol["source"]["version"],
            "file":symbol["source"]["file"], "context":symbol["source"]["context"], "area":"lib.rs",
            "section_offset":0, "anchor":null
        })
    };
    let refresh = app
        .refresh(
            &serde_json::from_value(json!({
                "snapshot":fixture.snapshot, "views":[view("v1", &start),view("v2", &run)],
                "links":[{"id":"l1","from":"v1","to":"v2","spelling":"self.run","kind":"self"}]
            }))
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        refresh["views"][0]["calls"]["items"][0]["binding"],
        call["binding"]
    );
    assert_eq!(
        refresh["views"][0]["actions"][0]["binding"],
        call["binding"]
    );
    assert_eq!(refresh["links"][0]["check"]["state"], "current");
}

#[test]
fn a_rejected_file_local_caller_is_not_reintroduced_from_the_repository() {
    let fixture = ProjectedFiles::new(
        &[("lib.rs", PARTIAL), ("other.rs", "fn unbound() { run(); }")],
        3,
    );
    let app = StructureHost::new(fixture.host.clone());
    let run_a = fixture.function(&app, "lib.rs", "A::run");
    let run_b = fixture.function(&app, "lib.rs", "B::run");
    let to_b = fixture.callers(&app, &run_b);
    assert_eq!(to_b["precision"], "name_candidates");
    assert_eq!(to_b["matching"], 1);
    assert_eq!(to_b["items"][0]["source"]["path"], "other.rs");
    let to_a = fixture.callers(&app, &run_a);
    assert_eq!(to_a["matching"], 2);
    assert_eq!(to_a["items"][0]["source"]["path"], "lib.rs");
    assert_eq!(to_a["items"][0]["site"]["binding"], "rust_self_receiver");
}

struct ProviderLocations {
    definition: Location,
    references: Vec<Location>,
}

impl Provider for ProviderLocations {
    fn resolve(&self, _: Arc<Snapshot>, _: &CallLookup) -> Result<Answer, String> {
        Ok(Answer {
            locations: vec![self.definition.clone()],
            outside: 0,
            context: "a".repeat(64),
        })
    }

    fn references(&self, _: Arc<Snapshot>, _: &Symbol, _: usize) -> Result<Answer, String> {
        Ok(Answer {
            locations: self.references.clone(),
            outside: 0,
            context: "a".repeat(64),
        })
    }
}

fn location(path: &str, character: u32) -> Location {
    Location {
        capture: None,
        path: path.into(),
        position: Position { line: 0, character },
    }
}

const CROSS_FILE: &[(&str, &str)] = &[
    ("caller.rs", "fn caller() { actual(); }"),
    ("target.rs", "pub fn actual() {}"),
];

#[test]
fn semantic_definition_recovers_destination_in_a_different_omitted_file() {
    let fixture = ProjectedFiles::new(CROSS_FILE, 0);
    let app = StructureHost::with_definitions(
        fixture.host.clone(),
        Arc::new(ProviderLocations {
            definition: location("target.rs", 7),
            references: vec![],
        }),
    );
    let caller = fixture.function(&app, "caller.rs", "caller");
    let target = fixture.function(&app, "target.rs", "actual");
    assert!(fixture.function(&app, "", "actual").is_null());
    let calls = fixture.calls(&app, "caller.rs", &caller);
    let result = serde_json::to_value(
        app.definition(&DefinitionRequest {
            snapshot: fixture.snapshot.clone(),
            path: Some("caller.rs".into()),
            call: calls["items"][0]["id"].as_str().unwrap().into(),
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(result["state"], "unique");
    assert_eq!(result["invocation"]["candidates"][0], target);
}

#[test]
fn semantic_callers_recover_call_witnesses_in_other_omitted_files() {
    let fixture = ProjectedFiles::new(CROSS_FILE, 0);
    let app = StructureHost::with_definitions(
        fixture.host.clone(),
        Arc::new(ProviderLocations {
            definition: location("target.rs", 7),
            references: vec![location("caller.rs", 14)],
        }),
    );
    let caller = fixture.function(&app, "caller.rs", "caller");
    let target = fixture.function(&app, "target.rs", "actual");
    assert!(fixture.function(&app, "", "caller").is_null());
    let callers = fixture.callers(&app, &target);
    assert_eq!(callers["precision"], "semantic");
    assert_eq!(callers["outside"], 0);
    assert_eq!(callers["omitted"], 0);
    assert_eq!(callers["matching"], 1);
    assert_eq!(callers["items"][0]["symbol"], caller);
    assert_eq!(
        callers["items"][0]["site"]["binding"],
        "language_server_definition"
    );
}

#[test]
fn semantic_callers_report_locations_beyond_the_file_projection_allowance() {
    let names: Vec<_> = (0..18).map(|n| format!("caller{n}.rs")).collect();
    let mut files = vec![("target.rs", "pub fn actual() {}")];
    files.extend(
        names
            .iter()
            .map(|name| (name.as_str(), "fn caller() { actual(); }")),
    );
    let fixture = ProjectedFiles::new(&files, 0);
    let app = StructureHost::with_definitions(
        fixture.host.clone(),
        Arc::new(ProviderLocations {
            definition: location("target.rs", 7),
            references: names.iter().map(|name| location(name, 14)).collect(),
        }),
    );
    let target = fixture.function(&app, "target.rs", "actual");
    let result = fixture.callers(&app, &target);
    assert_eq!(result["matching"], 16);
    assert_eq!(result["omitted"], 2);
    assert_eq!(result["outside"], 0);
}
