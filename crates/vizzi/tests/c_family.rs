//! Scoped C/C++ inputs, cache isolation, and incremental equivalence.

use std::{path::Path, sync::Arc};
use vizzi::{
    cache::Cache,
    engine::Engine,
    source::{CaptureScope, Directory},
};
use vizzi_core::{
    analysis::Languages,
    policy::{Assignment, Classification, FamilyLanguage, PolicyConfig, SourceOnlyReason},
};

fn engine(root: &Path, cache: &Path, config: PolicyConfig) -> Engine {
    Engine::new(
        Directory::with_policy(root, config, Arc::new(Languages::builtin().unwrap())).unwrap(),
        Cache::new(cache.into()),
        4,
    )
    .unwrap()
}

#[test]
fn explicit_headers_and_cpp_in_c_are_analyzed_while_cuda_ambiguous_headers_and_lpc_stay_out() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    for (path, bytes) in [
        ("a.c", b"struct C { int n; };".as_slice()),
        ("foreign.c", b"namespace Foreign { class X {}; }"),
        ("include/as_c.h", b"typedef int Value;"),
        (
            "include/as_cpp.h",
            b"class Header { public: int read() { return 1; } };",
        ),
        ("include/shared.h", b"ambiguous"),
        ("cpp/kernel.cu", b"__global__ void kernel() {}"),
        ("cpp/device.cuh", b"__device__ void device();"),
        ("lpc/room.c", b"nomask public void create() {}"),
        ("skip.lpc", b"LPC"),
        ("bad.c", &[255]),
    ] {
        let target = root.path().join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, bytes).unwrap();
    }
    let config = PolicyConfig {
        c_family_roots: vec![".".into()],
        exclusions: vec!["lpc".into()],
        assignments: vec![
            Assignment {
                scope: "foreign.c".into(),
                language: FamilyLanguage::Cpp,
            },
            Assignment {
                scope: "include/as_c.h".into(),
                language: FamilyLanguage::C,
            },
            Assignment {
                scope: "include/as_cpp.h".into(),
                language: FamilyLanguage::Cpp,
            },
            Assignment {
                scope: "cpp".into(),
                language: FamilyLanguage::Cpp,
            },
        ],
    };
    let engine = engine(root.path(), cache.path(), config);
    let captured = engine.refresh(None, &|| false).unwrap().unwrap();
    assert_eq!(captured.measurements.files, 8);
    assert_eq!(captured.measurements.analyzed, 5);
    assert_eq!(captured.measurements.source_only, 3);
    let file = |path: &str| {
        captured
            .snapshot
            .documents()
            .iter()
            .find(|file| file.path() == path)
            .unwrap()
    };
    assert_eq!(file("a.c").analyses()[0].context.language, "c");
    assert_eq!(file("foreign.c").analyses()[0].context.language, "cpp");
    assert_eq!(file("foreign.c").analyses()[0].facts[0].name, "Foreign");
    assert_eq!(file("include/as_c.h").analyses()[0].facts[0].name, "Value");
    assert_eq!(
        file("include/as_cpp.h").analyses()[0].context.language,
        "cpp"
    );
    assert!(matches!(
        file("include/shared.h").classification(),
        Classification::SourceOnly {
            reason: SourceOnlyReason::LanguageUnassigned,
            ..
        }
    ));
    for path in ["cpp/kernel.cu", "cpp/device.cuh"] {
        assert!(file(path).analyses().is_empty());
        assert!(matches!(
            file(path).classification(),
            Classification::SourceOnly {
                reason: SourceOnlyReason::DialectUnsupported,
                ..
            }
        ));
    }
    assert_eq!(
        file("bad.c").analyses()[0].coverage,
        vizzi_core::model::Coverage::Unavailable
    );
    let baseline = engine
        .refresh(Some(&captured.snapshot), &|| false)
        .unwrap()
        .unwrap();
    std::fs::write(
        root.path().join("include/as_cpp.h"),
        "class Header { public: int read() { return 2; } };",
    )
    .unwrap();
    let edited = engine
        .refresh_scoped(
            Some(&baseline.snapshot),
            &CaptureScope::Paths(["include/as_cpp.h".into()].into()),
            &|| false,
        )
        .unwrap()
        .unwrap();
    assert_eq!(edited.measurements.read_files, 1);
    assert_eq!(edited.measurements.parsed, 1);
    assert_eq!(edited.measurements.retained, 7);
    assert!(!edited.measurements.reconciled);
    assert_eq!(
        edited.snapshot.id(),
        engine
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
}

