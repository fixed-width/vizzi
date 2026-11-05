//! Orientation without a supplied source path or source text reads.
use std::sync::Arc;
use vizzi_core::{
    analysis::LanguageAdapter,
    discovery::{DiscoveryQuery, PAGE_SIZE},
    model::{CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance, SourceOnlyReason},
    python::PythonAdapter,
    rust::RustAnalyzer,
};

fn snapshot(files: &[(&str, &str)]) -> Snapshot {
    let workspace = WorkspaceId::derive(&[b"discovery"]);
    let documents = files
        .iter()
        .map(|(path, text)| {
            let (language, analyses) = if path.ends_with(".rs") {
                (
                    "rust",
                    vec![
                        RustAnalyzer::new()
                            .unwrap()
                            .analyze(text.as_bytes())
                            .unwrap(),
                    ],
                )
            } else if path.ends_with(".py") {
                (
                    "python",
                    vec![
                        PythonAdapter
                            .create()
                            .unwrap()
                            .analyze(text.as_bytes())
                            .unwrap(),
                    ],
                )
            } else {
                ("", vec![])
            };
            Arc::new(
                Document::capture(
                    workspace.clone(),
                    (*path).into(),
                    text.as_bytes(),
                    analyses,
                    if language.is_empty() {
                        Classification::SourceOnly {
                            language: None,
                            provenance: Provenance::Extension {
                                extension: "txt".into(),
                            },
                            reason: SourceOnlyReason::AnalysisNotRequested,
                        }
                    } else {
                        Classification::Assigned {
                            language: language.into(),
                            provenance: Provenance::Extension {
                                extension: language.into(),
                            },
                        }
                    },
                )
                .unwrap(),
            )
        })
        .collect();
    Snapshot::new(workspace, documents, CapturePolicyId::derive(&[b"scope"])).unwrap()
}

#[test]
fn a_newcomer_gets_real_areas_counts_languages_and_names_without_a_query() {
    let snapshot = snapshot(&[
        ("src/compiler/read.rs", "pub trait SourceReader {}"),
        ("src/main.rs", "fn main() {}"),
        ("tools/build.py", "class Builder: pass"),
        ("data/input.txt", "literal"),
    ]);
    let view = snapshot.discover(&DiscoveryQuery::default()).unwrap();
    assert_eq!(view.snapshot, *snapshot.id());
    assert_eq!(view.scope_files, 4);
    assert_eq!(view.source_only, 1);
    assert_eq!(
        view.entries
            .iter()
            .map(|e| e.path.as_str())
            .collect::<Vec<_>>(),
        ["data", "src", "tools"]
    );
    let src = &view.entries[1];
    assert_eq!(src.files, 2);
    assert_eq!(src.languages.get("rust"), Some(&2));
    assert_eq!(view.languages.get("rust"), Some(&2));
    assert_eq!(view.languages.get("python"), Some(&1));
    assert_eq!(view.languages.get("unassigned"), Some(&1));
    assert_eq!(
        src.bytes,
        snapshot
            .documents()
            .iter()
            .filter(|d| d.path().starts_with("src/"))
            .map(|d| d.bytes())
            .sum::<usize>()
    );
    assert!(src.examples.contains(&"SourceReader".into()));
    assert!(src.location.is_none());
    assert!(view.next.is_none());
}

