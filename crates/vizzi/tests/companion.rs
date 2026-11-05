//! Native source evidence, persistence and editor handoff over module interfaces.
use std::sync::{Arc, Mutex};
use vizzi::{
    cache::Cache,
    companion::*,
    editor::*,
    engine::Engine,
    host::{Host, HostOptions},
    source::Directory,
};
fn frozen(engine: Engine) -> Host {
    Host::start_with_options(
        engine,
        HostOptions {
            watch: false,
            reconcile_interval: std::time::Duration::from_secs(3600),
            idle_maintenance: false,
        },
    )
    .unwrap()
}
fn setup() -> (tempfile::TempDir, tempfile::TempDir, Arc<Host>) {
    setup_with("fn entry() {}\nfn reader() {}\n")
}
fn setup_with(text: &str) -> (tempfile::TempDir, tempfile::TempDir, Arc<Host>) {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::write(root.path().join("src/a.rs"), text).unwrap();
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().join("analysis")),
        1,
    )
    .unwrap();
    let host = Arc::new(frozen(engine));
    (root, cache, host)
}
struct Recording(Mutex<Vec<EditorLocation>>);
impl Editor for Recording {
    fn open(&self, location: &EditorLocation) -> Result<(), EditorError> {
        self.0.lock().unwrap().push(location.clone());
        Ok(())
    }
    fn name(&self) -> &str {
        "recording"
    }
}
#[test]
fn source_evidence_is_frozen_and_only_matching_working_content_is_handed_to_the_editor() {
    let (root, cache, host) = setup();
    let editor = Arc::new(Recording(Mutex::new(vec![])));
    let app = Companion::new(
        host.clone(),
        Some(cache.path().join("investigation.json")),
        editor.clone(),
    )
    .unwrap();
    let target = app.lookup("src/a.rs").unwrap();
    let id = app
        .pin(PinEvidence {
            revision: 0,
            snapshot: target.snapshot,
            file: target.file,
            version: target.version,
            line: 1,
            title: "Entry".into(),
        })
        .unwrap();
    app.open(id).unwrap();
    assert_eq!(editor.0.lock().unwrap()[0].line, 1);
    let saved = app.view().board.evidence[0].clone();
    std::fs::write(root.path().join("src/a.rs"), "fn changed() {}\n").unwrap();
    assert!(app.open(id).is_err());
    assert_eq!(editor.0.lock().unwrap().len(), 1);
    assert_eq!(app.view().board.evidence[0], saved);
    assert!(saved.source.excerpt.contains("entry"));
}
#[test]
fn stale_requests_and_failed_persistence_do_not_replace_the_investigation() {
    let (_root, cache, host) = setup();
    let editor = Arc::new(Recording(Mutex::new(vec![])));
    let path = cache.path().join("board.json");
    let app = Companion::new(host.clone(), Some(path.clone()), editor.clone()).unwrap();
    app.edit(EditInvestigation::Question {
        revision: 0,
        text: "Question".into(),
    })
    .unwrap();
    assert!(
        app.edit(EditInvestigation::Question {
            revision: 0,
            text: "Obsolete".into()
        })
        .is_err()
    );
    assert_eq!(app.view().board.question, "Question");
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(
        app.edit(EditInvestigation::Question {
            revision: 1,
            text: "Not saved".into()
        })
        .is_err()
    );
    assert_eq!(app.view().board.question, "Question");
}
#[test]
fn persistence_roundtrips_and_invalid_pin_versions_or_lines_fail() {
    let (_root, cache, host) = setup();
    let editor = Arc::new(Recording(Mutex::new(vec![])));
    let path = cache.path().join("board.json");
    {
        let app = Companion::new(host.clone(), Some(path.clone()), editor.clone()).unwrap();
        let mut target = app.lookup("src/a.rs").unwrap();
        target.version = "wrong".into();
        assert!(
            app.pin(PinEvidence {
                revision: 0,
                snapshot: target.snapshot.clone(),
                file: target.file.clone(),
                version: target.version,
                line: 1,
                title: "bad".into()
            })
            .is_err()
        );
        let target = app.lookup("src/a.rs").unwrap();
        assert!(
            app.pin(PinEvidence {
                revision: 0,
                snapshot: target.snapshot.clone(),
                file: target.file.clone(),
                version: target.version.clone(),
                line: 999,
                title: "bad".into()
            })
            .is_err()
        );
        app.pin(PinEvidence {
            revision: 0,
            snapshot: target.snapshot,
            file: target.file,
            version: target.version,
            line: 1,
            title: "Entry".into(),
        })
        .unwrap();
    }
    let loaded = Companion::new(host, Some(path), editor).unwrap();
    assert_eq!(loaded.view().board.evidence.len(), 1);
}
#[cfg(unix)]
#[test]
fn symlink_replacement_cannot_be_opened_as_saved_evidence() {
    let (root, cache, host) = setup();
    let editor = Arc::new(Recording(Mutex::new(vec![])));
    let app = Companion::new(host, Some(cache.path().join("board.json")), editor.clone()).unwrap();
    let t = app.lookup("src/a.rs").unwrap();
    let id = app
        .pin(PinEvidence {
            revision: 0,
            snapshot: t.snapshot,
            file: t.file,
            version: t.version,
            line: 1,
            title: "a".into(),
        })
        .unwrap();
    std::fs::write(
        cache.path().join("other.rs"),
        "fn entry() {}\nfn reader() {}\n",
    )
    .unwrap();
    std::fs::remove_file(root.path().join("src/a.rs")).unwrap();
    std::os::unix::fs::symlink(cache.path().join("other.rs"), root.path().join("src/a.rs"))
        .unwrap();
    assert!(app.open(id).is_err());
    assert!(editor.0.lock().unwrap().is_empty());
}
#[test]
fn vscode_uses_literal_arguments_without_shell_interpretation() {
    let target = EditorLocation {
        path: "/tmp/a file;$(echo nope).rs".into(),
        line: 12,
    };
    let args = VsCode::arguments(&target);
    assert_eq!(
        args,
        vec![
            "--reuse-window",
            "--goto",
            "/tmp/a file;$(echo nope).rs:12:1"
        ]
    );
}

