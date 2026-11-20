//! Immutable per-function history through production Git and analysis interfaces.
use serde_json::{Value, json};
use std::{process::Command, sync::Arc};
use vizzi::function_history::CapturedRequest;
use vizzi::function_history::{CompareRequest, ComparisonWindow};
fn captured_source(frame: &Value, owner: Value) -> Value {
    let s = &frame["symbol"]["source"];
    json!({"owner":owner,"snapshot":frame["snapshot"],"file":s["file"],"version":s["version"],"symbol":frame["symbol"]["id"]})
}
fn captured_query(
    history: &FunctionHistory,
    source: &Value,
    query: Value,
) -> Result<Value, vizzi::companion::CompanionError> {
    let request: CapturedRequest =
        serde_json::from_value(json!({"source":source,"query":query})).unwrap();
    history.captured_query(&request)
}
fn call_named(history: &FunctionHistory, source: &Value, name: &str) -> Value {
    captured_query(history, source, json!({"kind":"calls","offset":0})).unwrap()["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
        .clone()
}
#[test]
fn captured_calls_follow_old_helpers_and_recursive_chains_without_current_source_fallback() {
    let (root, _cache, host, history, request, _) = fixture("");
    let old = "fn target() { helper(); external(); }\nfn helper() { target(); }\n";
    let revision = commit(root.path(), old);
    commit(
        root.path(),
        "fn target() { let changed=1; helper(); external(); }\nfn helper() { current_only(); }\n",
    );
    std::fs::write(root.path().join("external.rs"), "fn external() {}\n").unwrap();
    let request = refresh_request(&host, &request);
    let frame = history.start(&request).unwrap();
    assert_eq!(frame["commit"], revision);
    let owner = json!({"kind":"history","token":frame["token"],"index":frame["index"]});
    let source = captured_source(&frame, owner.clone());
    let actions = captured_query(
        &history,
        &source,
        json!({"kind":"actions","line_start":1,"line_count":96,"offset":0}),
    )
    .unwrap();
    assert_eq!(actions["matching"], 2);
    history.step(&step(&frame["token"], 1)).unwrap();
    let helper = call_named(&history, &source, "helper");
    let definition = captured_query(
        &history,
        &source,
        json!({"kind":"definition","call":helper["id"]}),
    )
    .unwrap();
    assert_eq!(definition["invocation"]["candidate_count"], 1);
    assert_eq!(definition["complete"], true);
    assert_eq!(definition["coverage"]["available_files"], 1);
    let opened=captured_query(&history,&source,json!({"kind":"open","call":helper["id"],"target":definition["invocation"]["candidates"][0]["id"]})).unwrap();
    assert_eq!(
        opened["frame"]["window"]["lines"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        opened["frame"]["window"]["outline"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(opened["frame"]["window"]["lines"][0]["ending"], "\n");
    assert!(
        opened["frame"]["window"]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["text"]
                .as_str()
                .unwrap()
                .contains("fn helper() { target(); }"))
    );
    assert_eq!(opened["frame"]["captured"]["label"], revision);
    assert_eq!(opened["frame"]["captured"]["owner"], owner);
    let child = captured_source(&opened["frame"], owner);
    let back = call_named(&history, &child, "target");
    assert_eq!(back["candidates"][0]["id"], source["symbol"]);
    let external = call_named(&history, &source, "external");
    assert_eq!(external["candidate_count"], 0);
    let mut wrong = source.clone();
    wrong["symbol"] = child["symbol"].clone();
    assert!(
        captured_query(
            &history,
            &wrong,
            json!({"kind":"definition","call":helper["id"]})
        )
        .is_err()
    );
    for field in ["snapshot", "file", "version", "symbol"] {
        let mut bad = source.clone();
        bad[field] = json!("0".repeat(64));
        assert!(captured_query(&history, &bad, json!({"kind":"calls","offset":0})).is_err());
    }
    assert!(
        captured_query(
            &history,
            &source,
            json!({"kind":"open","call":helper["id"],"target":source["symbol"]})
        )
        .is_err()
    );
}
#[test]
fn captured_comparison_sides_keep_their_own_call_destinations_and_exact_ownership() {
    let (root, _cache, host, history, request, _) = fixture("");
    commit(
        root.path(),
        "fn target() { helper(); }\nfn helper() { old(); }\n",
    );
    commit(
        root.path(),
        "fn target() { let changed=1; helper(); }\nfn helper() { new(); }\n",
    );
    let request = refresh_request(&host, &request);
    let pair = history.compare(&compare_current(&request)).unwrap();
    for (side, marker) in [("older", "old();"), ("newer", "new();")] {
        let owner = json!({"kind":"comparison","token":pair["token"],"side":side});
        let source = captured_source(&pair[side], owner);
        let call = call_named(&history, &source, "helper");
        let opened = captured_query(
            &history,
            &source,
            json!({"kind":"open","call":call["id"],"target":call["candidates"][0]["id"]}),
        )
        .unwrap();
        assert!(
            opened["frame"]["window"]["lines"][0]["text"]
                .as_str()
                .unwrap()
                .contains(marker)
        );
        let mut bad = source.clone();
        bad["owner"]["side"] = json!(if side == "older" { "newer" } else { "older" });
        assert!(captured_query(&history, &bad, json!({"kind":"calls","offset":0})).is_err());
    }
}
#[test]
fn captured_rust_self_bindings_and_same_name_ambiguity_retain_native_evidence() {
    let (root, _cache, host, history, request, _) = fixture("");
    let old = "struct A; struct B;\nfn target() { A.run(); dynamic(); }\nimpl A { fn run(&self) { self.helper(); } fn helper(&self) {} }\nimpl B { fn helper(&self) {} }\nfn dynamic() { value.helper(); }\n";
    commit(root.path(), old);
    commit(
        root.path(),
        &old.replace("A.run(); dynamic();", "let now=1; A.run(); dynamic();"),
    );
    let request = refresh_request(&host, &request);
    let frame = history.start(&request).unwrap();
    let owner = json!({"kind":"history","token":frame["token"],"index":frame["index"]});
    let source = captured_source(&frame, owner.clone());
    for (name, binding, count) in [("run", Some("rust_self_receiver"), 1), ("dynamic", None, 2)] {
        let call = call_named(&history, &source, name);
        let opened = captured_query(
            &history,
            &source,
            json!({"kind":"open","call":call["id"],"target":call["candidates"][0]["id"]}),
        )
        .unwrap();
        let child = captured_source(&opened["frame"], owner.clone());
        let helper = call_named(&history, &child, "helper");
        assert_eq!(helper["candidate_count"], count);
        assert_eq!(helper["binding"].as_str(), binding);
    }
}

#[test]
fn captured_partial_analysis_keeps_its_coverage_and_does_not_claim_complete_name_candidates() {
    let (root, _cache, host, history, request, _) = fixture("");
    let old = "fn target() { helper(); }\nfn helper() {}\n@@@\n";
    commit(root.path(), old);
    commit(
        root.path(),
        &old.replace("helper();", "let phase=1; helper();"),
    );
    let request = refresh_request(&host, &request);
    let frame = history.start(&request).unwrap();
    let source = captured_source(
        &frame,
        json!({"kind":"history","token":frame["token"],"index":frame["index"]}),
    );
    let call = call_named(&history, &source, "helper");
    let result = captured_query(
        &history,
        &source,
        json!({"kind":"definition","call":call["id"]}),
    )
    .unwrap();
    assert_eq!(result["invocation"]["candidate_count"], 1);
    assert_eq!(result["complete"], false);
    assert_eq!(result["coverage"]["partial_files"], 1);
    let mut expired = source.clone();
    expired["owner"]["token"] = json!("0".repeat(64));
    assert!(captured_query(&history, &expired, json!({"kind":"calls","offset":0})).is_err());
    for (start, count) in [(0, 1), (1, 0), (1, 97)] {
        assert!(
            captured_query(
                &history,
                &source,
                json!({"kind":"actions","line_start":start,"line_count":count,"offset":0})
            )
            .is_err()
        );
    }
}

#[test]
fn captured_destination_frames_bound_the_declaration_and_preserve_its_complete_origin() {
    let (root, _cache, host, history, request, _) = fixture("");
    let indent = " ".repeat(400);
    let code = |phase| {
        format!(
            "fn target() {{ let phase={phase}; helper(); }}\n// before helper\n{indent}fn helper() {{\n{indent}    let line=1;\n{indent}    let next=2;\n{indent}}}\n{}fn after() {{}}\n",
            "// later context\n".repeat(120)
        )
    };
    commit(root.path(), &code(1));
    commit(root.path(), &code(2));
    let request = refresh_request(&host, &request);
    let frame = history.start(&request).unwrap();
    let source = captured_source(
        &frame,
        json!({"kind":"history","token":frame["token"],"index":frame["index"]}),
    );
    let call = call_named(&history, &source, "helper");
    let result = captured_query(
        &history,
        &source,
        json!({"kind":"open","call":call["id"],"target":call["candidates"][0]["id"]}),
    )
    .unwrap();
    let frame = &result["frame"];
    assert_eq!(frame["window"]["lines"].as_array().unwrap().len(), 4);
    assert_eq!(frame["window"]["lines"][0]["number"], 3);
    assert_eq!(frame["window"]["lines"][3]["number"], 6);
    assert_eq!(frame["origin"]["number"], 3);
    assert_eq!(frame["origin"]["right"], false);
    assert!(
        frame["origin"]["text"]
            .as_str()
            .unwrap()
            .starts_with(&(indent + "fn helper"))
    );
}
use vizzi::{
    cache::Cache,
    engine::Engine,
    function_history::{FunctionHistory, Request, Step},
    host::{Host, HostOptions},
    source::Directory,
};
use vizzi_core::{projection::WindowQuery, structure::Structure};
fn compare_current(request: &Request) -> CompareRequest {
    serde_json::from_value(json!({"kind":"current","source":request})).unwrap()
}

#[test]
fn historical_rust_module_ignores_unrelated_non_utf8_source() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let repo = root.path();
    git(repo, &["init", "-q"]);
    std::fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = 'demo'\n[lib]\npath = 'lib.rs'\n",
    )
    .unwrap();
    std::fs::write(repo.join("unrelated.rs"), [0xff, 0xfe, 0xfd]).unwrap();
    git(repo, &["add", "--", "Cargo.toml", "unrelated.rs"]);
    commit(repo, "fn target() { old(); }\n");
    std::fs::write(repo.join("lib.rs"), "fn target() { working(); }\n").unwrap();

    let host = history_host(repo, cache.path(), None);
    let (request, module) = module_request(&host, "lib.rs");
    assert_eq!(module["label"], "demo");
    let pair = FunctionHistory::new(host)
        .compare(&compare_current(&request))
        .unwrap();
    assert_eq!(pair["older"]["symbol"]["module"], module);
}

#[test]
fn historical_python_module_uses_the_package_chain_at_each_revision() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let repo = root.path();
    git(repo, &["init", "-q"]);
    std::fs::create_dir_all(repo.join("pkg/sub")).unwrap();
    std::fs::write(repo.join("pkg/sub/__init__.py"), "").unwrap();
    let initializer = repo.join("pkg/__init__.py");
    std::fs::create_dir(&initializer).unwrap();
    std::fs::write(initializer.join("marker"), "").unwrap();
    let worker = "pkg/sub/worker.py";
    std::fs::write(repo.join(worker), "def target():\n    return 1\n").unwrap();
    git(repo, &["add", "--", "pkg"]);
    commit_staged(repo);
    std::fs::remove_file(initializer.join("marker")).unwrap();
    std::fs::remove_dir(&initializer).unwrap();
    std::fs::write(initializer, "").unwrap();
    std::fs::write(repo.join(worker), "def target():\n    return 2\n").unwrap();
    git(repo, &["add", "--", "pkg"]);
    commit_staged(repo);
    std::fs::write(repo.join(worker), "def target():\n    return 3\n").unwrap();

    let host = history_host(repo, cache.path(), None);
    let (request, module) = module_request(&host, worker);
    assert_eq!(module["label"], "pkg.sub.worker");
    let history = FunctionHistory::new(host);
    let pair = history.compare(&compare_current(&request)).unwrap();
    for side in ["newer", "older"] {
        assert_eq!(pair[side]["symbol"]["module"], module, "{side}");
    }
    let newer = history.start(&request).unwrap();
    let older = history.step(&step(&newer["token"], 1)).unwrap();
    assert_eq!(older["symbol"]["module"]["label"], "sub.worker");
    assert_eq!(older["symbol"]["module"]["id"], module["id"]);
}

