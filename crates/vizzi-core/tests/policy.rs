//! Portable source selection and immutable policy provenance.

use vizzi_core::{
    model::{Document, Snapshot, WorkspaceId},
    policy::{
        Assignment, Classification, FamilyLanguage, PolicyConfig, Provenance, SourceOnlyReason,
        SourcePolicy,
    },
};

fn policy(config: PolicyConfig) -> SourcePolicy {
    SourcePolicy::new(config, ["rust", "python", "c", "cpp"].map(String::from)).unwrap()
}

#[test]
fn scopes_and_explicit_exclusions_preserve_unsupported_and_ambiguous_inputs() {
    let policy = policy(PolicyConfig {
        c_family_roots: vec!["src".into()],
        exclusions: vec!["src/lpc".into()],
        assignments: vec![Assignment {
            scope: "src/headers".into(),
            language: FamilyLanguage::Cpp,
        }],
    });
    for path in [
        "src2/a.c",
        "src/lpc/a.c",
        "src/a.lpc",
        "target/a.rs",
        ".git/a.py",
        "node_modules/b.rs",
        "notes.txt",
    ] {
        assert!(policy.classify(path).unwrap().is_none(), "{path}");
    }
    for (path, language) in [
        ("a.rs", "rust"),
        ("a.py", "python"),
        ("src/a.c", "c"),
        ("src/a.C", "cpp"),
        ("src/a.c++", "cpp"),
        ("src/a.h++", "cpp"),
        ("drivers/target/a.rs", "rust"),
    ] {
        assert_eq!(
            policy.classify(path).unwrap().unwrap().assigned_language(),
            Some(language),
            "{path}"
        );
    }
    let header = policy.classify("src/shared.h").unwrap().unwrap();
    assert!(matches!(
        header,
        Classification::SourceOnly {
            reason: SourceOnlyReason::LanguageUnassigned,
            language: None,
            ..
        }
    ));
    let assigned = policy.classify("src/headers/shared.h").unwrap().unwrap();
    assert_eq!(assigned.assigned_language(), Some("cpp"));
    assert!(
        matches!(assigned, Classification::Assigned { provenance: Provenance::ExplicitScope { scope }, .. } if scope == "src/headers")
    );
    assert!(matches!(
        policy.classify("src/headers/device.cuh").unwrap().unwrap(),
        Classification::SourceOnly {
            reason: SourceOnlyReason::DialectUnsupported,
            ..
        }
    ));
    assert!(matches!(
        policy.classify("src/a.inc").unwrap().unwrap(),
        Classification::SourceOnly {
            reason: SourceOnlyReason::FragmentUnassigned,
            ..
        }
    ));
}

#[test]
fn normalized_rules_have_stable_identity_and_conflicts_and_bad_paths_fail() {
    let a = PolicyConfig {
        c_family_roots: vec!["src/".into(), "./src".into(), "src/child".into()],
        exclusions: vec!["src/lpc/".into()],
        assignments: vec![Assignment {
            scope: "src/headers".into(),
            language: FamilyLanguage::C,
        }],
    };
    let b = PolicyConfig {
        c_family_roots: vec!["src".into()],
        exclusions: vec!["src/lpc".into()],
        assignments: vec![Assignment {
            scope: "./src//headers/".into(),
            language: FamilyLanguage::C,
        }],
    };
    assert_eq!(policy(a).id(), policy(b.clone()).id());
    let mut changed = b.clone();
    changed.assignments[0].language = FamilyLanguage::Cpp;
    assert_ne!(policy(b.clone()).id(), policy(changed).id());
    let mut conflict = b;
    conflict.assignments.push(Assignment {
        scope: "src/headers/".into(),
        language: FamilyLanguage::Cpp,
    });
    assert!(SourcePolicy::new(conflict, ["c".into()]).is_err());
    for scope in ["", "/src", "../src", "src/../x", "C:/src", "src\\x"] {
        assert!(
            SourcePolicy::new(
                PolicyConfig {
                    c_family_roots: vec![scope.into()],
                    ..Default::default()
                },
                []
            )
            .is_err(),
            "{scope}"
        );
    }
    let p = policy(PolicyConfig::default());
    for path in [
        "", "/a.rs", "../a.rs", "a/./b.rs", "a//b.rs", "a\\b.rs", "C:/a.rs",
    ] {
        assert!(p.classify(path).is_err(), "{path}");
    }
    assert!(
        policy(PolicyConfig {
            c_family_roots: vec![".".into()],
            ..Default::default()
        })
        .classify("a.c")
        .unwrap()
        .is_some()
    );
}

