//! Source-policy changes must reconcile before retaining unread documents.

use std::sync::Arc;
use vizzi::{
    cache::Cache,
    engine::Engine,
    source::{CaptureScope, Directory},
};
use vizzi_core::{
    analysis::Languages,
    policy::{Assignment, FamilyLanguage, PolicyConfig},
};

fn engine(root: &std::path::Path, cache: &std::path::Path, config: PolicyConfig) -> Engine {
    Engine::new(
        Directory::with_policy(root, config, Arc::new(Languages::builtin().unwrap())).unwrap(),
        Cache::new(cache.into()),
        2,
    )
    .unwrap()
}

#[test]
fn expanding_an_empty_snapshot_discovers_unhinted_headers() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.h"), "struct A;").unwrap();
    let initial = engine(root.path(), cache.path(), PolicyConfig::default())
        .refresh(None, &|| false)
        .unwrap()
        .unwrap();
    assert!(initial.snapshot.documents().is_empty());
    let expanded_engine = engine(
        root.path(),
        cache.path(),
        PolicyConfig {
            c_family_roots: vec![".".into()],
            ..Default::default()
        },
    );
    let expanded = expanded_engine
        .refresh_scoped(
            Some(&initial.snapshot),
            &CaptureScope::Paths(["absent.rs".into()].into()),
            &|| false,
        )
        .unwrap()
        .unwrap();
    assert!(expanded.measurements.reconciled);
    assert_eq!(expanded.snapshot.documents().len(), 1);
    assert_eq!(expanded.measurements.parsed, 0);
    assert_eq!(expanded.snapshot.documents()[0].source(), Some("struct A;"));
    assert!(expanded.snapshot.documents()[0].analyses().is_empty());
    assert_ne!(initial.snapshot.id(), expanded.snapshot.id());
    assert_eq!(
        expanded.snapshot.id(),
        expanded_engine
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
}

#[test]
fn changed_policy_keeps_raw_cache_and_file_identity_but_updates_pinned_metadata() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn rust() {}").unwrap();
    std::fs::write(root.path().join("b.h"), "struct Header;").unwrap();
    let config = PolicyConfig {
        c_family_roots: vec![".".into()],
        ..Default::default()
    };
    let first_engine = engine(root.path(), cache.path(), config.clone());
    let first = first_engine.refresh(None, &|| false).unwrap().unwrap();
    let mut changed_config = config;
    changed_config.assignments.push(Assignment {
        scope: "b.h".into(),
        language: FamilyLanguage::Cpp,
    });
    let changed_engine = engine(root.path(), cache.path(), changed_config);
    let changed = changed_engine
        .refresh_scoped(
            Some(&first.snapshot),
            &CaptureScope::Paths(["a.rs".into()].into()),
            &|| false,
        )
        .unwrap()
        .unwrap();
    assert!(changed.measurements.reconciled);
    assert_eq!(changed.measurements.files, 2);
    assert_eq!(changed.measurements.parsed, 0);
    assert!(Arc::ptr_eq(
        &first.snapshot.documents()[0],
        &changed.snapshot.documents()[0]
    ));
    assert_eq!(
        first.snapshot.documents()[1].id(),
        changed.snapshot.documents()[1].id()
    );
    assert_ne!(
        first.snapshot.documents()[1].classification(),
        changed.snapshot.documents()[1].classification()
    );
    assert_ne!(
        first.snapshot.documents()[1].version(),
        changed.snapshot.documents()[1].version()
    );
    let view = first.snapshot.view(&Default::default());
    assert_eq!(view.schema, 2);
    assert_eq!(view.analyzed, 1);
    assert_eq!(view.source_only, 1);
    let source_view = view.source.unwrap();
    assert_eq!(source_view.outlines[0].mode, "raw_syntax");
    assert_eq!(source_view.outlines[0].configuration_coverage, "unknown");
    assert_eq!(
        source_view.encoding,
        vizzi_core::model::SourceEncoding::Utf8
    );
    std::fs::write(root.path().join("b.h"), "struct Updated;").unwrap();
    let edited = changed_engine
        .refresh_scoped(
            Some(&changed.snapshot),
            &CaptureScope::Paths(["b.h".into()].into()),
            &|| false,
        )
        .unwrap()
        .unwrap();
    assert!(!edited.measurements.reconciled);
    assert_eq!(edited.measurements.read_files, 1);
    assert_eq!(edited.measurements.retained, 1);
    assert_eq!(
        edited.snapshot.id(),
        changed_engine
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
    assert_eq!(
        first.snapshot.documents()[1].source(),
        Some("struct Header;")
    );
}