#[test]
fn historical_typescript_labels_use_the_package_name_at_each_revision() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let repo = root.path();
    git(repo, &["init", "-q"]);
    let path = "src/main.ts";
    std::fs::create_dir(repo.join("src")).unwrap();
    let nearest = "src/package.json";
    let oversized = "x".repeat(128 * 1024 + 1);
    std::fs::write(repo.join("package.json"), r#"{"name":"@work/root"}"#).unwrap();
    std::fs::write(repo.join(nearest), &oversized).unwrap();
    std::fs::write(repo.join(path), "function target() { return 0; }\n").unwrap();
    git(repo, &["add", "--", path, "package.json", nearest]);
    commit_staged(repo);
    std::fs::write(repo.join("package.json"), oversized).unwrap();
    std::fs::write(repo.join(nearest), r#"{"name":"@work/old"}"#).unwrap();
    std::fs::write(repo.join(path), "function target() { return 1; }\n").unwrap();
    git(repo, &["add", "--", path, "package.json", nearest]);
    commit_staged(repo);
    std::fs::write(repo.join(nearest), r#"{"name":"@work/current"}"#).unwrap();
    std::fs::write(repo.join(path), "function target() { return 2; }\n").unwrap();

    let host = history_host(repo, cache.path(), None);
    let (request, live) = module_request(&host, path);
    assert_eq!(live["label"], "@work/current");
    assert_eq!(live["root"], nearest);
    let history = FunctionHistory::new(host);
    let pair = history.compare(&compare_current(&request)).unwrap();
    assert_eq!(pair["newer"]["symbol"]["module"], live);
    let old = &pair["older"]["symbol"]["module"];
    assert_eq!(old["label"], "@work/old");
    assert_eq!(old["id"], live["id"]);
    let first = history.start(&request).unwrap();
    let older = history.step(&step(&first["token"], 1)).unwrap();
    assert!(older["symbol"]["module"].is_null());
}

#[test]
fn ruby_history_and_captured_calls_use_the_declared_namespace() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let repo = root.path();
    git(repo, &["init", "-q"]);
    let path = "reading.rb";
    let source = |value| {
        format!(
            "module Audio\n  def self.target; helper({value}); end\n  def self.helper(value); value; end\nend\n"
        )
    };
    std::fs::write(repo.join(path), source(1)).unwrap();
    git(repo, &["add", "--", path]);
    commit_staged(repo);
    std::fs::write(repo.join(path), source(2)).unwrap();

    let host = history_host(repo, cache.path(), None);
    let (request, module) = module_request(&host, path);
    assert_eq!(module["label"], "Audio");
    assert_eq!(module["basis"], "ruby_declarations");
    let history = FunctionHistory::new(host);
    let pair = history.compare(&compare_current(&request)).unwrap();
    for side in ["older", "newer"] {
        assert_eq!(pair[side]["symbol"]["module"], module);
    }
    let source = captured_source(
        &pair["older"],
        json!({"kind":"comparison","token":pair["token"],"side":"older"}),
    );
    assert_eq!(
        open_module_helper(&history, &source)["symbol"]["module"],
        module
    );
}

#[test]
fn comparison_preserves_the_saved_module_and_the_verified_historical_peer() {
    let (root, _cache, history, request, live_module) = module_fixture(
        &[("task", &module_source("1"))],
        "task",
        &module_source("2"),
    );
    assert_eq!(live_module["label"], "demo::task");
    let pair = history.compare(&compare_current(&request)).unwrap();
    for side in ["newer", "older"] {
        assert_eq!(pair[side]["symbol"]["module"], live_module, "{side}");
    }

    declare_module(root.path(), "changed_after_capture");
    let source = captured_source(
        &pair["newer"],
        json!({"kind":"comparison","token":pair["token"],"side":"newer"}),
    );
    let helper = open_module_helper(&history, &source);
    assert_eq!(helper["symbol"]["module"]["label"], "demo::task::nested");
}

#[test]
fn history_recovers_module_bindings_from_each_commit_even_with_repeated_file_bytes() {
    let repeated = module_source("1");
    let (_root, _cache, history, request, _) = module_fixture(
        &[
            ("old", &repeated),
            ("ambiguous", &module_source("2")),
            ("new", &repeated),
        ],
        "live",
        &module_source("3"),
    );
    let newer = history.start(&request).unwrap();
    assert_eq!(newer["symbol"]["module"]["label"], "demo::new");
    let pair = history.compare(&compare_current(&request)).unwrap();
    assert_eq!(pair["older"]["symbol"]["module"], newer["symbol"]["module"]);
    assert_eq!(pair["newer"]["symbol"]["module"]["label"], "demo::live");
    let middle = history.step(&step(&newer["token"], 1)).unwrap();
    assert!(middle["symbol"]["module"].is_null());
    let older = history.step(&step(&newer["token"], 2)).unwrap();
    assert_eq!(older["symbol"]["module"]["label"], "demo::old");
    assert_eq!(
        older["symbol"]["source"]["version"],
        newer["symbol"]["source"]["version"]
    );
    assert_ne!(older["snapshot"], newer["snapshot"]);
    let source = captured_source(
        &newer,
        json!({"kind":"history","token":newer["token"],"index":newer["index"]}),
    );
    let helper = open_module_helper(&history, &source);
    assert_eq!(helper["symbol"]["module"]["label"], "demo::new::nested");
}

#[test]
fn history_keeps_source_readable_when_module_context_exceeds_its_allowance() {
    let pad = |value, bytes: usize| {
        let mut source = module_source(value);
        source.push_str(&" ".repeat(bytes - source.len()));
        source
    };
    let (_root, _cache, history, request, _) = module_fixture(
        &[
            ("task", &pad("1", 1024 * 1024)),
            ("task", &pad("2", 1024 * 1024 + 1)),
        ],
        "task",
        &module_source("3"),
    );
    let newer = history.start(&request).unwrap();
    assert!(newer["symbol"]["module"].is_null());
    assert!(text(&newer).contains("helper(2)"));
    let older = history.step(&step(&newer["token"], 1)).unwrap();
    assert_eq!(older["symbol"]["module"]["label"], "demo::task");
}

fn open_module_helper(history: &FunctionHistory, source: &Value) -> Value {
    let invocation = call_named(history, source, "helper");
    let opened = captured_query(
        history,
        source,
        json!({"kind":"open","call":invocation["id"],"target":invocation["candidates"][0]["id"]}),
    )
    .unwrap();
    opened["frame"].clone()
}

fn declare_module(root: &std::path::Path, name: &str) {
    let declaration = match name {
        "ambiguous" => "#[path = \"lib.rs\"] mod left;\n#[path = \"lib.rs\"] mod right;\n".into(),
        _ => format!("#[path = \"lib.rs\"] mod {name};\n"),
    };
    std::fs::write(root.join("root.rs"), declaration).unwrap();
}

fn module_source(value: &str) -> String {
    format!("fn target() {{ helper({value}); }}\nmod nested {{ pub fn helper(_: u32) {{}} }}\n")
}

fn module_fixture(
    versions: &[(&str, &str)],
    working_name: &str,
    working_source: &str,
) -> (
    tempfile::TempDir,
    tempfile::TempDir,
    FunctionHistory,
    Request,
    Value,
) {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    std::fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nedition = \"2024\"\n[lib]\npath = \"root.rs\"\n",
    )
    .unwrap();
    for (name, source) in versions {
        declare_module(root.path(), name);
        git(root.path(), &["add", "--", "Cargo.toml", "root.rs"]);
        commit(root.path(), source);
    }
    declare_module(root.path(), working_name);
    std::fs::write(root.path().join("lib.rs"), working_source).unwrap();
    let host = history_host(root.path(), cache.path(), None);
    let (request, module) = module_request(&host, "lib.rs");
    let history = FunctionHistory::new(host);
    (root, cache, history, request, module)
}

