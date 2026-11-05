//! Portable review behaviour through supplied immutable observations, without Git or IO.

use std::sync::Arc;
use vizzi_core::{
    comparison::{
        ChangeKind, Comparison, Difference, EntryContent, EntryKind, EntryState, InventoryEntry,
        Observation, ReviewPath, Side, Unavailability,
    },
    model::{CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance, SourceOnlyReason},
};

fn path(bytes: &[u8]) -> ReviewPath {
    ReviewPath::parse(bytes.to_vec()).unwrap()
}

fn workspace() -> WorkspaceId {
    WorkspaceId::derive(&[b"logical-workspace"])
}

fn policy() -> CapturePolicyId {
    CapturePolicyId::derive(&[b"review-root-scope"])
}

fn snapshot(workspace: WorkspaceId, documents: Vec<Arc<Document>>) -> Arc<Snapshot> {
    Arc::new(
        Snapshot::new(
            workspace,
            documents,
            CapturePolicyId::derive(&[b"analysis"]),
        )
        .unwrap(),
    )
}

fn regular(bytes: &[u8], executable: bool) -> EntryState {
    EntryState::Present {
        kind: EntryKind::Regular { executable },
        content: EntryContent::verified(bytes),
    }
}

fn entry(name: &[u8], state: EntryState) -> InventoryEntry {
    InventoryEntry {
        path: path(name),
        state,
    }
}

fn observe(entries: Vec<InventoryEntry>) -> Arc<Observation> {
    Arc::new(Observation::from_inventory(snapshot(workspace(), vec![]), policy(), entries).unwrap())
}

fn compare(before: Option<EntryState>, after: Option<EntryState>) -> Comparison {
    let entries = |state: Option<EntryState>| {
        state
            .into_iter()
            .map(|state| entry(b"file", state))
            .collect()
    };
    Comparison::between(observe(entries(before)), observe(entries(after))).unwrap()
}

#[test]
fn review_inventory_handles_add_remove_content_and_mode_without_language_analysis() {
    use ChangeKind::*;
    use Difference::*;
    let cases = [
        (
            None,
            Some(regular(b"", false)),
            Added,
            NotApplicable,
            NotApplicable,
        ),
        (
            Some(regular(b"", false)),
            None,
            Removed,
            NotApplicable,
            NotApplicable,
        ),
        (
            Some(regular(b"same", false)),
            Some(regular(b"same", false)),
            Unchanged,
            Equal,
            Equal,
        ),
        (
            Some(regular(b"before", false)),
            Some(regular(b"after", false)),
            Modified,
            Different,
            Equal,
        ),
        (
            Some(regular(b"same", false)),
            Some(regular(b"same", true)),
            Modified,
            Equal,
            Different,
        ),
        (
            Some(regular(b"before", false)),
            Some(regular(b"after", true)),
            Modified,
            Different,
            Different,
        ),
    ];
    for (base, working, change, content, metadata) in cases {
        let pair = compare(base.clone(), working.clone());
        let rows: Vec<_> = pair.entries().collect();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(
            (row.change, row.content, row.metadata),
            (change, content, metadata)
        );
        assert_eq!(row.path, &path(b"file"));
        assert_eq!((row.base, row.working), (base.as_ref(), working.as_ref()));
        let lookup = pair.entry(&path(b"file")).unwrap();
        assert_eq!(
            (lookup.change, lookup.content, lookup.metadata),
            (change, content, metadata)
        );
        assert!(pair.document(Side::Working, &path(b"file")).is_none());
    }
}