#[test]
fn captured_excerpts_report_omitted_lines_and_respect_utf8_byte_limits() {
    let text = format!("{}\n{}\n", "a\n".repeat(8), "界".repeat(3000));
    let (_root, _cache, host) = setup_with(&text);
    let app = Companion::new(host, None, Arc::new(Recording(Mutex::new(vec![])))).unwrap();
    let target = app.lookup("src/a.rs").unwrap();
    for (revision, line) in [(0, 1), (1, 10)] {
        let id = app
            .pin(PinEvidence {
                revision,
                snapshot: target.snapshot.clone(),
                file: target.file.clone(),
                version: target.version.clone(),
                line,
                title: "Excerpt".into(),
            })
            .unwrap();
        let evidence = app
            .view()
            .board
            .evidence
            .into_iter()
            .find(|e| e.id == id)
            .unwrap();
        assert!(evidence.source.truncated);
        assert!(evidence.source.excerpt.len() <= vizzi_core::investigation::EXCERPT_BYTES);
        if line == 10 {
            assert_eq!(evidence.source.excerpt, "界".repeat(1365));
            assert_eq!(evidence.source.end_line, 10);
        } else {
            assert_eq!(evidence.source.excerpt, "a\n".repeat(6));
            assert_eq!(evidence.source.end_line, 6);
        }
    }
}

#[cfg(unix)]
#[test]
fn vscode_adapter_reports_recorded_acknowledgement_refusal_and_missing_executable() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let executable = temp.path().join("record-editor");
    let target = EditorLocation {
        path: temp.path().join("a file;$(nope).rs"),
        line: 7,
    };
    std::fs::write(
        &executable,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$0.args\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let editor = VsCode::new(executable.clone());
    assert_eq!(editor.name(), "VS Code");
    editor.open(&target).unwrap();
    let recorded = std::fs::read_to_string(temp.path().join("record-editor.args")).unwrap();
    assert_eq!(
        recorded,
        format!("--reuse-window\n--goto\n{}:7:1\n", target.path.display())
    );
    std::fs::write(&executable, "#!/bin/sh\nexit 1\n").unwrap();
    assert!(editor.open(&target).is_err());
    assert!(
        VsCode::new(temp.path().join("missing"))
            .open(&target)
            .is_err()
    );
}