fn module_request(host: &Host, path: &str) -> (Request, Value) {
    let snapshot = host.application().snapshot().unwrap();
    let symbol = Structure::new(&snapshot)
        .functions(path, "target", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let request = Request {
        snapshot: snapshot.id().as_str().into(),
        file: symbol.source.file,
        version: symbol.source.version,
        tracking: symbol.tracking,
    };
    let module = serde_json::to_value(symbol.module).unwrap();
    (request, module)
}
fn comparison_window(pair: &Value, side: &str) -> ComparisonWindow {
    let frame = &pair[side];
    let s = &frame["symbol"]["source"];
    serde_json::from_value(json!({"token":pair["token"],"side":side,"snapshot":frame["snapshot"],"file":s["file"],"version":s["version"]})).unwrap()
}
#[test]
fn comparison_freezes_saved_source_and_head_independently_of_disk_git_and_history_selection() {
    let (root, _cache, host, history, request, commits) =
        fixture("fn target() { working(); }\nfn other() { outside(); }\n");
    let pair = history.compare(&compare_current(&request)).unwrap();
    assert_eq!(pair["older"]["history"]["commit"], commits[3]);
    assert_eq!(pair["older"]["changes"], json!([[2, 3]]));
    assert_eq!(pair["newer"]["changes"], json!([[1, 2]]));
    for side in ["older", "newer"] {
        let frame = &pair[side];
        assert_eq!(frame["window"]["lines"].as_array().unwrap().len(), 1);
        assert_eq!(frame["window"]["outline"].as_array().unwrap().len(), 1);
        assert_eq!(frame["window"]["lines"][0]["ending"], "\n");
        assert_eq!(frame["origin"]["number"], frame["symbol"]["source"]["line"]);
    }
    assert!(pair["newer"]["history"].is_null());
    assert_eq!(pair["edits"].as_array().unwrap().len(), 1);
    assert_eq!(
        pair["edits"][0]["base"]["lines"],
        json!({"start":1,"end":2})
    );
    let token = pair["older"]["history"]["token"].clone();
    history.step(&step(&token, 1)).unwrap();
    std::fs::write(root.path().join("lib.rs"), "fn target() { later(); }\n").unwrap();
    host.request(vizzi::source::CaptureScope::Reconcile);
    std::fs::rename(root.path().join(".git"), root.path().join("hidden-git")).unwrap();
    for (side, marker) in [("newer", "working()"), ("older", "newest()")] {
        let binding = comparison_window(&pair, side);
        let data = history
            .comparison_window(&binding, &WindowQuery::default())
            .unwrap();
        assert!(
            data["lines"]
                .as_array()
                .unwrap()
                .iter()
                .any(|l| l["text"].as_str().unwrap().contains(marker))
        );
        for field in ["version", "snapshot", "file", "token", "side"] {
            let mut bad = serde_json::to_value(&binding).unwrap();
            bad[field] = json!("0".repeat(64));
            assert!(
                history
                    .comparison_window(
                        &serde_json::from_value(bad).unwrap(),
                        &WindowQuery::default()
                    )
                    .is_err(),
                "accepted an invalid {field}"
            );
        }
    }
}
#[test]
fn historical_comparison_uses_the_selected_version_and_next_distinct_ancestor() {
    let (_root, _cache, _host, history, request, commits) = fixture("");
    let first = history.start(&request).unwrap();
    let s = &first["symbol"]["source"];
    let request=serde_json::from_value(json!({"kind":"historical","source":{"token":first["token"],"index":first["index"],"snapshot":first["snapshot"],"file":s["file"],"version":s["version"]}})).unwrap();
    let pair = history.compare(&request).unwrap();
    assert_eq!(pair["newer"]["history"]["commit"], commits[1]);
    assert_eq!(pair["older"]["history"]["commit"], commits[0]);
}
#[test]
fn comparison_finds_edits_after_line_96_and_ignores_file_movement_and_unrelated_functions() {
    let (root, _cache, host, history, request, _) = fixture("");
    let body = format!(
        "fn target() {{\n{}    before();\n}}\nfn other() {{ before(); }}\n",
        "    // context\n".repeat(160)
    );
    commit(root.path(), &body);
    commit(
        root.path(),
        &(String::from("// shifted\n\n")
            + &body
                .replace("    before();", "    after();")
                .replace("other() { before();", "other() { unrelated();")),
    );
    let request = refresh_request(&host, &request);
    let pair = history.compare(&compare_current(&request)).unwrap();
    assert_eq!(pair["edits"].as_array().unwrap().len(), 1);
    assert_eq!(pair["edits"][0]["base"]["lines"]["start"], 161);
    assert_eq!(pair["edits"][0]["working"]["lines"]["start"], 163);
    for side in ["older", "newer"] {
        assert!(pair[side]["window"]["line_start"].as_u64().unwrap() > 96);
        assert_eq!(
            pair[side]["origin"]["number"],
            pair[side]["symbol"]["source"]["line"]
        );
    }
}
#[test]
fn comparison_admission_expires_old_pairs_and_rejects_foreign_bindings() {
    let (_root, _cache, _host, history, request, _) = fixture("");
    let first = history.compare(&compare_current(&request)).unwrap();
    for _ in 0..12 {
        history.compare(&compare_current(&request)).unwrap();
    }
    assert!(
        history
            .comparison_window(&comparison_window(&first, "older"), &WindowQuery::default())
            .unwrap_err()
            .to_string()
            .contains("expired")
    );
    let mut bad = serde_json::to_value(&request).unwrap();
    bad["snapshot"] = json!("0".repeat(64));
    let bad: Request = serde_json::from_value(bad).unwrap();
    assert!(history.compare(&compare_current(&bad)).is_err());
}

#[test]
fn comparison_window_binding_accepts_source_projection_parameters() {
    let uri="/api/function-comparisons/window?token=t&side=newer&snapshot=s&file=f&version=v&line_start=97&line_count=96&outline_count=1&reflow=true".parse().unwrap();
    let axum::extract::Query(binding) =
        axum::extract::Query::<ComparisonWindow>::try_from_uri(&uri).unwrap();
    let axum::extract::Query(window) =
        axum::extract::Query::<WindowQuery>::try_from_uri(&uri).unwrap();
    assert_eq!(binding.side, "newer");
    assert_eq!(window.line_start, Some(97));
    assert_eq!(window.reflow, Some(true));
}

#[test]
fn comparison_source_byte_allowance_is_inclusive_and_independent_of_history_sessions() {
    let (root, _cache, host, history, request, _) = fixture("");
    let large = |body: &str| {
        format!(
            "/*{}*/\n{body}",
            "x".repeat(2 * 1024 * 1024 - 5 - body.len())
        )
    };
    commit(root.path(), &large("fn target() { old(); }\n"));
    commit(root.path(), &large("fn target() { fresh(); }\n"));
    let request = refresh_request(&host, &request);
    let first = history.compare(&compare_current(&request)).unwrap();
    for _ in 0..7 {
        history.compare(&compare_current(&request)).unwrap();
    }
    assert!(
        history
            .comparison_window(
                &comparison_window(&first, "older"),
                &WindowQuery {
                    line_start: Some(2),
                    ..WindowQuery::default()
                }
            )
            .is_ok()
    );
    history.compare(&compare_current(&request)).unwrap();
    assert!(
        history
            .comparison_window(&comparison_window(&first, "older"), &WindowQuery::default())
            .unwrap_err()
            .to_string()
            .contains("expired")
    );
}

#[test]
fn comparison_origin_keeps_the_complete_declaration_indent_when_the_edit_is_on_a_later_page() {
    let (root, _cache, host, history, request, _) = fixture("");
    let indent = " ".repeat(400);
    let body = |name: &str| {
        format!(
            "{indent}fn target() {{\n{}{indent}    let {name}=1;\n{indent}}}\n",
            format!("{indent}    // context\n").repeat(120)
        )
    };
    commit(root.path(), &body("old"));
    commit(root.path(), &body("new"));
    let request = refresh_request(&host, &request);
    let pair = history.compare(&compare_current(&request)).unwrap();
    for side in ["older", "newer"] {
        assert!(pair[side]["window"]["line_start"].as_u64().unwrap() > 96);
        assert_eq!(pair[side]["origin"]["right"], false);
        assert!(
            pair[side]["origin"]["text"]
                .as_str()
                .unwrap()
                .starts_with(&(indent.clone() + "fn target"))
        );
    }
}

fn git(root: &std::path::Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn commit(root: &std::path::Path, text: &str) -> String {
    std::fs::write(root.join("lib.rs"), text).unwrap();
    git(root, &["add", "--", "lib.rs"]);
    commit_staged(root)
}

fn commit_staged(root: &std::path::Path) -> String {
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ],
    );
    git(root, &["rev-parse", "HEAD"])
}
fn refresh_request(host: &Host, previous: &Request) -> Request {
    let expected: Vec<_> = std::fs::read_dir(host.root())
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "rs"))
        .map(|entry| {
            let source = vizzi_core::model::CapturedSource::capture(
                std::fs::read(entry.path()).unwrap(),
                None,
            );
            (
                entry.file_name().into_string().unwrap(),
                source.content().clone(),
            )
        })
        .collect();
    host.request(vizzi::source::CaptureScope::Reconcile);
    let at = std::time::Instant::now();
    let snapshot = loop {
        let snapshot = host.application().snapshot().unwrap();
        if snapshot.documents().len() == expected.len()
            && expected.iter().all(|(path, content)| {
                snapshot
                    .file_by_path(path)
                    .is_some_and(|doc| doc.content() == content)
            })
        {
            break snapshot;
        }
        assert!(
            at.elapsed() < std::time::Duration::from_secs(10),
            "Fixture source did not settle after {}",
            previous.snapshot
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let s = Structure::new(&snapshot)
        .functions("", "target", 0)
        .unwrap()
        .items
        .into_iter()
        .map(|e| e.symbol)
        .find(|s| s.parents.is_empty())
        .unwrap();
    serde_json::from_value(json!({"snapshot":snapshot.id().as_str(),"file":s.source.file,"version":s.source.version,"tracking":s.tracking})).unwrap()
}
type Fixture = (
    tempfile::TempDir,
    tempfile::TempDir,
    Arc<Host>,
    FunctionHistory,
    Request,
    Vec<String>,
);
fn fixture(working: &str) -> Fixture {
    fixture_with_budget(working, None)
}
fn fixture_with_budget(working: &str, budget: Option<Arc<vizzi::admission::Admission>>) -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    let a = commit(root.path(), "fn target() { old(); }\nfn other() {}\n");
    let b = commit(root.path(), "fn target() { middle(); }\nfn other() {}\n");
    let c = commit(
        root.path(),
        "// move\nfn target() { middle(); }\nfn other() { unrelated(); }\n",
    );
    let d = commit(
        root.path(),
        "// move\nfn target() { newest(); }\nfn other() { unrelated(); }\n",
    );
    if !working.is_empty() {
        std::fs::write(root.path().join("lib.rs"), working).unwrap();
    }
    let host = history_host(root.path(), cache.path(), budget);
    let snapshot = host.application().snapshot().unwrap();
    let symbol = Structure::new(&snapshot)
        .functions("", "target", 0)
        .unwrap()
        .items[0]
        .symbol
        .clone();
    let request = serde_json::from_value(json!({"snapshot":snapshot.id().as_str(),"file":symbol.source.file,"version":symbol.source.version,"tracking":symbol.tracking})).unwrap();
    let history = FunctionHistory::new(host.clone());
    (root, cache, host, history, request, vec![a, b, c, d])
}