#[test]
fn unavailable_evidence_never_means_equal_or_absent() {
    use ChangeKind::*;
    use Difference::*;
    let unknown = EntryState::Unobserved(Unavailability::Sparse);
    let excluded = EntryState::Present {
        kind: EntryKind::Regular { executable: false },
        content: EntryContent::Unavailable(Unavailability::Excluded),
    };
    let denied = EntryState::Present {
        kind: EntryKind::Regular { executable: true },
        content: EntryContent::Unavailable(Unavailability::Unreadable),
    };
    let cases = [
        (Some(unknown.clone()), None, Unassessed, Unknown, Unknown),
        (None, Some(unknown.clone()), Unassessed, Unknown, Unknown),
        (
            Some(unknown.clone()),
            Some(regular(b"x", false)),
            Unassessed,
            Unknown,
            Unknown,
        ),
        (
            Some(regular(b"x", false)),
            Some(unknown.clone()),
            Unassessed,
            Unknown,
            Unknown,
        ),
        (
            Some(unknown.clone()),
            Some(unknown),
            Unassessed,
            Unknown,
            Unknown,
        ),
        (
            Some(excluded.clone()),
            Some(excluded.clone()),
            Unassessed,
            Unknown,
            Equal,
        ),
        (
            Some(excluded.clone()),
            Some(regular(b"x", false)),
            Unassessed,
            Unknown,
            Equal,
        ),
        (
            Some(regular(b"x", false)),
            Some(excluded.clone()),
            Unassessed,
            Unknown,
            Equal,
        ),
        (
            Some(excluded.clone()),
            Some(denied),
            Modified,
            Unknown,
            Different,
        ),
        (
            None,
            Some(excluded.clone()),
            Added,
            NotApplicable,
            NotApplicable,
        ),
        (Some(excluded), None, Removed, NotApplicable, NotApplicable),
    ];
    for (base, working, change, content, metadata) in cases {
        let pair = compare(base.clone(), working.clone());
        let row = pair.entry(&path(b"file")).unwrap();
        assert_eq!(
            (row.change, row.content, row.metadata),
            (change, content, metadata)
        );
        assert_eq!((row.base, row.working), (base.as_ref(), working.as_ref()));
    }
}

#[test]
fn symlinks_references_special_files_and_binary_bytes_remain_in_inventory() {
    let present = |kind, bytes: &[u8]| EntryState::Present {
        kind,
        content: EntryContent::verified(bytes),
    };
    for (base, working) in [
        (
            present(EntryKind::Symlink, b"../old"),
            present(EntryKind::Symlink, b"../new"),
        ),
        (
            present(EntryKind::Reference, b"pointer-a"),
            present(EntryKind::Reference, b"pointer-b"),
        ),
        (
            regular(b"\0binary\xff", false),
            regular(b"\0binary\xfe", false),
        ),
    ] {
        let pair = compare(Some(base), Some(working));
        let row = pair.entry(&path(b"file")).unwrap();
        assert_eq!(
            (row.change, row.content, row.metadata),
            (
                ChangeKind::Modified,
                Difference::Different,
                Difference::Equal
            )
        );
    }
    let pair = compare(
        Some(regular(b"target", false)),
        Some(present(EntryKind::Symlink, b"target")),
    );
    let row = pair.entry(&path(b"file")).unwrap();
    assert_eq!(
        (row.change, row.content, row.metadata),
        (
            ChangeKind::Modified,
            Difference::Equal,
            Difference::Different
        )
    );
    let special = EntryState::Present {
        kind: EntryKind::Special,
        content: EntryContent::Unavailable(Unavailability::UnsupportedKind),
    };
    let pair = compare(Some(special.clone()), Some(special));
    assert_eq!(
        pair.entries().next().unwrap().change,
        ChangeKind::Unassessed
    );
}

#[test]
fn inventory_union_is_sorted_complete_and_does_not_infer_renames() {
    let base = observe(vec![
        entry(b"z-removed", regular(b"moved", false)),
        entry(b"b-shared", regular(b"same", false)),
        entry(b"d-base", regular(b"", false)),
    ]);
    let working = observe(vec![
        entry(b"c-added", regular(b"moved", false)),
        entry(b"b-shared", regular(b"same", false)),
        entry(b"a-first", regular(b"", false)),
    ]);
    assert_eq!(
        base.inventory()
            .iter()
            .map(|e| e.path.bytes())
            .collect::<Vec<_>>(),
        [b"b-shared".as_slice(), b"d-base", b"z-removed"]
    );
    let pair = Comparison::between(base, working).unwrap();
    let rows = pair
        .entries()
        .map(|e| (e.path.display(), e.change))
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            ("a-first".into(), ChangeKind::Added),
            ("b-shared".into(), ChangeKind::Unchanged),
            ("c-added".into(), ChangeKind::Added),
            ("d-base".into(), ChangeKind::Removed),
            ("z-removed".into(), ChangeKind::Removed),
        ]
    );
    assert!(pair.entry(&path(b"missing")).is_none());
    let empty = compare(None, None);
    assert!(empty.entries().next().is_none());
    assert!(empty.entry(&path(b"file")).is_none());
}