#[test]
fn index_freshness_is_separate_from_saved_evidence_and_editor_disk_verification() {
    let (root, cache, host) = setup();
    let path = cache.path().join("board.json");
    let editor = Arc::new(Recording(Mutex::new(vec![])));
    let app = Companion::new(host.clone(), Some(path.clone()), editor.clone()).unwrap();
    let target = app.lookup("src/a.rs").unwrap();
    let id = app
        .pin(PinEvidence {
            revision: 0,
            snapshot: target.snapshot,
            file: target.file,
            version: target.version,
            line: 1,
            title: "Entry".into(),
        })
        .unwrap();
    let saved = app.view().board;
    let states = |app: &Companion| serde_json::to_value(app.view()).unwrap()["freshness"].clone();
    assert_eq!(states(&app), serde_json::json!([[id, "current"]]));
    let original = std::fs::read(root.path().join("src/a.rs")).unwrap();
    std::fs::write(root.path().join("src/a.rs"), "fn changed() {}\n").unwrap();
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().join("changed")),
        1,
    )
    .unwrap();
    let changed = Arc::new(frozen(engine));
    let app = Companion::new(changed, Some(path.clone()), editor.clone()).unwrap();
    assert_eq!(states(&app), serde_json::json!([[id, "changed"]]));
    std::fs::write(root.path().join("src/a.rs"), original).unwrap();
    assert!(
        app.open(id).is_err(),
        "An older disk copy cannot bypass a differing current index"
    );
    assert!(editor.0.lock().unwrap().is_empty());
    assert_eq!(app.view().board, saved);
    std::fs::remove_file(root.path().join("src/a.rs")).unwrap();
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().join("missing")),
        1,
    )
    .unwrap();
    let app = Companion::new(Arc::new(frozen(engine)), Some(path.clone()), editor.clone()).unwrap();
    assert_eq!(states(&app), serde_json::json!([[id, "missing"]]));
    assert!(app.open(id).is_err());
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().join("idle")),
        1,
    )
    .unwrap();
    let app = Companion::new(Arc::new(Host::prepare(engine).unwrap()), Some(path), editor).unwrap();
    assert_eq!(states(&app), serde_json::json!([[id, "unchecked"]]));
    assert!(app.open(id).is_err());
}

#[test]
fn line_boundaries_are_exact_and_imported_anchors_cannot_bypass_verification() {
    let (root, cache, host) = setup();
    let path = cache.path().join("board.json");
    let editor = Arc::new(Recording(Mutex::new(vec![])));
    let app = Companion::new(host.clone(), Some(path.clone()), editor.clone()).unwrap();
    let target = app.lookup("src/a.rs").unwrap();
    let pin = |line, revision| PinEvidence {
        revision,
        snapshot: target.snapshot.clone(),
        file: target.file.clone(),
        version: target.version.clone(),
        line,
        title: "Boundary".into(),
    };
    assert!(app.pin(pin(0, 0)).is_err());
    assert!(app.pin(pin(target.lines + 1, 0)).is_err());
    let id = app.pin(pin(target.lines, 0)).unwrap();
    assert!(!app.view().board.evidence[0].source.truncated);
    app.open(id).unwrap();
    let saved = app.view().board;
    std::fs::write(
        root.path().join("src/alias.rs"),
        std::fs::read(root.path().join("src/a.rs")).unwrap(),
    )
    .unwrap();
    for field in ["line", "path"] {
        let mut value = serde_json::to_value(&saved).unwrap();
        if field == "line" {
            value["evidence"][0]["source"]["line"] = (target.lines + 1).into();
            value["evidence"][0]["source"]["end_line"] = (target.lines + 1).into();
        } else {
            value["evidence"][0]["source"]["path"] = "src/alias.rs".into();
        }
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let imported = Companion::new(host.clone(), Some(path.clone()), editor.clone()).unwrap();
        assert!(imported.open(id).is_err(), "{field}");
        assert_eq!(editor.0.lock().unwrap().len(), 1);
    }
}