#[test]
fn missing_analyzers_keep_source_and_change_policy_identity() {
    let config = PolicyConfig {
        c_family_roots: vec![".".into()],
        ..Default::default()
    };
    let limited = SourcePolicy::new(config.clone(), ["rust".into()]).unwrap();
    let complete = policy(config);
    assert_ne!(limited.id(), complete.id());
    assert!(
        matches!(limited.classify("a.c++").unwrap().unwrap(), Classification::SourceOnly { reason: SourceOnlyReason::AnalyzerUnavailable, language: Some(language), .. } if language == "cpp")
    );
    assert_eq!(
        complete
            .classify("a.c++")
            .unwrap()
            .unwrap()
            .assigned_language(),
        Some("cpp")
    );
}

#[test]
fn specific_assignments_override_parents_and_redundant_rules_do_not_change_identity() {
    let config = PolicyConfig {
        c_family_roots: vec![".".into()],
        assignments: vec![
            Assignment {
                scope: ".".into(),
                language: FamilyLanguage::C,
            },
            Assignment {
                scope: "src".into(),
                language: FamilyLanguage::Cpp,
            },
            Assignment {
                scope: "src/c".into(),
                language: FamilyLanguage::C,
            },
        ],
        ..Default::default()
    };
    let original = policy(config.clone());
    for (path, language) in [
        ("a.h", "c"),
        ("src/a.h", "cpp"),
        ("src/c/a.h", "c"),
        ("src2/a.h", "c"),
        ("src/a.c", "cpp"),
        ("a.inc", "c"),
    ] {
        assert_eq!(
            original
                .classify(path)
                .unwrap()
                .unwrap()
                .assigned_language(),
            Some(language),
            "{path}"
        );
    }
    let mut redundant = config;
    redundant.assignments.reverse();
    redundant.assignments.push(Assignment {
        scope: "src/c/nested".into(),
        language: FamilyLanguage::C,
    });
    redundant.assignments.push(Assignment {
        scope: "src/".into(),
        language: FamilyLanguage::Cpp,
    });
    assert_eq!(original.id(), policy(redundant).id());
    assert!(
        SourcePolicy::new(
            PolicyConfig {
                assignments: vec![Assignment {
                    scope: "elsewhere".into(),
                    language: FamilyLanguage::C
                }],
                ..Default::default()
            },
            []
        )
        .is_err()
    );
}

#[test]
fn dotfiles_are_not_suffixes_and_all_candidate_suffixes_keep_case_sensitive_meanings() {
    let p = policy(PolicyConfig {
        c_family_roots: vec![".".into()],
        ..Default::default()
    });
    for path in [
        ".rs", ".py", ".c", "src/.cpp", "a.PY", "a.RS", "a.lpc", "a.",
    ] {
        assert!(p.classify(path).unwrap().is_none(), "{path}");
    }
    for suffix in ["cc", "cpp", "cxx", "C", "c++", "hh", "hpp", "hxx", "h++"] {
        assert_eq!(
            p.classify(&format!("a.{suffix}"))
                .unwrap()
                .unwrap()
                .assigned_language(),
            Some("cpp")
        );
    }
    for suffix in ["h", "H"] {
        assert!(matches!(
            p.classify(&format!("a.{suffix}")).unwrap().unwrap(),
            Classification::SourceOnly {
                reason: SourceOnlyReason::LanguageUnassigned,
                ..
            }
        ));
    }
    for suffix in ["inc", "inl", "ipp", "tpp"] {
        assert!(matches!(
            p.classify(&format!("a.{suffix}")).unwrap().unwrap(),
            Classification::SourceOnly {
                reason: SourceOnlyReason::FragmentUnassigned,
                ..
            }
        ));
    }
    for suffix in ["cu", "cuh"] {
        assert!(matches!(
            p.classify(&format!("a.{suffix}")).unwrap().unwrap(),
            Classification::SourceOnly {
                reason: SourceOnlyReason::DialectUnsupported,
                ..
            }
        ));
    }
}