#[test]
fn paths_preserve_unusual_names_without_aliasing_or_traversal() {
    for invalid in [
        b"".as_slice(),
        b"/absolute",
        b"a/",
        b"a//b",
        b".",
        b"..",
        b"a/./b",
        b"a/../b",
        b"a\0b",
    ] {
        assert!(ReviewPath::parse(invalid.to_vec()).is_err(), "{invalid:?}");
    }
    for valid in [
        b"a".as_slice(),
        b"a/.../b",
        b".hidden",
        b"a.b",
        "文書.md".as_bytes(),
        b"a\xff",
        b"a\\b",
        b"a\nb",
    ] {
        assert_eq!(path(valid).bytes(), valid);
    }
    assert_eq!(path("文書.md".as_bytes()).document_path(), Some("文書.md"));
    assert_eq!(path("文書.md".as_bytes()).display(), "文書.md");
    assert!(path(b"a\xff").document_path().is_none());
    assert!(path(b"a\\b").document_path().is_none());
    assert_eq!(path(b"a\xff").display(), "a\\xff");
    assert_eq!(path(b"a\\b").display(), "a\\\\b");
    assert_eq!(path(b"a\nb").display(), "a\\nb");
    let names = [b"a\xff".as_slice(), b"a\\xff", b"a\nb", b"a\\nb"];
    let observation = observe(
        names
            .iter()
            .map(|name| {
                entry(
                    name,
                    EntryState::Unobserved(Unavailability::UnsupportedPath),
                )
            })
            .collect(),
    );
    assert_eq!(observation.inventory().len(), 4);
    let ids: std::collections::BTreeSet<_> = names
        .iter()
        .map(|name| path(name).file_id(&workspace()))
        .collect();
    assert_eq!(ids.len(), 4);
    assert_ne!(
        path(b"a").file_id(&workspace()),
        path(b"a").file_id(&WorkspaceId::derive(&[b"other"]))
    );
}

fn source_observation(bytes: &[u8]) -> Arc<Observation> {
    let document = Arc::new(
        Document::capture(
            workspace(),
            "doc/guide.md".into(),
            bytes,
            vec![],
            Classification::SourceOnly {
                language: None,
                provenance: Provenance::Extension {
                    extension: "md".into(),
                },
                reason: SourceOnlyReason::LanguageUnassigned,
            },
        )
        .unwrap(),
    );
    Arc::new(
        Observation::from_inventory(
            snapshot(workspace(), vec![document]),
            policy(),
            vec![entry(b"doc/guide.md", regular(bytes, false))],
        )
        .unwrap(),
    )
}

#[test]
fn pair_keeps_side_specific_source_only_documents_and_compatible_file_identity() {
    let mut input = b"old\r\n".to_vec();
    let before = source_observation(&input);
    input.copy_from_slice(b"new\r\n");
    let after = source_observation(&input);
    let pair = Comparison::between(before.clone(), after.clone()).unwrap();
    let path = path(b"doc/guide.md");
    let base = pair.document(Side::Base, &path).unwrap();
    let working = pair.document(Side::Working, &path).unwrap();
    assert_eq!(base.resident_source(), Some("old\r\n"));
    assert_eq!(working.resident_source(), Some("new\r\n"));
    assert_eq!(path.file_id(&workspace()), *base.id());
    assert_eq!(base.id(), working.id());
    assert!(base.analyses().is_empty());
    assert_eq!(pair.observation(Side::Base).id(), before.id());
    assert_eq!(pair.observation(Side::Working).id(), after.id());
    assert!(pair.document(Side::Base, &self::path(b"unknown")).is_none());
    assert!(
        pair.document(Side::Base, &self::path(b"invalid\xff"))
            .is_none()
    );
    assert_ne!(base.version(), working.version());
}