#[test]
fn editor_handoff_verifies_complete_large_working_files() {
    let text = format!("//{}\n", "x".repeat(1_100_000));
    let (_root, _cache, host) = setup_with(&text);
    let editor = Arc::new(Recording(Mutex::new(vec![])));
    let app = Companion::new(host, None, editor.clone()).unwrap();
    let target = app.lookup("src/a.rs").unwrap();
    let id = app
        .pin(PinEvidence {
            revision: 0,
            snapshot: target.snapshot,
            file: target.file,
            version: target.version,
            line: 1,
            title: "Large source".into(),
        })
        .unwrap();
    app.open(id).unwrap();
    assert_eq!(editor.0.lock().unwrap().len(), 1);
}

#[test]
fn an_exact_byte_budget_excerpt_is_accepted_without_an_extra_newline() {
    let text = format!("{}\nnext\n", "x".repeat(4096));
    let (_root, _cache, host) = setup_with(&text);
    let app = Companion::new(host, None, Arc::new(Recording(Mutex::new(vec![])))).unwrap();
    let target = app.lookup("src/a.rs").unwrap();
    app.pin(PinEvidence {
        revision: 0,
        snapshot: target.snapshot,
        file: target.file,
        version: target.version,
        line: 1,
        title: "Full excerpt".into(),
    })
    .unwrap();
    let saved = app.view().board;
    assert_eq!(saved.evidence[0].source.excerpt, "x".repeat(4096));
    assert_eq!(saved.evidence[0].source.end_line, 1);
    assert!(saved.evidence[0].source.truncated);
}

#[tokio::test]
async fn investigation_http_preserves_state_on_invalid_stale_and_oversized_edits() {
    use axum::{body::Body, http::Request};
    use serde_json::json;
    use tower::ServiceExt;
    use vizzi::{review_application::ReviewApplication, review_source::CaptureLimits, server::*};
    let (_root, _cache, host) = setup();
    let app = Arc::new(
        Companion::new(host.clone(), None, Arc::new(Recording(Mutex::new(vec![])))).unwrap(),
    );
    let reviews =
        Arc::new(ReviewApplication::start(host.clone(), CaptureLimits::default()).unwrap());
    let router = router_with_companion(host, reviews, app.clone());
    let discovery = diagnostic(router.clone(), json!(1), "/api/discovery")
        .await
        .unwrap();
    assert_eq!(discovery.status, 200);
    let discovery: serde_json::Value = serde_json::from_str(&discovery.body).unwrap();
    assert_eq!(discovery["scope_files"], 1);
    assert_eq!(discovery["entries"][0]["path"], "src");
    assert_eq!(
        diagnostic(router.clone(), json!(1), "/api/discovery?path=../escape")
            .await
            .unwrap()
            .status,
        400
    );
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/investigation")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    for (body, status) in [
        (
            json!({"action":"question","revision":0,"text":"Saved question"}),
            200,
        ),
        (
            json!({"action":"question","revision":0,"text":"Obsolete"}),
            409,
        ),
        (
            json!({"action":"question","revision":1,"text":"x".repeat(2049)}),
            400,
        ),
        (
            json!({"action":"question","revision":1,"text":"Wrong schema","command":"code"}),
            422,
        ),
        (
            json!({"action":"question","revision":1,"text":"x".repeat(20000)}),
            413,
        ),
    ] {
        let response = diagnostic_request(
            router.clone(),
            json!(1),
            "POST",
            "/api/investigation",
            Some(body),
        )
        .await
        .unwrap();
        assert_eq!(response.status, status, "{}", response.body);
    }
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/investigation")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 415);
    let response = diagnostic_request(
        router,
        json!(1),
        "POST",
        "/api/investigation/evidence/1/open",
        Some(json!({"executable":"code"})),
    )
    .await
    .unwrap();
    assert_eq!(response.status, 422);
    assert_eq!(app.view().board.question, "Saved question");
    assert_eq!(app.view().board.revision, 1);
}
