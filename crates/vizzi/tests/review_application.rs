//! Native review application lifecycle and bounded immutable query contracts.

use std::{path::Path, sync::Arc, time::Duration};
use vizzi::{
    cache::Cache,
    engine::Engine,
    host::{Host, HostOptions},
    review_application::{CreateComparison, ReviewApiError, ReviewApplication, ReviewPageQuery},
    review_source::CaptureLimits,
    source::Directory,
};
use vizzi_core::{comparison::Side, projection::WindowQuery};

struct Repo {
    root: tempfile::TempDir,
    cache: tempfile::TempDir,
}
impl Repo {
    fn new() -> Self {
        let repo = Self {
            root: tempfile::tempdir().unwrap(),
            cache: tempfile::tempdir().unwrap(),
        };
        repo.git(&["init", "--quiet", "--initial-branch=main", "--template="]);
        repo
    }
    fn path(&self) -> &Path {
        self.root.path()
    }
    fn git(&self, args: &[&str]) -> Vec<u8> {
        let result = std::process::Command::new("git")
            .arg("-C")
            .arg(self.path())
            .args([
                "-c",
                "core.fsmonitor=false",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
            ])
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        result.stdout
    }
    fn write(&self, name: &str, text: &str) {
        let path = self.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    fn commit(&self) {
        self.git(&["add", "--all"]);
        self.git(&["commit", "--quiet", "-m", "fixture"]);
    }
    fn app(&self) -> ReviewApplication {
        ReviewApplication::start(self.host(), CaptureLimits::default()).unwrap()
    }
    fn host(&self) -> Arc<Host> {
        let engine = Engine::new(
            Directory::open(self.path()).unwrap(),
            Cache::new(self.cache.path().into()),
            1,
        )
        .unwrap();
        Arc::new(
            Host::start_with_options(
                engine,
                HostOptions {
                    watch: false,
                    reconcile_interval: Duration::from_secs(3600),
                    idle_maintenance: false,
                },
            )
            .unwrap(),
        )
    }
}

#[tokio::test]
async fn failures_and_invalid_requests_keep_active_state_and_latest_intent_wins() {
    let repo = Repo::new();
    repo.write("a.rs", "fn base() {}\n");
    repo.commit();
    repo.write("a.rs", "fn work() {}\n");
    let app = repo.app();
    let mut initial = app.subscribe();
    let before = initial.next().await.unwrap();
    assert!(!before.status.pending);
    for request in [
        CreateComparison {
            provider: "other".into(),
            revision: "HEAD".into(),
            scope: None,
        },
        CreateComparison {
            provider: "git".into(),
            revision: "".into(),
            scope: None,
        },
        CreateComparison {
            provider: "git".into(),
            revision: "HEAD".into(),
            scope: Some("../outside".into()),
        },
    ] {
        assert!(matches!(
            app.create(request),
            Err(ReviewApiError::Invalid(_))
        ));
    }
    assert_eq!(app.status().ticket, None);
    app.create(CreateComparison {
        provider: "git".into(),
        revision: "HEAD".into(),
        scope: None,
    })
    .unwrap();
    let active = ready(&app).await.active.unwrap();
    assert!(matches!(
        app.refresh("wrong"),
        Err(ReviewApiError::Review(vizzi::review::ReviewError::Stale))
    ));
    app.create(CreateComparison {
        provider: "git".into(),
        revision: "does-not-exist".into(),
        scope: None,
    })
    .unwrap();
    let failed = ready(&app).await;
    assert!(failed.error.is_some());
    assert_eq!(failed.error.as_ref().unwrap().code, "capture_failed");
    assert_eq!(failed.active.as_ref().unwrap().token, active.token);
    app.create(CreateComparison {
        provider: "git".into(),
        revision: "missing".into(),
        scope: None,
    })
    .unwrap();
    let accepted = app
        .create(CreateComparison {
            provider: "git".into(),
            revision: "HEAD".into(),
            scope: None,
        })
        .unwrap();
    let final_state = ready(&app).await;
    assert_eq!(
        final_state.ticket.as_deref(),
        Some(accepted.ticket.as_str())
    );
    assert!(final_state.error.is_none());
    let token = final_state.active.unwrap().token;
    let held = app.query(&token).unwrap();
    let row = &held.files(&ReviewPageQuery::default()).unwrap().files[0];
    assert!(app.close("wrong").is_err());
    app.close(&token).unwrap();
    assert!(app.query(&token).is_err());
    assert_eq!(
        held.window(
            &row.file,
            Side::Working,
            row.working_version.as_deref().unwrap(),
            &WindowQuery::default()
        )
        .unwrap()
        .window
        .lines[0]
            .text,
        "fn work() {}"
    );
    app.shutdown();
    assert!(matches!(app.refresh(&token), Err(ReviewApiError::Closed)));
    assert!(matches!(app.query(&token), Err(ReviewApiError::Closed)));
    assert!(app.subscribe().next().await.is_none());
}

#[tokio::test]
async fn inventory_pages_are_bounded_and_filters_preserve_full_scope_counts() {
    let repo = Repo::new();
    for i in 0..205 {
        repo.write(&format!("f{i:03}.txt"), "before\n");
    }
    repo.commit();
    for i in 0..205 {
        repo.write(&format!("f{i:03}.txt"), "after\n");
    }
    let long = (0..6).map(|_| "a".repeat(90)).collect::<Vec<_>>().join("/") + "/added.txt";
    repo.write(&long, "added\n");
    repo.write("unchanged.md", "context\n");
    repo.git(&["add", "unchanged.md"]);
    repo.git(&["commit", "--quiet", "-m", "context only", "unchanged.md"]);
    let app = repo.app();
    app.create(CreateComparison {
        provider: "git".into(),
        revision: "HEAD".into(),
        scope: None,
    })
    .unwrap();
    let token = ready(&app).await.active.unwrap().token;
    assert!(!token.is_empty());
    let query = app.query(&token).unwrap();
    let first = query
        .files(&ReviewPageQuery {
            count: Some(usize::MAX),
            ..ReviewPageQuery::default()
        })
        .unwrap();
    assert_eq!(first.files.len(), 200);
    assert_eq!(first.total, 206);
    assert_eq!(first.next, Some(200));
    assert_eq!(first.remaining, 6);
    assert_eq!(first.totals.paths, 207);
    assert_eq!(first.totals.unchanged, 1);
    assert_eq!(first.totals.modified, 205);
    assert_eq!(first.totals.added, 1);
    let second = query
        .files(&ReviewPageQuery {
            offset: first.next,
            ..ReviewPageQuery::default()
        })
        .unwrap();
    assert_eq!(second.files.len(), 6);
    assert_eq!(second.next, None);
    assert_eq!(second.remaining, 0);
    let beyond = second
        .files
        .iter()
        .find(|row| row.path == "f204.txt")
        .unwrap();
    let selected = query
        .files(&ReviewPageQuery {
            file: Some(beyond.file.clone()),
            ..ReviewPageQuery::default()
        })
        .unwrap();
    assert_eq!(selected.files.len(), 200);
    assert_eq!(selected.selected.unwrap().path, "f204.txt");
    let only = query
        .files(&ReviewPageQuery {
            filter: Some("ADDED".into()),
            count: Some(0),
            ..ReviewPageQuery::default()
        })
        .unwrap();
    assert_eq!(only.total, 1);
    assert_eq!(only.files.len(), 1);
    assert!(only.files[0].path_truncated);
    assert!(only.files[0].path.len() <= 512);
    assert_eq!(only.totals.paths, 207);
    let context = query
        .files(&ReviewPageQuery {
            filter: Some("unchanged".into()),
            unchanged: Some(true),
            ..ReviewPageQuery::default()
        })
        .unwrap();
    assert_eq!(context.total, 1);
    assert_eq!(context.files[0].change, "unchanged");
    assert!(
        query
            .files(&ReviewPageQuery {
                filter: Some("x".repeat(257)),
                ..ReviewPageQuery::default()
            })
            .is_err()
    );
    assert!(
        query
            .files(&ReviewPageQuery {
                filter: Some("x".repeat(256)),
                ..ReviewPageQuery::default()
            })
            .is_ok()
    );
    assert!(
        query
            .files(&ReviewPageQuery {
                offset: Some(usize::MAX),
                ..ReviewPageQuery::default()
            })
            .unwrap()
            .files
            .is_empty()
    );
}

#[tokio::test]
async fn diff_pages_and_source_windows_require_exact_side_versions() {
    use vizzi::review_application::ReviewDiffQuery;
    let repo = Repo::new();
    let base = (0..1601).map(|i| format!("line {i}\n")).collect::<String>();
    repo.write("many.txt", &base);
    repo.commit();
    let working = (0..1601)
        .map(|i| {
            if i % 8 == 0 {
                format!("edit {i}\n")
            } else {
                format!("line {i}\n")
            }
        })
        .collect::<String>();
    repo.write("many.txt", &working);
    repo.write("new.md", "added\n");
    let app = repo.app();
    app.create(CreateComparison {
        provider: "git".into(),
        revision: "HEAD".into(),
        scope: None,
    })
    .unwrap();
    let token = ready(&app).await.active.unwrap().token;
    let query = app.query(&token).unwrap();
    let list = query.files(&ReviewPageQuery::default()).unwrap();
    let row = list.files.iter().find(|f| f.path == "many.txt").unwrap();
    let diff = query
        .diff(
            &row.file,
            row.base_version.as_deref(),
            row.working_version.as_deref(),
            &ReviewDiffQuery::default(),
        )
        .unwrap();
    assert_eq!(diff.hunk_total, 201);
    assert_eq!(diff.hunks.len(), 200);
    assert_eq!(diff.remaining_hunks, 1);
    assert_eq!(diff.next_hunk, Some(200));
    assert_eq!(diff.edit_total, 201);
    assert_eq!(diff.edits.len(), 200);
    assert_eq!(diff.next_edit, Some(200));
    assert_eq!(diff.remaining_edits, 1);
    let last = query
        .diff(
            &row.file,
            row.base_version.as_deref(),
            row.working_version.as_deref(),
            &ReviewDiffQuery {
                hunk_start: Some(200),
                edit_start: Some(200),
                ..ReviewDiffQuery::default()
            },
        )
        .unwrap();
    assert_eq!(last.hunks.len(), 1);
    assert_eq!(last.edits.len(), 1);
    assert_eq!(last.next_hunk, None);
    assert_eq!(last.next_edit, None);
    assert_eq!(last.remaining_hunks, 0);
    assert_eq!(last.remaining_edits, 0);
    assert!(matches!(
        query.diff(
            &row.file,
            None,
            row.working_version.as_deref(),
            &ReviewDiffQuery::default()
        ),
        Err(ReviewApiError::Version)
    ));
    let added = list.files.iter().find(|row| row.path == "new.md").unwrap();
    assert!(
        query
            .diff(
                &added.file,
                None,
                added.working_version.as_deref(),
                &ReviewDiffQuery::default()
            )
            .is_ok()
    );
    assert!(matches!(
        query.window(
            &added.file,
            Side::Base,
            added.working_version.as_deref().unwrap(),
            &WindowQuery::default()
        ),
        Err(ReviewApiError::NotFound)
    ));
    let window = query
        .window(
            &row.file,
            Side::Working,
            row.working_version.as_deref().unwrap(),
            &WindowQuery {
                line_count: Some(usize::MAX),
                outline_count: Some(usize::MAX),
                ..WindowQuery::default()
            },
        )
        .unwrap();
    assert!(window.window.lines.len() <= 96);
    assert!(
        window
            .window
            .lines
            .iter()
            .map(|line| line.text.len())
            .sum::<usize>()
            <= 32768
    );
}

#[tokio::test]
async fn http_and_diagnostic_share_creation_queries_errors_and_event_boundaries() {
    use futures_util::StreamExt;
    use serde_json::{Value, json};
    use tower::ServiceExt;
    use vizzi::server::{diagnostic, diagnostic_request, router_with_reviews};
    let repo = Repo::new();
    repo.write("a.rs", "fn base() {}\n");
    repo.commit();
    repo.write("a.rs", "fn work() {}\n");
    let host = repo.host();
    let app = Arc::new(ReviewApplication::start(host.clone(), CaptureLimits::default()).unwrap());
    let router = router_with_reviews(host.application(), app.clone());
    let response = diagnostic_request(
        router.clone(),
        json!(1),
        "POST",
        "/api/comparisons",
        Some(json!({"provider":"git","revision":"HEAD"})),
    )
    .await
    .unwrap();
    assert_eq!(response.status, 202);
    let accepted: Value = serde_json::from_str(&response.body).unwrap();
    assert!(accepted["ticket"].is_string());
    let token = ready(&app).await.active.unwrap().token;
    assert_eq!(accepted["ticket"], token);
    let response = diagnostic(
        router.clone(),
        json!(2),
        &format!("/api/comparisons/{token}/files?count=1"),
    )
    .await
    .unwrap();
    assert_eq!(response.status, 200);
    let files: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(files["token"], token);
    assert_eq!(files["origin"], token);
    assert!(files["comparison"].is_string());
    let file = files["files"][0]["file"].as_str().unwrap();
    let version = files["files"][0]["working_version"].as_str().unwrap();
    let base_version = files["files"][0]["base_version"].as_str().unwrap();
    let selected = diagnostic(
        router.clone(),
        json!(21),
        &format!("/api/comparisons/{token}/files?file={file}&filter=does-not-match"),
    )
    .await
    .unwrap();
    assert_eq!(selected.status, 200);
    let selected: Value = serde_json::from_str(&selected.body).unwrap();
    assert_eq!(selected["total"], 0);
    assert_eq!(selected["files"], json!([]));
    assert_eq!(selected["selected"]["file"], file);
    assert_eq!(selected["selected"]["working_version"], version);
    let diff=diagnostic(router.clone(),json!(20),&format!("/api/comparisons/{token}/diff?file={file}&base_version={base_version}&working_version={version}")).await.unwrap();
    assert_eq!(diff.status, 200);
    let diff: Value = serde_json::from_str(&diff.body).unwrap();
    assert_eq!(diff["token"], token);
    assert_eq!(diff["hunk_total"], 1);
    assert_eq!(diff["hunks"].as_array().unwrap().len(), 1);
    let uri = format!("/api/comparisons/{token}/window?file={file}&side=working&version={version}");
    let window = diagnostic(router.clone(), json!(3), &uri).await.unwrap();
    assert_eq!(window.status, 200);
    let window: Value = serde_json::from_str(&window.body).unwrap();
    assert_eq!(window["token"], token);
    assert_eq!(window["side"], "working");
    assert_eq!(window["window"]["lines"][0]["text"], "fn work() {}");
    assert_eq!(
        diagnostic(
            router.clone(),
            json!(4),
            &uri.replace("side=working", "side=latest")
        )
        .await
        .unwrap()
        .status,
        400
    );
    assert_eq!(
        diagnostic(router.clone(), json!(5), &uri.replace(version, "wrong"))
            .await
            .unwrap()
            .status,
        409
    );
    assert_eq!(
        diagnostic(router.clone(), json!(6), "/api/comparisons/expired/files")
            .await
            .unwrap()
            .status,
        409
    );
    assert_eq!(
        diagnostic(router.clone(), json!(7), "/api/comparisons/events")
            .await
            .unwrap()
            .status,
        400
    );
    let events = router
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/comparisons/events")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(events.headers()["cache-control"], "no-store");
    let mut stream = events.into_body().into_data_stream();
    let first = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(String::from_utf8_lossy(&first).contains("event: comparison"));
    let bad = router
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/comparisons")
                .header("content-type", "text/plain")
                .body(axum::body::Body::from("{} "))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad.status(), 415);
    let closed = diagnostic_request(
        router.clone(),
        json!(8),
        "DELETE",
        &format!("/api/comparisons/{token}"),
        Some(json!({})),
    )
    .await
    .unwrap();
    assert_eq!(closed.status, 204);
    assert_eq!(
        diagnostic(router, json!(9), &uri).await.unwrap().status,
        409
    );
    app.shutdown();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), stream.next())
            .await
            .unwrap()
            .is_none()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn inventory_describes_literal_targets_binary_data_and_unassessed_presence() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let repo = Repo::new();
    repo.write("a.rs", "fn base() {}\n");
    repo.write("sparse.txt", "present\n");
    repo.write("deleted.txt", "removed\n");
    symlink("a.rs", repo.path().join("link")).unwrap();
    repo.commit();
    repo.git(&["update-index", "--skip-worktree", "sparse.txt"]);
    std::fs::remove_file(repo.path().join("sparse.txt")).unwrap();
    std::fs::remove_file(repo.path().join("deleted.txt")).unwrap();
    std::fs::remove_file(repo.path().join("link")).unwrap();
    symlink("deleted.txt", repo.path().join("link")).unwrap();
    std::fs::write(repo.path().join("binary"), b"\0\xff").unwrap();
    std::fs::set_permissions(
        repo.path().join("a.rs"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let app = repo.app();
    app.create(CreateComparison {
        provider: "git".into(),
        revision: "HEAD".into(),
        scope: None,
    })
    .unwrap();
    let token = ready(&app).await.active.unwrap().token;
    let query = app.query(&token).unwrap();
    let rows = query.files(&ReviewPageQuery::default()).unwrap();
    assert_eq!(
        (
            rows.totals.added,
            rows.totals.removed,
            rows.totals.modified,
            rows.totals.unassessed,
            rows.totals.unavailable
        ),
        (1, 1, 2, 1, 1)
    );
    let link = rows.files.iter().find(|row| row.path == "link").unwrap();
    assert_eq!(link.base.as_ref().unwrap().kind, Some("symlink"));
    assert_eq!(link.base.as_ref().unwrap().literal.as_deref(), Some("a.rs"));
    assert_eq!(
        link.working.as_ref().unwrap().literal.as_deref(),
        Some("deleted.txt")
    );
    let sparse = rows
        .files
        .iter()
        .find(|row| row.path == "sparse.txt")
        .unwrap();
    assert_eq!(sparse.change, "unassessed");
    assert_eq!(sparse.working.as_ref().unwrap().unavailable, Some("sparse"));
    let binary = rows.files.iter().find(|row| row.path == "binary").unwrap();
    assert_eq!(binary.working.as_ref().unwrap().bytes, Some(2));
    assert!(matches!(
        query.diff(
            &binary.file,
            None,
            binary.working_version.as_deref(),
            &vizzi::review_application::ReviewDiffQuery::default()
        ),
        Err(ReviewApiError::Diff(_))
    ));
    let mode = rows.files.iter().find(|row| row.path == "a.rs").unwrap();
    assert_eq!(mode.working.as_ref().unwrap().executable, Some(true));
}

#[tokio::test]
async fn review_scope_is_literal_and_metadata_limits_do_not_mutate_capture_scope() {
    let repo = Repo::new();
    repo.write("src/a.rs", "fn base() {}\n");
    repo.write("src-other/a.rs", "fn outside() {}\n");
    repo.commit();
    repo.write("src/a.rs", "fn changed() {}\n");
    repo.write("src-other/a.rs", "fn changed_outside() {}\n");
    let app = repo.app();
    app.create(CreateComparison {
        provider: "git".into(),
        revision: "HEAD".into(),
        scope: Some("src".into()),
    })
    .unwrap();
    let state = ready(&app).await;
    let metadata = state.active.unwrap();
    assert_eq!(metadata.scope, "src");
    let query = app.query(&metadata.token).unwrap();
    let all = query
        .files(&ReviewPageQuery {
            unchanged: Some(true),
            ..ReviewPageQuery::default()
        })
        .unwrap();
    assert_eq!(all.totals.paths, 1);
    assert_eq!(all.files[0].path, "src/a.rs");
    let filtered = query
        .files(&ReviewPageQuery {
            filter: Some("nothing".into()),
            ..ReviewPageQuery::default()
        })
        .unwrap();
    assert_eq!(filtered.total, 0);
    assert_eq!(filtered.totals.paths, 1);
    assert!(matches!(
        query.window("unknown", Side::Base, "unknown", &WindowQuery::default()),
        Err(ReviewApiError::NotFound)
    ));
    let mut subscription = app.subscribe();
    let first = subscription.next().await.unwrap();
    assert_eq!(first.status.active.as_ref().unwrap().token, metadata.token);
    let later = app.subscribe().next().await.unwrap();
    assert_eq!(later.stream, first.stream);
    app.close(&metadata.token).unwrap();
    let closed = subscription.next().await.unwrap();
    assert!(closed.status.active.is_none());
    assert!(closed.revision.parse::<u64>().unwrap() > first.revision.parse::<u64>().unwrap());
}

#[tokio::test]
async fn dropping_the_application_closes_subscriptions_without_retaining_capture_worker() {
    let repo = Repo::new();
    repo.write("a.rs", "fn base() {}\n");
    repo.commit();
    repo.write("a.rs", "fn work() {}\n");
    let app = repo.app();
    app.create(CreateComparison {
        provider: "git".into(),
        revision: "HEAD".into(),
        scope: None,
    })
    .unwrap();
    let token = ready(&app).await.active.unwrap().token;
    let query = app.query(&token).unwrap();
    let row = &query.files(&ReviewPageQuery::default()).unwrap().files[0];
    let mut updates = app.subscribe();
    assert!(updates.next().await.is_some());
    drop(app);
    assert!(
        tokio::time::timeout(Duration::from_secs(2), updates.next())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        query
            .window(
                &row.file,
                Side::Working,
                row.working_version.as_deref().unwrap(),
                &WindowQuery::default()
            )
            .unwrap()
            .window
            .lines[0]
            .text,
        "fn work() {}"
    );
}

#[tokio::test]
async fn request_lengths_and_capture_intervals_are_explicit_not_silent_truncation() {
    let repo = Repo::new();
    repo.write("a.rs", "fn base() {}\n");
    repo.commit();
    let app = repo.app();
    for revision in ["x".repeat(257), "bad\0revision".into()] {
        assert!(matches!(
            app.create(CreateComparison {
                provider: "git".into(),
                revision,
                scope: None
            }),
            Err(ReviewApiError::Invalid(_))
        ));
    }
    assert!(matches!(
        app.create(CreateComparison {
            provider: "git".into(),
            revision: "HEAD".into(),
            scope: Some("s".repeat(4097))
        }),
        Err(ReviewApiError::Invalid(_))
    ));
    let accepted = app
        .create(CreateComparison {
            provider: "git".into(),
            revision: "x".repeat(256),
            scope: None,
        })
        .unwrap();
    assert_eq!(
        ready(&app).await.ticket.as_deref(),
        Some(accepted.ticket.as_str())
    );
    assert!(app.status().error.is_some());
    app.create(CreateComparison {
        provider: "git".into(),
        revision: "HEAD".into(),
        scope: Some("s".repeat(4096)),
    })
    .unwrap();
    assert!(ready(&app).await.error.is_none());
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    app.create(CreateComparison {
        provider: "git".into(),
        revision: "HEAD".into(),
        scope: None,
    })
    .unwrap();
    let metadata = ready(&app).await.active.unwrap();
    let finished = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    assert!(metadata.working_started_ms >= started);
    assert!(metadata.working_finished_ms >= metadata.working_started_ms);
    assert!(metadata.working_finished_ms <= finished);
}

struct VerboseAdapter {
    description: String,
    diagnostic: String,
}
impl vizzi_core::analysis::LanguageAdapter for VerboseAdapter {
    fn context(&self) -> vizzi_core::model::Context {
        vizzi_core::model::Context {
            language: "rust".into(),
            description: self.description.clone(),
            id: vizzi_core::model::ContextId::derive(&[
                self.description.as_bytes(),
                self.diagnostic.as_bytes(),
            ]),
        }
    }
    fn create(
        &self,
    ) -> Result<Box<dyn vizzi_core::analysis::Analyzer>, vizzi_core::model::ModelError> {
        Ok(Box::new(VerboseParser {
            context: self.context(),
            diagnostic: self.diagnostic.clone(),
            inner: vizzi_core::rust::RustAnalyzer::new()?,
        }))
    }
}
struct VerboseParser {
    context: vizzi_core::model::Context,
    diagnostic: String,
    inner: vizzi_core::rust::RustAnalyzer,
}
impl vizzi_core::analysis::Analyzer for VerboseParser {
    fn analyze(
        &mut self,
        bytes: &[u8],
    ) -> Result<vizzi_core::model::Analysis, vizzi_core::model::ModelError> {
        let mut result = self.inner.analyze(bytes)?;
        result.context = self.context.clone();
        result.diagnostics = vec![self.diagnostic.clone(); 5];
        Ok(result)
    }
}

#[tokio::test]
async fn source_context_descriptors_and_diagnostics_are_bounded_with_explicit_hidden_counts() {
    for (description, diagnostic) in [
        ("x".repeat(600), "message".into()),
        ("short".into(), "界".repeat(200)),
    ] {
        let repo = Repo::new();
        repo.write("a.rs", "fn base() {}\n");
        repo.commit();
        repo.write("a.rs", "fn work() {}\n");
        let languages = vizzi_core::analysis::Languages::new(vec![Arc::new(VerboseAdapter {
            description,
            diagnostic,
        })])
        .unwrap();
        let engine = Engine::new(
            Directory::with_languages(repo.path(), Arc::new(languages)).unwrap(),
            Cache::new(repo.cache.path().into()),
            1,
        )
        .unwrap();
        let host = Arc::new(
            Host::start_with_options(
                engine,
                HostOptions {
                    watch: false,
                    reconcile_interval: Duration::from_secs(3600),
                    idle_maintenance: false,
                },
            )
            .unwrap(),
        );
        let app = ReviewApplication::start(host, CaptureLimits::default()).unwrap();
        app.create(CreateComparison {
            provider: "git".into(),
            revision: "HEAD".into(),
            scope: None,
        })
        .unwrap();
        let token = ready(&app).await.active.unwrap().token;
        let query = app.query(&token).unwrap();
        let rows = query.files(&ReviewPageQuery::default()).unwrap();
        let row = &rows.files[0];
        let result = query
            .window(
                &row.file,
                Side::Working,
                row.working_version.as_deref().unwrap(),
                &WindowQuery::default(),
            )
            .unwrap();
        assert_eq!(result.contexts.len(), 1);
        assert_eq!(result.hidden_contexts, 0);
        let context = &result.contexts[0];
        assert!(context.description.len() <= 512);
        assert_eq!(context.diagnostics.len(), 3);
        assert_eq!(context.hidden_diagnostics, 2);
        assert!(context.diagnostics.iter().all(|text| text.len() <= 512));
        assert!(context.truncated);
        assert_eq!(context.declarations, 1);
    }
}
async fn ready(app: &ReviewApplication) -> vizzi::review_application::ReviewStatus {
    let mut updates = app.subscribe();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let state = updates.next().await.unwrap().status.clone();
            if !state.pending {
                return state;
            }
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn refresh_origin_survives_working_updates_but_new_creation_gets_a_new_origin() {
    let repo = Repo::new();
    repo.write("a.rs", "fn base() {}\n");
    repo.commit();
    let app = repo.app();
    let create = || CreateComparison {
        provider: "git".into(),
        revision: "HEAD".into(),
        scope: None,
    };
    app.create(create()).unwrap();
    let initial = ready(&app).await.active.unwrap();
    assert_eq!(initial.origin, initial.token);
    repo.write("a.rs", "fn next() {}\n");
    app.refresh(&initial.token).unwrap();
    let refreshed = ready(&app).await.active.unwrap();
    assert_eq!(refreshed.origin, initial.origin);
    assert_ne!(refreshed.token, initial.token);
    app.refresh(&refreshed.token).unwrap();
    let same_contents = ready(&app).await.active.unwrap();
    assert_eq!(same_contents.origin, initial.origin);
    assert_eq!(same_contents.comparison, refreshed.comparison);
    app.create(create()).unwrap();
    let created = ready(&app).await.active.unwrap();
    assert_eq!(created.origin, created.token);
    assert_ne!(created.origin, initial.origin);
    assert_eq!(created.comparison, same_contents.comparison);
}

#[tokio::test]
async fn selected_inventory_rows_resolve_fresh_versions_outside_the_projected_page_and_filter() {
    let repo = Repo::new();
    for name in ["a.txt", "z.txt", "context.txt"] {
        repo.write(name, "before\n");
    }
    repo.commit();
    repo.write("a.txt", "after\n");
    repo.write("z.txt", "selected\n");
    let app = repo.app();
    app.create(CreateComparison {
        provider: "git".into(),
        revision: "HEAD".into(),
        scope: None,
    })
    .unwrap();
    let initial = ready(&app).await.active.unwrap();
    let selected = {
        let query = app.query(&initial.token).unwrap();
        let files = query.files(&ReviewPageQuery::default()).unwrap();
        assert!(files.selected.is_none());
        files
            .files
            .into_iter()
            .find(|row| row.path == "z.txt")
            .unwrap()
    };
    repo.write("z.txt", "before\n");
    app.refresh(&initial.token).unwrap();
    let fresh = ready(&app).await.active.unwrap();
    let query = app.query(&fresh.token).unwrap();
    let result = query
        .files(&ReviewPageQuery {
            file: Some(selected.file.clone()),
            filter: Some("a.txt".into()),
            count: Some(1),
            ..ReviewPageQuery::default()
        })
        .unwrap();
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].path, "a.txt");
    assert_eq!(result.total, 1);
    assert_eq!(result.totals.unchanged, 2);
    let current = result.selected.unwrap();
    assert_eq!(current.file, selected.file);
    assert_eq!(current.path, "z.txt");
    assert_eq!(current.change, "unchanged");
    assert_eq!(current.base_version, selected.base_version);
    assert_ne!(current.working_version, selected.working_version);
    assert_eq!(
        query
            .window(
                &current.file,
                Side::Working,
                current.working_version.as_deref().unwrap(),
                &WindowQuery::default()
            )
            .unwrap()
            .window
            .lines[0]
            .text,
        "before"
    );
    let missing = query
        .files(&ReviewPageQuery {
            file: Some("absent".into()),
            ..ReviewPageQuery::default()
        })
        .unwrap();
    assert!(missing.selected.is_none());
    assert_eq!(missing.total, 1);
}