fn history_host(
    root: &std::path::Path,
    cache: &std::path::Path,
    budget: Option<Arc<vizzi::admission::Admission>>,
) -> Arc<Host> {
    let engine = Engine::new(
        Directory::open(root).unwrap(),
        Cache::new(cache.join("analysis")),
        1,
    )
    .unwrap();
    let engine = if let Some(budget) = budget {
        engine.with_admission(budget)
    } else {
        engine
    };
    Arc::new(
        Host::start_with_options(
            engine,
            HostOptions {
                watch: false,
                reconcile_interval: std::time::Duration::from_secs(3600),
                idle_maintenance: false,
            },
        )
        .unwrap(),
    )
}

#[test]
fn missing_declarations_stop_history_without_losing_the_selected_capture() {
    let (root, _cache, host, history, request, _) = fixture("");
    commit(root.path(), "fn renamed() {}\n");
    commit(root.path(), "fn target() { reintroduced(); }\n");
    commit(root.path(), "fn target() { current(); }\n");
    let request = refresh_request(&host, &request);
    let first = history.start(&request).unwrap();
    let error = history
        .step(&step(&first["token"], 1))
        .unwrap_err()
        .to_string();
    assert!(error.contains("missing or ambiguous"), "{error}");
    assert!(history.step(&step(&first["token"], 1)).is_err());
    let again = history.step(&step(&first["token"], 0)).unwrap();
    assert_eq!(text(&again), text(&first));
}