#[test]
fn narrowing_an_area_preserves_directory_boundaries_and_exact_source_locations() {
    let snapshot = snapshot(&[
        ("src/a.rs", "struct Alpha;"),
        ("src/deep/b.rs", "struct Beta;"),
        ("src-other/c.rs", "struct Other;"),
    ]);
    let view = snapshot
        .discover(&DiscoveryQuery {
            path: "src".into(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(view.scope_files, 2);
    assert_eq!(view.entries.len(), 2);
    assert_eq!(view.entries[0].path, "src/deep");
    let item = &view.entries[1];
    let location = item.location.as_ref().unwrap();
    let doc = snapshot.file_by_path("src/a.rs").unwrap();
    assert_eq!(location.file, *doc.id());
    assert_eq!(location.version, *doc.version());
    assert_eq!(location.line, 1);
    assert_eq!(item.label, "a.rs");
}

#[test]
fn names_and_paths_are_searchable_without_knowing_a_file_but_body_text_is_not_claimed_as_search() {
    let snapshot = snapshot(&[
        (
            "src/a.rs",
            "struct SourceReader;\nimpl SourceReader { fn read_file() {} }\n// invisible_body_keyword",
        ),
        ("tools/reader.py", "class Reader: pass"),
    ]);
    let view = snapshot
        .discover(&DiscoveryQuery {
            search: "READ".into(),
            ..Default::default()
        })
        .unwrap();
    assert!(view.entries.iter().any(|e| e.label == "SourceReader"));
    let method = view
        .entries
        .iter()
        .find(|e| e.label == "read_file")
        .unwrap();
    assert_eq!(method.location.as_ref().unwrap().line, 2);
    assert!(view.entries.iter().any(|e| e.path == "tools/reader.py"));
    let scoped = snapshot
        .discover(&DiscoveryQuery {
            path: "tools".into(),
            search: "read".into(),
            ..Default::default()
        })
        .unwrap();
    assert!(scoped.entries.iter().all(|e| e.path.starts_with("tools/")));
    let absent = snapshot
        .discover(&DiscoveryQuery {
            search: "invisible_body_keyword".into(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(absent.matching, 0);
}

#[test]
fn directory_and_name_results_are_paged_without_hiding_continuation() {
    let files: Vec<_> = (0..(PAGE_SIZE + 2))
        .map(|i| (format!("area{i:03}/file.rs"), "struct Example;".to_owned()))
        .collect();
    let snapshot = snapshot(
        &files
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect::<Vec<_>>(),
    );
    for search in ["", "Example"] {
        let first = snapshot
            .discover(&DiscoveryQuery {
                search: search.into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(first.entries.len(), PAGE_SIZE);
        assert_eq!(first.matching, PAGE_SIZE + 2);
        assert_eq!(first.next, Some(PAGE_SIZE));
        let last = snapshot
            .discover(&DiscoveryQuery {
                search: search.into(),
                offset: first.next.unwrap(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(last.entries.len(), 2);
        assert!(last.next.is_none());
        assert_ne!(first.entries[0].path, last.entries[0].path);
    }
}

#[test]
fn empty_scopes_invalid_input_and_utf8_labels_remain_explicit_and_bounded() {
    let snapshot = snapshot(&[("src/a.rs", &format!("struct {};", "界".repeat(100)))]);
    let search = snapshot
        .discover(&DiscoveryQuery {
            search: "界".into(),
            ..Default::default()
        })
        .unwrap();
    assert!(search.entries[0].label.len() <= 128);
    assert!(search.entries[0].label.ends_with('界'));
    assert!(
        snapshot
            .discover(&DiscoveryQuery {
                path: "../src".into(),
                ..Default::default()
            })
            .is_err()
    );
    assert!(
        snapshot
            .discover(&DiscoveryQuery {
                search: "x".repeat(257),
                ..Default::default()
            })
            .is_err()
    );
    let empty = snapshot
        .discover(&DiscoveryQuery {
            path: "missing".into(),
            offset: usize::MAX,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(empty.scope_files, 0);
    assert!(empty.entries.is_empty());
    assert!(empty.next.is_none());
    for length in [4096, 4097] {
        let result = snapshot.discover(&DiscoveryQuery {
            path: "a".repeat(length),
            ..Default::default()
        });
        assert_eq!(result.is_ok(), length == 4096);
    }
    assert!(
        snapshot
            .discover(&DiscoveryQuery {
                search: "x".repeat(256),
                ..Default::default()
            })
            .is_ok()
    );
}

#[test]
fn clues_are_unique_and_limited_and_deferred_coverage_is_visible() {
    use vizzi_core::model::Coverage;
    let base = snapshot(&[
        (
            "area/a.rs",
            "struct First; struct Second; struct Third; struct Fourth;",
        ),
        ("area/b.rs", "struct First; struct Fifth;"),
        ("pending.rs", "struct Pending;"),
    ]);
    let pending = base.file_by_path("pending.rs").unwrap();
    let mut analysis = pending.analyses()[0].clone();
    analysis.coverage = Coverage::Deferred;
    analysis.facts.clear();
    analysis.coloring = None;
    let deferred = Arc::new(
        Document::capture(
            base.workspace().clone(),
            "pending.rs".into(),
            b"struct Pending;",
            vec![analysis],
            pending.classification().clone(),
        )
        .unwrap(),
    );
    let mut documents: Vec<_> = base
        .documents()
        .iter()
        .filter(|d| d.path() != "pending.rs")
        .cloned()
        .collect();
    documents.push(deferred);
    let snapshot =
        Snapshot::new(base.workspace().clone(), documents, base.policy().clone()).unwrap();
    let view = snapshot.discover(&DiscoveryQuery::default()).unwrap();
    assert_eq!(view.deferred, 1);
    assert_eq!(view.entries[0].examples, ["First", "Second", "Third"]);
    let scoped = snapshot
        .discover(&DiscoveryQuery {
            path: "area".into(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(scoped.deferred, 0);
}
