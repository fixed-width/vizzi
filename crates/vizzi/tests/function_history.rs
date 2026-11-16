//! Immutable per-function history through production Git and analysis interfaces.
use serde_json::{Value, json};
use std::{process::Command, sync::Arc};
use vizzi::{
    cache::Cache,
    engine::Engine,
    function_history::{FunctionHistory, Request, Step},
    host::{Host, HostOptions},
    source::Directory,
};
use vizzi_core::{projection::WindowQuery, structure::Structure};

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
    host.request(vizzi::source::CaptureScope::Reconcile);
    let at = std::time::Instant::now();
    while host.application().snapshot().unwrap().id().as_str() == previous.snapshot {
        assert!(at.elapsed() < std::time::Duration::from_secs(10));
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let snapshot = host.application().snapshot().unwrap();
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
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().join("analysis")),
        1,
    )
    .unwrap();
    let engine = if let Some(budget) = budget {
        engine.with_admission(budget)
    } else {
        engine
    };
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
    let (_root, _cache, _host, history, request, _) = fixture("");
    let answer = history.start(&request).unwrap();
    let rows = answer["window"]["lines"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["text"], "fn target() { middle(); }");
    assert_eq!(rows[0]["ending"], "\n");
    assert_eq!(answer["window"]["outline_total"], 2);
    assert_eq!(answer["window"]["outline"].as_array().unwrap().len(), 1);
}