#[test]
fn duplicates_are_a_boundary_and_same_named_functions_in_other_scopes_are_not_adopted() {
    let (root, _cache, host, history, request, _) = fixture("");
    commit(
        root.path(),
        "fn target() { one(); }\nfn target() { two(); }\n",
    );
    commit(
        root.path(),
        "fn target() { restored(); }\nstruct A; impl A { fn target() { unrelated(); } }\n",
    );
    commit(
        root.path(),
        "fn target() { current(); }\nstruct A; impl A { fn target() { unrelated(); } }\n",
    );
    let request = refresh_request(&host, &request);
    let first = history.start(&request).unwrap();
    assert!(text(&first).contains("restored()"));
    assert!(
        history
            .step(&step(&first["token"], 1))
            .unwrap_err()
            .to_string()
            .contains("ambiguous")
    );
}

#[test]
fn new_sessions_expire_oldest_history_but_current_bindings_remain_available() {
    let (_root, _cache, host, history, request, _) = fixture("");
    let first = history.start(&request).unwrap();
    for _ in 0..24 {
        history.start(&request).unwrap();
    }
    assert!(
        history
            .step(&step(&first["token"], 0))
            .unwrap_err()
            .to_string()
            .contains("expired")
    );
    assert_eq!(
        host.application().snapshot().unwrap().id().as_str(),
        request.snapshot
    );
}

