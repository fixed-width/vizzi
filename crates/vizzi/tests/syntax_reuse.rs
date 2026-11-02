//! Retained mutable syntax remains independent of immutable published observations.

use std::{path::Path, sync::Arc};
use vizzi::{
    cache::Cache,
    engine::Engine,
    source::{CaptureScope, Directory},
    syntax_cache::SyntaxCache,
};
use vizzi_core::{analysis::Languages, policy::PolicyConfig};

fn engine(root: &Path, cache: &Path) -> Engine {
    Engine::new(
        Directory::with_policy(
            root,
            PolicyConfig {
                c_family_roots: vec![".".into()],
                ..Default::default()
            },
            Arc::new(Languages::builtin().unwrap()),
        )
        .unwrap(),
        Cache::new(cache.into()),
        4,
    )
    .unwrap()
}

#[test]
fn repeated_targeted_edits_reuse_trees_across_workers_and_match_clean_captures() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let pool = Arc::new(SyntaxCache::new(8 * 1024 * 1024, 1024 * 1024, 8).unwrap());
    let files = [
        ("a.rs", "fn rust() {}"),
        ("b.py", "def python(): pass\n"),
        ("c.c", "int c(void) { return 1; }"),
        ("d.cpp", "class Cpp {};"),
    ];
    for (path, text) in files {
        std::fs::write(root.path().join(path), text).unwrap();
    }
    let live = engine(root.path(), cache.path()).with_syntax_cache(pool.clone());
    let original = live.refresh(None, &|| false).unwrap().unwrap().snapshot;
    assert_eq!(
        pool.stats().idle_entries,
        0,
        "cold indexing must not fill a hot-file cache"
    );
    let hints = CaptureScope::Paths(files.iter().map(|(path, _)| (*path).into()).collect());
    let mut previous = original.clone();
    for index in 1..=3 {
        for (path, text) in files {
            let marker = if path.ends_with(".py") { "#" } else { "//" };
            std::fs::write(
                root.path().join(path),
                format!("{marker} π {index}\r\n{text}"),
            )
            .unwrap();
        }
        let result = live
            .refresh_scoped(Some(&previous), &hints, &|| false)
            .unwrap()
            .unwrap();
        assert!(!result.measurements.reconciled);
        assert_eq!(result.measurements.parsed, 4);
        assert_eq!(
            result.measurements.tree_reuses,
            if index == 1 { 0 } else { 4 }
        );
        assert_eq!(
            result.snapshot.id(),
            engine(root.path(), cache.path())
                .refresh(None, &|| false)
                .unwrap()
                .unwrap()
                .snapshot
                .id()
        );
        assert_eq!(pool.stats().active_leases, 0);
        assert_eq!(pool.stats().idle_entries, 4);
        let languages = Languages::builtin().unwrap();
        for file in result.snapshot.documents() {
            let language = &file.analyses()[0].context.language;
            let fresh = languages
                .select(language)
                .unwrap()
                .create()
                .unwrap()
                .analyze(file.source().unwrap().as_bytes())
                .unwrap();
            assert_eq!(file.analyses(), [fresh]);
        }
        previous = result.snapshot;
    }
    assert_eq!(
        original.file_by_path("a.rs").unwrap().source(),
        Some("fn rust() {}")
    );
    std::fs::remove_file(root.path().join("c.c")).unwrap();
    std::fs::rename(root.path().join("a.rs"), root.path().join("a.py")).unwrap();
    std::fs::write(root.path().join("b.py"), [255]).unwrap();
    let result = live.refresh(Some(&previous), &|| false).unwrap().unwrap();
    assert_eq!(
        result.snapshot.id(),
        engine(root.path(), cache.path())
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
    assert!(
        pool.stats().idle_entries <= 2,
        "deleted and reassigned paths must lose retained state"
    );
}