#[tokio::test]
async fn create_refresh_and_queries_keep_the_frozen_baseline_and_source_anchors() {
    let repo = Repo::new();
    repo.write("a.rs", "fn base() {}\n");
    repo.write("context.py", "def helper(): pass\n");
    repo.commit();
    let pin = String::from_utf8(repo.git(&["rev-parse", "HEAD"]))
        .unwrap()
        .trim()
        .to_owned();
    repo.write("a.rs", "fn one() {}\n");
    let app = repo.app();
    app.create(CreateComparison {
        provider: "git".into(),
        revision: "HEAD".into(),
        scope: None,
    })
    .unwrap();
    let first = ready(&app).await;
    assert!(first.error.is_none());
    let metadata = first.active.unwrap();
    assert_eq!(metadata.resolved, pin);
    let query = app.query(&metadata.token).unwrap();
    let files = query.files(&ReviewPageQuery::default()).unwrap();
    assert_eq!(files.total, 1);
    assert_eq!(files.files[0].path, "a.rs");
    let file = &files.files[0];
    let window = query
        .window(
            &file.file,
            Side::Working,
            file.working_version.as_deref().unwrap(),
            &WindowQuery::default(),
        )
        .unwrap();
    assert_eq!(window.window.lines[0].text, "fn one() {}");
    assert_eq!(window.token, metadata.token);
    assert!(matches!(
        query.window(
            &file.file,
            Side::Base,
            file.working_version.as_deref().unwrap(),
            &WindowQuery::default()
        ),
        Err(ReviewApiError::Version)
    ));
    repo.write("a.rs", "fn committed() {}\n");
    repo.commit();
    repo.write("a.rs", "fn two() {}\n");
    app.refresh(&metadata.token).unwrap();
    let second = ready(&app).await.active.unwrap();
    assert_eq!(second.resolved, pin);
    assert_ne!(second.token, metadata.token);
    assert!(matches!(
        app.query(&metadata.token),
        Err(ReviewApiError::Review(vizzi::review::ReviewError::Stale))
    ));
    assert_eq!(
        query
            .window(
                &file.file,
                Side::Working,
                file.working_version.as_deref().unwrap(),
                &WindowQuery::default()
            )
            .unwrap()
            .window
            .lines[0]
            .text,
        "fn one() {}"
    );
    let current = app.query(&second.token).unwrap();
    let current_file = &current.files(&ReviewPageQuery::default()).unwrap().files[0];
    assert_eq!(
        current
            .window(
                &current_file.file,
                Side::Base,
                current_file.base_version.as_deref().unwrap(),
                &WindowQuery::default()
            )
            .unwrap()
            .window
            .lines[0]
            .text,
        "fn base() {}"
    );
}