#[test]
fn reducing_scope_discovers_removal_without_losing_raw_analysis() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn rust() {}").unwrap();
    std::fs::write(root.path().join("b.h"), "struct Header;").unwrap();
    let initial = engine(
        root.path(),
        cache.path(),
        PolicyConfig {
            c_family_roots: vec![".".into()],
            ..Default::default()
        },
    )
    .refresh(None, &|| false)
    .unwrap()
    .unwrap();
    let reduced_engine = engine(root.path(), cache.path(), PolicyConfig::default());
    let reduced = reduced_engine
        .refresh_scoped(
            Some(&initial.snapshot),
            &CaptureScope::Paths(["a.rs".into()].into()),
            &|| false,
        )
        .unwrap()
        .unwrap();
    assert!(reduced.measurements.reconciled);
    assert_eq!(reduced.measurements.files, 1);
    assert_eq!(
        reduced.snapshot.documents()[0].id(),
        initial.snapshot.documents()[0].id()
    );
    assert_eq!(
        reduced.snapshot.id(),
        reduced_engine
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
}

#[test]
fn cli_scopes_are_repeatable_and_invalid_assignments_fail_at_startup() {
    use std::process::Command;
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("include")).unwrap();
    for (path, text) in [
        ("a.rs", "fn rust() {}"),
        ("include/a.h", "struct Header;"),
        ("include/skip.h", "skip"),
        ("b.c++", "class Example {};"),
    ] {
        std::fs::write(root.path().join(path), text).unwrap();
    }
    let run = |options: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_vizzi"))
            .arg("--root")
            .arg(root.path())
            .arg("--cache")
            .arg(cache.path())
            .arg("--once")
            .args(options)
            .output()
            .unwrap()
    };
    let output = run(&[
        "--c-family-root",
        "include",
        "--c-family-root",
        "b.c++",
        "--exclude-source",
        "include/skip.h",
        "--syntax-language",
        "include=cpp",
        "--workers",
        "1",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let status: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status["schema"], 1);
    assert_eq!(status["measurements"]["files"], 3);
    assert_eq!(status["measurements"]["analyzed"], 1);
    assert_eq!(status["measurements"]["source_only"], 2);
    for options in [
        vec![
            "--c-family-root",
            "include",
            "--syntax-language",
            "include=c",
        ],
        vec!["--port", "0"],
    ] {
        let output = run(&options);
        assert!(
            output.status.success(),
            "{options:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for option in ["--help", "-h"] {
        let output = Command::new(env!("CARGO_BIN_EXE_vizzi"))
            .arg(option)
            .output()
            .unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("--c-family-root"));
        assert!(help.contains("--syntax-language"));
    }
    assert!(!run(&["--port", "70000"]).status.success());
    for options in [
        vec!["--c-family-root"],
        vec!["--syntax-language", "include=cpp"],
        vec![
            "--c-family-root",
            "include",
            "--syntax-language",
            "include=c",
            "--syntax-language",
            "include=cpp",
        ],
        vec!["--c-family-root", "../outside"],
        vec!["--syntax-language", "include=unknown"],
    ] {
        let output = run(&options);
        assert!(!output.status.success(), "{options:?}");
        assert!(output.stdout.is_empty());
    }
}