#[test]
fn policy_and_classification_participate_in_versions_without_changing_file_identity() {
    let workspace = WorkspaceId::derive(&[b"test"]);
    let p = policy(PolicyConfig {
        c_family_roots: vec![".".into()],
        ..Default::default()
    });
    let q = policy(PolicyConfig {
        c_family_roots: vec!["include".into()],
        ..Default::default()
    });
    let empty_a = Snapshot::new(workspace.clone(), vec![], p.id().clone()).unwrap();
    let empty_b = Snapshot::new(workspace.clone(), vec![], q.id().clone()).unwrap();
    assert_ne!(empty_a.id(), empty_b.id());
    assert_eq!(empty_a.policy(), p.id());
    let before = p.classify("include/a.h").unwrap().unwrap();
    let mut after = before.clone();
    if let Classification::SourceOnly { provenance, .. } = &mut after {
        *provenance = Provenance::ExplicitScope {
            scope: "include".into(),
        };
    }
    let a = Document::capture(
        workspace.clone(),
        "include/a.h".into(),
        b"struct Header;",
        vec![],
        before.clone(),
    )
    .unwrap();
    let b = Document::capture(
        workspace,
        "include/a.h".into(),
        b"struct Header;",
        vec![],
        after.clone(),
    )
    .unwrap();
    assert_eq!(a.id(), b.id());
    assert_ne!(a.version(), b.version());
    assert_eq!(a.classification(), &before);
    assert_eq!(b.classification(), &after);
}

#[test]
fn source_only_encoding_and_pinned_classification_do_not_require_fake_analysis() {
    use std::sync::Arc;
    use vizzi_core::{model::SourceEncoding, rust::RustAnalyzer};
    let workspace = WorkspaceId::derive(&[b"test"]);
    let p = policy(PolicyConfig {
        c_family_roots: vec![".".into()],
        ..Default::default()
    });
    let metadata = p.classify("a.h").unwrap().unwrap();
    let analysis = RustAnalyzer::new()
        .unwrap()
        .analyze(b"fn rust() {}")
        .unwrap();
    assert!(
        Document::capture(
            workspace.clone(),
            "a.h".into(),
            b"fn rust() {}",
            vec![analysis],
            metadata.clone()
        )
        .is_err()
    );
    let a = Arc::new(
        Document::capture(
            workspace.clone(),
            "a.h".into(),
            &[255],
            vec![],
            metadata.clone(),
        )
        .unwrap(),
    );
    assert_eq!(a.encoding(), SourceEncoding::Unsupported);
    assert!(a.resident_source().is_none());
    let snapshot = Snapshot::new(workspace.clone(), vec![a.clone()], p.id().clone()).unwrap();
    let view = snapshot.view(&Default::default()).unwrap();
    assert_eq!(view.source_only, 1);
    assert_eq!(view.analyzed, 0);
    assert_eq!(view.total, 1);
    let source = view.source.unwrap();
    assert_eq!(source.classification, metadata);
    assert_eq!(source.encoding, SourceEncoding::Unsupported);
    assert!(source.outlines.is_empty());
    let q = policy(PolicyConfig {
        c_family_roots: vec!["include".into()],
        ..Default::default()
    });
    let same_documents = Snapshot::new(workspace, vec![a], q.id().clone()).unwrap();
    assert_ne!(snapshot.id(), same_documents.id());
}