#[test]
fn shallow_history_is_reported_separately_from_the_beginning() {
    let (root, _cache, _host, history, request, commits) = fixture("");
    std::fs::write(
        root.path().join(".git/shallow"),
        format!("{}\n", commits[2]),
    )
    .unwrap();
    let first = history.start(&request).unwrap();
    assert!(
        history
            .step(&step(&first["token"], 1))
            .unwrap_err()
            .to_string()
            .contains("shallow")
    );
}

#[test]
fn history_requires_git_and_rejects_foreign_file_and_tracking_bindings() {
    let (root, _cache, _host, history, request, _) = fixture("");
    for field in ["file", "version", "tracking"] {
        let mut bad = serde_json::to_value(&request).unwrap();
        bad[field] = json!("0".repeat(64));
        assert!(
            history
                .start(&serde_json::from_value(bad).unwrap())
                .is_err()
        );
    }
    std::fs::rename(root.path().join(".git"), root.path().join("hidden-git")).unwrap();
    assert!(
        history
            .start(&request)
            .unwrap_err()
            .to_string()
            .contains("local Git worktree")
    );
}

#[test]
fn exact_file_and_total_source_allowances_are_inclusive_and_evict_before_session_count() {
    let (root, _cache, host, history, request, _) = fixture("");
    let prefix = "fn target() {\n/*";
    let suffix = "*/\n}\n";
    let large = format!(
        "{prefix}{}{suffix}",
        "a".repeat(2 * 1024 * 1024 - prefix.len() - suffix.len())
    );
    commit(root.path(), &large);
    commit(root.path(), "fn target() { small_current(); }\n");
    let request = refresh_request(&host, &request);
    let first = history.start(&request).unwrap();
    let mut tokens = vec![];
    for _ in 0..15 {
        tokens.push(history.start(&request).unwrap()["token"].clone());
    }
    assert!(history.step(&step(&first["token"], 0)).is_ok());
    // Accessing the first session moves it to the end; another admission evicts the next.
    let second = history.start(&request).unwrap();
    assert!(history.step(&step(&tokens[0], 0)).is_err());
    assert!(history.step(&step(&second["token"], 0)).is_ok());
    for _ in 0..16 {
        history.start(&request).unwrap();
    }
    assert!(history.step(&step(&first["token"], 0)).is_err());
}