#[test]
fn observations_reject_duplicate_missing_or_inconsistent_document_evidence() {
    let one = entry(b"a", regular(b"x", false));
    assert!(
        Observation::from_inventory(
            snapshot(workspace(), vec![]),
            policy(),
            vec![one.clone(), one]
        )
        .is_err()
    );
    let original = source_observation(b"content");
    let snap = original.snapshot().clone();
    assert!(Observation::from_inventory(snap.clone(), policy(), vec![]).is_err());
    for state in [
        regular(b"different", false),
        EntryState::Unobserved(Unavailability::Unreadable),
        EntryState::Present {
            kind: EntryKind::Symlink,
            content: EntryContent::verified(b"content"),
        },
        EntryState::Present {
            kind: EntryKind::Regular { executable: false },
            content: EntryContent::Unavailable(Unavailability::Excluded),
        },
    ] {
        assert!(
            Observation::from_inventory(
                snap.clone(),
                policy(),
                vec![entry(b"doc/guide.md", state)]
            )
            .is_err()
        );
    }
    let mut wrong_length = EntryContent::verified(b"content");
    if let EntryContent::Verified { bytes, .. } = &mut wrong_length {
        *bytes += 1;
    }
    assert!(
        Observation::from_inventory(
            snap,
            policy(),
            vec![entry(
                b"doc/guide.md",
                EntryState::Present {
                    kind: EntryKind::Regular { executable: false },
                    content: wrong_length
                }
            )]
        )
        .is_err()
    );
}

#[test]
fn incompatible_workspace_and_both_policy_kinds_are_rejected() {
    let original = observe(vec![]);
    let other = |snap, review| Arc::new(Observation::from_inventory(snap, review, vec![]).unwrap());
    for incompatible in [
        other(snapshot(WorkspaceId::derive(&[b"other"]), vec![]), policy()),
        other(
            original.snapshot().clone(),
            CapturePolicyId::derive(&[b"narrow-scope"]),
        ),
        other(
            Arc::new(
                Snapshot::new(
                    workspace(),
                    vec![],
                    CapturePolicyId::derive(&[b"other-analysis"]),
                )
                .unwrap(),
            ),
            policy(),
        ),
    ] {
        assert!(Comparison::between(original.clone(), incompatible).is_err());
    }
}

#[test]
fn identities_bind_order_inventory_policy_and_source_snapshot() {
    let a = entry(b"a", regular(b"a", false));
    let b = entry(b"b", regular(b"b", false));
    let original = observe(vec![a.clone(), b.clone()]);
    assert_eq!(original.id(), observe(vec![b, a]).id());
    let changed = observe(vec![entry(b"a", regular(b"changed", false))]);
    let pair = Comparison::between(original.clone(), changed.clone()).unwrap();
    assert_eq!(
        pair.id(),
        Comparison::between(original.clone(), changed.clone())
            .unwrap()
            .id()
    );
    assert_ne!(
        pair.id(),
        Comparison::between(changed.clone(), original.clone())
            .unwrap()
            .id()
    );
    assert_ne!(original.id(), changed.id());
    assert_ne!(original.id(), observe(vec![]).id());
    let changed_policy = Observation::from_inventory(
        original.snapshot().clone(),
        CapturePolicyId::derive(&[b"other"]),
        original.inventory().to_vec(),
    )
    .unwrap();
    assert_ne!(original.id(), changed_policy.id());
    let regular_file = observe(vec![entry(b"a", regular(b"a", false))]);
    for state in [
        regular(b"a", true),
        EntryState::Present {
            kind: EntryKind::Symlink,
            content: EntryContent::verified(b"a"),
        },
        EntryState::Unobserved(Unavailability::Excluded),
        EntryState::Unobserved(Unavailability::Unreadable),
    ] {
        assert_ne!(regular_file.id(), observe(vec![entry(b"a", state)]).id());
    }
    let source = source_observation(b"a");
    let metadata_only = observe(source.inventory().to_vec());
    assert_ne!(source.id(), metadata_only.id());
}

#[test]
fn identity_distinguishes_each_inventory_dimension_and_availability_reason() {
    let mut other_length = EntryContent::verified(b"x");
    if let EntryContent::Verified { bytes, .. } = &mut other_length {
        *bytes = 2;
    }
    let states = [
        regular(b"x", false),
        regular(b"x", true),
        regular(b"y", false),
        EntryState::Present {
            kind: EntryKind::Regular { executable: false },
            content: other_length,
        },
        EntryState::Present {
            kind: EntryKind::Symlink,
            content: EntryContent::verified(b"x"),
        },
        EntryState::Present {
            kind: EntryKind::Reference,
            content: EntryContent::verified(b"x"),
        },
        EntryState::Present {
            kind: EntryKind::Regular { executable: false },
            content: EntryContent::Unavailable(Unavailability::Excluded),
        },
        EntryState::Present {
            kind: EntryKind::Regular { executable: false },
            content: EntryContent::Unavailable(Unavailability::Unreadable),
        },
        EntryState::Unobserved(Unavailability::Excluded),
        EntryState::Unobserved(Unavailability::Unreadable),
    ];
    let observations = states
        .into_iter()
        .map(|state| observe(vec![entry(b"file", state)]))
        .collect::<Vec<_>>();
    let identities = observations
        .iter()
        .map(|observation| observation.id())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(identities.len(), observations.len());
    assert_eq!(observations[0].policy(), &policy());
    assert_ne!(
        observations[0].id(),
        observe(vec![entry(b"other-path", regular(b"x", false))]).id()
    );
    let pair = Comparison::between(observations[0].clone(), observations[3].clone()).unwrap();
    assert_eq!(
        pair.entries().next().unwrap().content,
        Difference::Different
    );
}