#[test]
fn identical_c_cpp_bytes_have_distinct_cache_contexts_and_cross_language_renames_reparse() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let text = "int same(void) { return 0; }";
    std::fs::write(root.path().join("one.c"), text).unwrap();
    std::fs::write(root.path().join("two.cpp"), text).unwrap();
    let config = PolicyConfig {
        c_family_roots: vec![".".into()],
        ..Default::default()
    };
    let engine = engine(root.path(), cache.path(), config.clone());
    let cold = engine.refresh(None, &|| false).unwrap().unwrap();
    assert_eq!(cold.measurements.parsed, 2);
    assert_eq!(cold.measurements.cache_hits, 0);
    assert_ne!(
        cold.snapshot.documents()[0].analyses()[0].context.id,
        cold.snapshot.documents()[1].analyses()[0].context.id
    );
    let warm = engine.refresh(None, &|| false).unwrap().unwrap();
    assert_eq!(warm.measurements.cache_hits, 2);
    assert_eq!(warm.measurements.parsed, 0);
    assert_eq!(cold.snapshot.id(), warm.snapshot.id());
    std::fs::write(root.path().join("one.c"), format!("// shifted\n{text}")).unwrap();
    let edited = engine
        .refresh_scoped(
            Some(&warm.snapshot),
            &CaptureScope::Paths(["one.c".into()].into()),
            &|| false,
        )
        .unwrap()
        .unwrap();
    let before = &warm.snapshot.documents()[0];
    let after = &edited.snapshot.documents()[0];
    assert_eq!(
        before.entity_id(&before.analyses()[0], &before.analyses()[0].facts[0]),
        after.entity_id(&after.analyses()[0], &after.analyses()[0].facts[0])
    );
    std::fs::rename(root.path().join("one.c"), root.path().join("one.cpp")).unwrap();
    let renamed = engine
        .refresh_scoped(
            Some(&edited.snapshot),
            &CaptureScope::Paths(["one.c".into(), "one.cpp".into()].into()),
            &|| false,
        )
        .unwrap()
        .unwrap();
    assert_eq!(renamed.measurements.parsed, 1);
    assert_eq!(renamed.measurements.read_files, 1);
    assert_eq!(
        renamed.snapshot.documents()[0].analyses()[0]
            .context
            .language,
        "cpp"
    );
    let fresh_cache = tempfile::tempdir().unwrap();
    let clean = Engine::new(
        Directory::with_policy(root.path(), config, Arc::new(Languages::builtin().unwrap()))
            .unwrap(),
        Cache::new(fresh_cache.path().into()),
        1,
    )
    .unwrap()
    .refresh(None, &|| false)
    .unwrap()
    .unwrap();
    assert_eq!(renamed.snapshot.id(), clean.snapshot.id());
    std::fs::remove_file(root.path().join("one.cpp")).unwrap();
    let removed = engine
        .refresh_scoped(
            Some(&renamed.snapshot),
            &CaptureScope::Paths(["one.cpp".into()].into()),
            &|| false,
        )
        .unwrap()
        .unwrap();
    assert_eq!(removed.measurements.read_files, 0);
    assert_eq!(removed.snapshot.documents().len(), 1);
    assert_eq!(
        removed.snapshot.id(),
        engine
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
}