#[test]
fn oversized_current_files_are_refused_before_history_acquisition() {
    let source = format!("fn target() {{ /*{}*/ }}", "a".repeat(2 * 1024 * 1024));
    let (_root, _cache, _host, history, request, _) = fixture(&source);
    assert!(
        history
            .start(&request)
            .unwrap_err()
            .to_string()
            .contains("2 MiB")
    );
}

#[test]
fn temporarily_deferred_analysis_can_be_retried_without_creating_a_lineage_boundary() {
    let budget = Arc::new(vizzi::admission::Admission::new(1024, 1024, 0).unwrap());
    let (root, _cache, host, history, request, commits) =
        fixture_with_budget("", Some(budget.clone()));
    commit(root.path(), "fn target() { fourth(); }\n");
    commit(root.path(), "fn target() { fifth(); }\n");
    let request = refresh_request(&host, &request);
    let first = history.start(&request).unwrap();
    let permit = budget
        .acquire(1024, vizzi::admission::Priority::Foreground)
        .unwrap();
    assert!(
        history
            .step(&step(&first["token"], 1))
            .unwrap_err()
            .to_string()
            .contains("analysis is unavailable")
    );
    drop(permit);
    assert_eq!(
        history.step(&step(&first["token"], 1)).unwrap()["commit"],
        commits[3]
    );
}