#[test]
fn capture_plan_validation_is_not_interchangeable_with_generic_snapshot_policy() {
    use vizzi_core::{analysis::Languages, capture::CapturePlan, policy::PolicyConfig};
    let plan = CapturePlan::new(PolicyConfig::default(), &Languages::builtin().unwrap()).unwrap();
    let verified = Arc::new(plan.snapshot(workspace(), vec![]).unwrap());
    let generic = Arc::new(Snapshot::new(workspace(), vec![], plan.policy().id().clone()).unwrap());
    let before = Arc::new(Observation::from_inventory(verified, policy(), vec![]).unwrap());
    let after = Arc::new(Observation::from_inventory(generic, policy(), vec![]).unwrap());
    assert!(Comparison::between(before.clone(), before.clone()).is_ok());
    assert!(Comparison::between(before, after).is_err());
}

#[test]
fn added_and_removed_documents_are_readable_only_from_the_present_side() {
    let source = source_observation(b"retained evidence\n");
    let empty = observe(vec![]);
    let path = path(b"doc/guide.md");
    for (pair, present, absent, change) in [
        (
            Comparison::between(source.clone(), empty.clone()).unwrap(),
            Side::Base,
            Side::Working,
            ChangeKind::Removed,
        ),
        (
            Comparison::between(empty, source).unwrap(),
            Side::Working,
            Side::Base,
            ChangeKind::Added,
        ),
    ] {
        assert_eq!(pair.entry(&path).unwrap().change, change);
        assert_eq!(
            pair.document(present, &path).unwrap().resident_source(),
            Some("retained evidence\n")
        );
        assert!(pair.document(absent, &path).is_none());
    }
}

#[test]
fn analysis_coverage_changes_identity_without_inventing_a_content_change() {
    use vizzi_core::model::{Analysis, ContentId, Context, ContextId, Coverage};
    let bytes = b"fn duplicate() {}\nfn duplicate() {}\n";
    let observe_coverage = |coverage| {
        let document = Arc::new(
            Document::capture(
                workspace(),
                "code.rs".into(),
                bytes,
                vec![Analysis {
                    coloring: None,
                    content: ContentId::derive(&[bytes]),
                    context: Context {
                        language: "rust".into(),
                        description: "fixture context".into(),
                        id: ContextId::derive(&[b"fixture-context"]),
                    },
                    coverage,
                    facts: vec![],
                    diagnostics: vec![],
                }],
                Classification::Assigned {
                    language: "rust".into(),
                    provenance: Provenance::Extension {
                        extension: "rs".into(),
                    },
                },
            )
            .unwrap(),
        );
        Arc::new(
            Observation::from_inventory(
                snapshot(workspace(), vec![document]),
                policy(),
                vec![entry(b"code.rs", regular(bytes, false))],
            )
            .unwrap(),
        )
    };
    let base = observe_coverage(Coverage::Syntax);
    let working = observe_coverage(Coverage::Unavailable);
    assert_ne!(base.id(), working.id());
    let pair = Comparison::between(base, working).unwrap();
    assert_eq!(
        pair.entry(&path(b"code.rs")).unwrap().change,
        ChangeKind::Unchanged
    );
    assert_eq!(
        pair.document(Side::Base, &path(b"code.rs"))
            .unwrap()
            .analyses()[0]
            .coverage,
        Coverage::Syntax
    );
    assert_eq!(
        pair.document(Side::Working, &path(b"code.rs"))
            .unwrap()
            .analyses()[0]
            .coverage,
        Coverage::Unavailable
    );
}
