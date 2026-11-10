//! A coherent live refresh is native-bound and independent of editor/investigation state.
use serde_json::{Value, json};
use std::sync::Arc;
use vizzi::{
    cache::Cache,
    engine::Engine,
    host::{Host, HostOptions},
    source::{CaptureScope, Directory},
    structural::{Request, StructureHost, live},
};
fn fixture() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Arc<Host>,
    StructureHost,
) {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("lib.rs"),
        "fn start() {\n    end();\n}\nfn end() {}\n",
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
    (root, cache, host, app)
}
fn symbols(host: &Host, app: &StructureHost) -> Vec<Value> {
    let req:Request=serde_json::from_value(json!({"snapshot":host.application().snapshot().unwrap().id().as_str(),"query":{"kind":"inventory","area":"","search":"","functions_only":true,"offset":0}})).unwrap();
    app.query(&req).unwrap()["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["symbol"].clone())
        .collect()
}
fn request(snapshot: &str, symbols: &[Value]) -> Value {
    json!({"snapshot":snapshot,"views":symbols.iter().enumerate().map(|(i,s)|json!({"id":format!("v{}",i+1),"symbol":s["id"],"tracking":s["tracking"],"version":s["source"]["version"],"area":"","section_offset":0,"anchor":{"offset":1,"byte":4,"text":"    end();"}})).collect::<Vec<_>>(),"links":[]})
}
fn publish(host: &Host, old: &str) {
    host.request(CaptureScope::Reconcile);
    let at = std::time::Instant::now();
    while host.application().snapshot().unwrap().id().as_str() == old {
        assert!(at.elapsed() < std::time::Duration::from_secs(10));
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
#[test]
fn refresh_prepares_a_single_observation_and_maps_saved_reading_context() {
    let (root, _cache, host, app) = fixture();
    let old = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let syms = symbols(&host, &app);
    let mut req = request(&old, &syms);
    let unchanged: live::Request = serde_json::from_value(req.clone()).unwrap();
    let answer = app.refresh(&unchanged).unwrap();
    for view in answer["views"].as_array().unwrap() {
        let first = view["symbol"]["source"]["line"].as_u64().unwrap();
        let last = view["symbol"]["source"]["end_line"].as_u64().unwrap();
        assert_eq!(view["position"]["line"], (first + 1).min(last));
    }
    assert!(
        answer["views"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["window"].is_null())
    );
    std::fs::write(
        root.path().join("lib.rs"),
        "// moved\nfn start() {\n    changed();\n    end();\n}\nfn end() {}\n",
    )
    .unwrap();
    publish(&host, &old);
    assert!(app.refresh(&unchanged).is_err());
    let new = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    req["snapshot"] = json!(new);
    let answer = app.refresh(&serde_json::from_value(req).unwrap()).unwrap();
    assert_eq!(answer["snapshot"], new);
    let start = answer["views"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["symbol"]["name"] == "start")
        .unwrap();
    assert_eq!(start["symbol"]["source"]["line"], 2);
    assert_eq!(start["position"]["line"], 4);
    assert_eq!(start["position"]["reset"], false);
    assert_eq!(start["origin"]["number"], 2);
    assert_eq!(start["window"]["line_start"], start["position"]["section"]);
    assert_eq!(
        start["window"]["lines"].as_array().unwrap().last().unwrap()["number"],
        start["symbol"]["source"]["end_line"]
    );
    assert!(
        start["window"]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .all(|line| line["ending"].is_string())
    );
    assert!(
        start["window"]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["text"] == "    changed();")
    );
    assert_eq!(start["actions"][0]["name"], "changed");
}
#[test]
fn missing_functions_retain_unresolved_identity_and_invalid_batches_fail() {
    let (root, _cache, host, app) = fixture();
    let old = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let syms = symbols(&host, &app);
    let mut req = request(&old, &syms);
    std::fs::write(root.path().join("lib.rs"), "fn renamed() {}\n").unwrap();
    publish(&host, &old);
    req["snapshot"] = json!(host.application().snapshot().unwrap().id().as_str());
    let response = app
        .refresh(&serde_json::from_value(req.clone()).unwrap())
        .unwrap();
    assert!(
        response["views"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["state"] == "unresolved")
    );
    let mut duplicate = req.clone();
    duplicate["views"][1]["id"] = duplicate["views"][0]["id"].clone();
    assert!(
        app.refresh(&serde_json::from_value(duplicate).unwrap())
            .is_err()
    );
    let mut allowance = req.clone();
    allowance["views"] = json!(vec![req["views"][0].clone(); 25]);
    assert!(
        app.refresh(&serde_json::from_value(allowance).unwrap())
            .is_err()
    );
    assert!(
        serde_json::from_value::<live::Request>(
            json!({"snapshot":old,"views":[],"links":[],"command":"code"})
        )
        .is_err()
    );
}

#[test]
fn changed_ambiguous_or_missing_reading_hints_reset_safely() {
    let (root, _cache, host, app) = fixture();
    let old = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let syms = symbols(&host, &app);
    let mut req = request(&old, &syms);
    std::fs::write(
        root.path().join("lib.rs"),
        "fn start() {\n    end();\n    end();\n}\nfn end() {}\n",
    )
    .unwrap();
    publish(&host, &old);
    req["snapshot"] = json!(host.application().snapshot().unwrap().id().as_str());
    let answer = app
        .refresh(&serde_json::from_value(req.clone()).unwrap())
        .unwrap();
    let start = answer["views"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["symbol"]["name"] == "start")
        .unwrap();
    assert_eq!(start["position"]["reset"], true);
    assert_eq!(start["position"]["section"], 1);
    assert!(start["position"]["line"].is_null());
    for view in req["views"].as_array_mut().unwrap() {
        view["anchor"] = Value::Null;
    }
    assert!(
        app.refresh(&serde_json::from_value(req).unwrap()).unwrap()["views"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["position"]["reset"] == true)
    );
}
#[test]
fn refresh_bounds_and_identities_are_enforced_before_work() {
    let (_root, _cache, host, app) = fixture();
    let snap = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let syms = symbols(&host, &app);
    let req = request(&snap, &syms);
    for (field, value) in [
        ("tracking", json!("bad")),
        ("symbol", json!("bad")),
        ("version", json!("bad")),
        ("id", json!("x1")),
        ("area", json!("a".repeat(4097))),
        ("section_offset", json!(100_000_001)),
    ] {
        let mut bad = req.clone();
        bad["views"][0][field] = value;
        assert!(
            app.refresh(&serde_json::from_value(bad).unwrap()).is_err(),
            "{field}"
        );
    }
    for (field, value) in [
        ("offset", json!(100_000_001)),
        ("byte", json!(32769)),
        ("text", json!("a".repeat(1025))),
    ] {
        let mut bad = req.clone();
        bad["views"][0]["anchor"][field] = value;
        assert!(
            app.refresh(&serde_json::from_value(bad).unwrap()).is_err(),
            "{field}"
        );
    }
    let mut bad = req.clone();
    bad["snapshot"] = json!("bad");
    assert!(app.refresh(&serde_json::from_value(bad).unwrap()).is_err());
    for value in ["g".repeat(64), "F".repeat(64)] {
        let mut bad = req.clone();
        bad["views"][0]["tracking"] = json!(value);
        assert!(app.refresh(&serde_json::from_value(bad).unwrap()).is_err());
    }
    let mut too_many = req.clone();
    too_many["views"] = json!(
        (0..25)
            .map(|i| {
                let mut view = req["views"][0].clone();
                view["id"] = json!(format!("v{}", i + 1));
                view
            })
            .collect::<Vec<_>>()
    );
    assert!(
        app.refresh(&serde_json::from_value(too_many).unwrap())
            .is_err()
    );
    let link = json!({"id":"link","from":"v1","to":"v2","spelling":"end","kind":"candidate"});
    for (field, value) in [
        ("id", json!("x".repeat(1025))),
        ("from", json!("v".repeat(9))),
        ("to", json!("v".repeat(9))),
        ("spelling", json!("")),
        ("spelling", json!("x".repeat(257))),
        ("kind", json!("guessed")),
    ] {
        let mut bad = req.clone();
        let mut l = link.clone();
        l[field] = value;
        bad["links"] = json!([l]);
        assert!(
            app.refresh(&serde_json::from_value(bad).unwrap()).is_err(),
            "{field}"
        );
    }
    let mut bad = req.clone();
    bad["links"] = json!([link.clone(), link.clone()]);
    assert!(app.refresh(&serde_json::from_value(bad).unwrap()).is_err());
    let mut too_many = req.clone();
    too_many["links"] = json!(
        (0..97)
            .map(|i| {
                let mut item = link.clone();
                item["id"] = json!(format!("link{i}"));
                item
            })
            .collect::<Vec<_>>()
    );
    assert!(
        app.refresh(&serde_json::from_value(too_many).unwrap())
            .is_err()
    );
    let mut bad = req;
    bad["links"] = json!(vec![link; 97]);
    assert!(app.refresh(&serde_json::from_value(bad).unwrap()).is_err());
}
#[tokio::test]
async fn refresh_http_allows_a_bounded_working_set_and_rejects_commands() {
    use vizzi::{
        companion::Companion,
        editor::{Editor, EditorError, EditorLocation},
        review_application::ReviewApplication,
        review_source::CaptureLimits,
        server::{diagnostic_request, router_with_companion},
    };
    struct Recording;
    impl Editor for Recording {
        fn name(&self) -> &str {
            "unused"
        }
        fn open(&self, _: &EditorLocation) -> Result<(), EditorError> {
            panic!("refresh must not open an editor")
        }
    }
    let (_root, _cache, host, app) = fixture();
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let syms = symbols(&host, &app);
    let mut body = request(&snapshot, &syms);
    let mut views = Vec::new();
    for i in 0..24 {
        let mut v = body["views"][0].clone();
        v["id"] = json!(format!("v{}", i + 1));
        v["anchor"]["text"] = json!("x".repeat(1024));
        views.push(v);
    }
    body["views"] = json!(views);
    assert!(serde_json::to_vec(&body).unwrap().len() > 16 * 1024);
    let companion = Arc::new(Companion::new(host.clone(), None, Arc::new(Recording)).unwrap());
    let review =
        Arc::new(ReviewApplication::start(host.clone(), CaptureLimits::default()).unwrap());
    let router = router_with_companion(host, review, companion);
    let response = diagnostic_request(
        router.clone(),
        json!(1),
        "POST",
        "/api/visual/refresh",
        Some(body.clone()),
    )
    .await
    .unwrap();
    assert_eq!(response.status, 200, "{}", response.body);
    body["command"] = json!("code");
    let response = diagnostic_request(router, json!(1), "POST", "/api/visual/refresh", Some(body))
        .await
        .unwrap();
    assert_eq!(response.status, 422);
}

#[test]
fn reading_search_stays_in_its_function_and_admits_its_complete_long_line() {
    let (root, _cache, host, app) = fixture();
    let hint = format!("    // {}", "x".repeat(700));
    std::fs::write(
        root.path().join("lib.rs"),
        format!("fn start() {{\n{hint}\n}}\n"),
    )
    .unwrap();
    let old = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    publish(&host, &old);
    let old = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let syms = symbols(&host, &app);
    let mut req = request(&old, &syms);
    req["views"][0]["anchor"]["text"] = json!(hint);
    std::fs::write(
        root.path().join("lib.rs"),
        format!(
            "{}fn start() {{\n{hint}\n}}\n{}",
            "// prefix\n".repeat(202),
            "// tail\n".repeat(110)
        ),
    )
    .unwrap();
    publish(&host, &old);
    req["snapshot"] = json!(host.application().snapshot().unwrap().id().as_str());
    let result = app
        .refresh(&serde_json::from_value(req.clone()).unwrap())
        .unwrap();
    assert_eq!(result["views"][0]["position"]["line"], 204);
    assert_eq!(result["views"][0]["position"]["reset"], false);
    req["views"][0]["anchor"] = json!({"offset":2,"byte":0,"text":"}"});
    let result = app.refresh(&serde_json::from_value(req).unwrap()).unwrap();
    assert_eq!(result["views"][0]["position"]["line"], 205);
}

#[test]
fn valid_maximum_presentation_hints_remain_bounded_and_accepted() {
    let (_root, _cache, host, app) = fixture();
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let syms = symbols(&host, &app);
    let mut req = request(&snapshot, &syms);
    req["views"][0]["id"] = json!("v1234567");
    req["views"][0]["area"] = json!("x".repeat(4096));
    req["views"][0]["section_offset"] = json!(100_000_000);
    req["views"][0]["anchor"] = json!({"offset":100_000_000,"byte":32768,"text":"x".repeat(1024)});
    req["links"]=json!((0..96).map(|i|json!({"id":format!("{i:04}{}","x".repeat(1020)),"from":"v1234567","to":"v7654321","spelling":"x".repeat(256),"kind":"candidate"})).collect::<Vec<_>>());
    let result = app.refresh(&serde_json::from_value(req).unwrap()).unwrap();
    assert_eq!(result["links"].as_array().unwrap().len(), 96);
    assert!(
        result["views"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["state"] == "current")
    );
}

#[test]
fn refresh_captures_the_complete_declaration_origin_and_only_its_source_range() {
    let (root, _cache, host, app) = fixture();
    let old = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let syms = symbols(&host, &app);
    let mut req = request(&old, &syms);
    let declaration = format!("{}fn start() {{ changed(); }}", " ".repeat(400));
    std::fs::write(
        root.path().join("lib.rs"),
        format!("// moved\n{declaration}\n{}", "// tail\n".repeat(110)),
    )
    .unwrap();
    publish(&host, &old);
    req["snapshot"] = json!(host.application().snapshot().unwrap().id().as_str());
    let result = app.refresh(&serde_json::from_value(req).unwrap()).unwrap();
    let start = result["views"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["state"] == "current")
        .unwrap();
    assert_eq!(start["origin"]["text"], declaration);
    assert_eq!(start["origin"]["right"], false);
    assert_eq!(start["window"]["lines"].as_array().unwrap().len(), 1);
    assert_eq!(start["window"]["lines"][0]["number"], 2);
    assert_eq!(start["window"]["lines"][0]["right"], false);
}

#[test]
fn unchanged_reading_is_clamped_to_the_loaded_section_of_a_long_function() {
    let (root, _cache, host, app) = fixture();
    let old = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    std::fs::write(
        root.path().join("lib.rs"),
        format!(
            "{}fn start() {{\n{}}}\n",
            "// prefix\n".repeat(10),
            "    step();\n".repeat(220)
        ),
    )
    .unwrap();
    publish(&host, &old);
    let snapshot = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let mut req = request(&snapshot, &symbols(&host, &app));
    req["views"][0]["section_offset"] = json!(20);
    req["views"][0]["anchor"] = json!({"offset":180,"byte":0,"text":"    step();"});
    let result = app.refresh(&serde_json::from_value(req).unwrap()).unwrap();
    assert_eq!(result["views"][0]["position"]["section"], 31);
    assert_eq!(result["views"][0]["position"]["line"], 126);
    assert!(result["views"][0]["window"].is_null());
}

#[test]
fn changed_reading_finds_the_last_line_in_its_bounded_neighbourhood() {
    let (root, _cache, host, app) = fixture();
    let old = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let source = |prefix, before| {
        format!(
            "{}fn start() {{\n{}    checkpoint();\n{}}}\n",
            "// prefix\n".repeat(prefix),
            "    step();\n".repeat(before),
            "    after();\n".repeat(100)
        )
    };
    std::fs::write(root.path().join("lib.rs"), source(200, 79)).unwrap();
    publish(&host, &old);
    let old = host
        .application()
        .snapshot()
        .unwrap()
        .id()
        .as_str()
        .to_owned();
    let mut req = request(&old, &symbols(&host, &app));
    req["views"][0]["section_offset"] = json!(60);
    req["views"][0]["anchor"] = json!({"offset":80,"byte":32768,"text":"    checkpoint();"});
    std::fs::write(root.path().join("lib.rs"), source(300, 142)).unwrap();
    publish(&host, &old);
    req["snapshot"] = json!(host.application().snapshot().unwrap().id().as_str());
    let result = app.refresh(&serde_json::from_value(req).unwrap()).unwrap();
    assert_eq!(result["views"][0]["position"]["section"], 424);
    assert_eq!(result["views"][0]["position"]["line"], 444);
    assert_eq!(
        result["views"][0]["position"]["byte"],
        "    checkpoint();".len()
    );
    assert_eq!(result["views"][0]["position"]["reset"], false);
    assert!(
        result["views"][0]["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["name"] == "checkpoint")
    );
}