#[test]
fn file_revision_limit_is_reported_instead_of_claiming_complete_history() {
    let (root, _cache, host, history, request, _) = fixture("");
    for n in 0..65 {
        commit(
            root.path(),
            &format!("fn target() {{ unchanged(); }}\nfn other() {{ let n = {n}; }}\n"),
        );
    }
    let request = refresh_request(&host, &request);
    assert!(
        history
            .start(&request)
            .unwrap_err()
            .to_string()
            .contains("64 file revisions")
    );
}

#[test]
fn first_parent_history_does_not_enter_the_merged_branch() {
    let (root, _cache, host, history, request, commits) = fixture("");
    let branch = git(root.path(), &["symbolic-ref", "--short", "HEAD"]);
    git(root.path(), &["checkout", "-qb", "side"]);
    commit(root.path(), "fn target() { side(); }\n");
    git(root.path(), &["checkout", "-q", &branch]);
    commit(root.path(), "fn target() { mainline(); }\n");
    git(
        root.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "merge",
            "-q",
            "--no-edit",
            "-s",
            "ours",
            "side",
        ],
    );
    let request = refresh_request(&host, &request);
    assert_eq!(history.start(&request).unwrap()["commit"], commits[3]);
}

#[test]
fn git_paths_are_literal_even_when_the_filename_looks_like_a_glob() {
    let (root, _cache, host, history, request, _) = fixture("");
    git(root.path(), &["mv", "lib.rs", "[a]*.rs"]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "rename",
        ],
    );
    std::fs::write(
        root.path().join("[a]*.rs"),
        "fn target() { literal_current(); }\n",
    )
    .unwrap();
    std::fs::write(root.path().join("abc.rs"), "fn wrong() {}\n").unwrap();
    git(root.path(), &["add", "-A"]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "literal",
        ],
    );
    let request = refresh_request(&host, &request);
    let first = history.start(&request).unwrap();
    assert_eq!(first["symbol"]["source"]["path"], "[a]*.rs");
    assert!(text(&first).contains("newest()"));
}
fn step(token: &Value, index: usize) -> Step {
    serde_json::from_value(json!({"token":token,"index":index})).unwrap()
}
fn text(reply: &Value) -> String {
    reply["window"]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["text"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn traverses_function_changes_skipping_other_edits_and_revisits_frozen_commits() {
    let (root, _cache, _host, history, request, commits) = fixture("");
    let first = history.start(&request).unwrap();
    assert_eq!(first["commit"], commits[1]);
    assert!(text(&first).contains("middle()"));
    assert_eq!(first["tip"], commits[3]);
    let oldest = history.step(&step(&first["token"], 1)).unwrap();
    assert_eq!(oldest["commit"], commits[0]);
    assert!(text(&oldest).contains("old()"));
    let s = &first["symbol"]["source"];
    let retained = history
        .window(
            first["token"].as_str().unwrap(),
            first["snapshot"].as_str().unwrap(),
            s["file"].as_str().unwrap(),
            s["version"].as_str().unwrap(),
            &WindowQuery::default(),
        )
        .unwrap();
    assert!(
        retained["lines"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["text"].as_str().unwrap().contains("middle()"))
    );
    commit(root.path(), "fn target() { future(); }\n");
    let back = history.step(&step(&first["token"], 0)).unwrap();
    assert_eq!(back["commit"], first["commit"]);
    assert_eq!(back["tip"], commits[3]);
    assert!(history.step(&step(&first["token"], 2)).is_err());
    assert!(history.step(&step(&first["token"], 5)).is_err());
}

#[test]
fn saved_uncommitted_function_steps_to_head_and_window_pins_are_exact() {
    let (_root, _cache, _host, history, request, commits) = fixture("fn target() { working(); }\n");
    let first = history.start(&request).unwrap();
    assert_eq!(first["commit"], commits[3]);
    let s = &first["symbol"]["source"];
    let window = history
        .window(
            first["token"].as_str().unwrap(),
            first["snapshot"].as_str().unwrap(),
            s["file"].as_str().unwrap(),
            s["version"].as_str().unwrap(),
            &WindowQuery::default(),
        )
        .unwrap();
    assert!(
        window["lines"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["text"].as_str().unwrap().contains("newest()"))
    );
    assert!(
        history
            .window(
                first["token"].as_str().unwrap(),
                first["snapshot"].as_str().unwrap(),
                s["file"].as_str().unwrap(),
                &"0".repeat(64),
                &WindowQuery::default()
            )
            .is_err()
    );
    let mut stale = serde_json::to_value(&request).unwrap();
    stale["snapshot"] = json!("0".repeat(64));
    assert!(
        history
            .start(&serde_json::from_value(stale).unwrap())
            .is_err()
    );
}

#[test]
fn initial_history_window_contains_only_the_function_and_preserves_copy_endings() {
    let (_root, _cache, _host, history, request, _) =
        fixture("fn target() { working(); }\nfn other() {}\n");
    let answer = history.start(&request).unwrap();
    let rows = answer["window"]["lines"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["number"], 2);
    assert_eq!(rows[0]["text"], "fn target() { newest(); }");
    assert_eq!(rows[0]["ending"], "\n");
    assert_eq!(answer["window"]["outline_total"], 2);
    assert_eq!(answer["window"]["outline"].as_array().unwrap().len(), 1);
}
