//! Verified capture metadata and cheap immutable snapshot updates.

use std::sync::Arc;
use vizzi_core::{
    analysis::Languages,
    capture::CapturePlan,
    model::{Document, Snapshot, WorkspaceId},
    policy::{Classification, PolicyConfig, Provenance},
};

fn plan() -> CapturePlan {
    CapturePlan::new(PolicyConfig::default(), &Languages::builtin().unwrap()).unwrap()
}
fn workspace() -> WorkspaceId {
    WorkspaceId::derive(&[b"verified-capture"])
}
fn document(plan: &CapturePlan, path: &str, text: &str) -> Arc<Document> {
    let language = Languages::builtin().unwrap();
    let analysis = language
        .select("rust")
        .unwrap()
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    Arc::new(
        Document::capture(
            workspace(),
            path.into(),
            text.as_bytes(),
            vec![analysis],
            plan.policy().classify(path).unwrap().unwrap(),
        )
        .unwrap(),
    )
}

#[test]
fn verified_updates_match_full_capture_and_share_unchanged_documents() {
    let plan = plan();
    let a = document(&plan, "a.rs", "fn a() {}");
    let b = document(&plan, "b.rs", "fn b() {}");
    let before = plan
        .snapshot(workspace(), vec![b.clone(), a.clone()])
        .unwrap();
    assert_eq!(before.capture_plan(), Some(plan.id()));
    let changed = document(&plan, "b.rs", "fn changed() {}");
    let added = document(&plan, "c.rs", "fn added() {}");
    let updated = plan
        .update(&before, vec![added.clone(), changed.clone()], &[])
        .unwrap();
    assert!(Arc::ptr_eq(&updated.documents()[0], &a));
    assert_eq!(
        updated.id(),
        plan.snapshot(workspace(), vec![a, changed, added])
            .unwrap()
            .id()
    );
    let removed = plan.update(&updated, vec![], &["b.rs".into()]).unwrap();
    assert_eq!(
        removed
            .documents()
            .iter()
            .map(|file| file.path())
            .collect::<Vec<_>>(),
        ["a.rs", "c.rs"]
    );
    assert_eq!(
        removed.id(),
        plan.snapshot(
            workspace(),
            vec![
                removed.documents()[0].clone(),
                removed.documents()[1].clone()
            ]
        )
        .unwrap()
        .id()
    );
    assert_eq!(
        before.file_by_path("b.rs").unwrap().source(),
        Some("fn b() {}")
    );
    assert!(before.file_by_path("missing.rs").is_none());
}

#[test]
fn generic_snapshots_wrong_metadata_and_conflicting_updates_cannot_claim_validation() {
    let plan = plan();
    let a = document(&plan, "a.rs", "fn a() {}");
    let generic = Snapshot::new(workspace(), vec![a.clone()], plan.policy().id().clone()).unwrap();
    assert_eq!(generic.capture_plan(), None);
    assert!(plan.update(&generic, vec![], &[]).is_err());
    let good = plan.snapshot(workspace(), vec![a.clone()]).unwrap();
    assert!(plan.update(&good, vec![a.clone(), a.clone()], &[]).is_err());
    assert!(
        plan.update(&good, vec![a.clone()], &["a.rs".into()])
            .is_err()
    );
    assert!(plan.update(&good, vec![], &["../a.rs".into()]).is_err());
    let mut analysis = a.analyses()[0].clone();
    analysis.context.description = "different configuration".into();
    let bad = Arc::new(
        Document::capture(
            workspace(),
            "a.rs".into(),
            b"fn a() {}",
            vec![analysis],
            a.classification().clone(),
        )
        .unwrap(),
    );
    assert!(plan.snapshot(workspace(), vec![bad.clone()]).is_err());
    assert!(plan.update(&good, vec![bad], &[]).is_err());
    let bad = Arc::new(
        Document::capture(
            workspace(),
            "a.rs".into(),
            b"fn a() {}",
            a.analyses().to_vec(),
            Classification::Assigned {
                language: "rust".into(),
                provenance: Provenance::ExplicitScope { scope: ".".into() },
            },
        )
        .unwrap(),
    );
    assert!(plan.snapshot(workspace(), vec![bad]).is_err());
}

#[test]
fn policy_and_analysis_plan_changes_invalidate_even_empty_snapshots() {
    let first = plan();
    let empty = first.snapshot(workspace(), vec![]).unwrap();
    let other = CapturePlan::new(
        PolicyConfig {
            exclusions: vec!["ignored".into()],
            ..Default::default()
        },
        &Languages::builtin().unwrap(),
    )
    .unwrap();
    assert_ne!(first.id(), other.id());
    assert!(other.update(&empty, vec![], &[]).is_err());
    assert_ne!(
        empty.id(),
        other.snapshot(workspace(), vec![]).unwrap().id()
    );
    let generic = Snapshot::new(workspace(), vec![], first.policy().id().clone()).unwrap();
    assert_ne!(empty.id(), generic.id());
}

#[test]
fn full_context_descriptors_invalidate_a_plan_without_changing_source_policy() {
    use vizzi_core::{
        analysis::{Analyzer, LanguageAdapter},
        model::{Context, ModelError},
        rust::RustAdapter,
    };
    struct Descriptor {
        revision: &'static str,
    }
    impl LanguageAdapter for Descriptor {
        fn context(&self) -> Context {
            let mut context = RustAdapter.context();
            context.description.push_str(self.revision);
            context
        }
        fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
            Err(ModelError("no inputs in empty workspace".into()))
        }
    }
    let make = |revision| {
        CapturePlan::new(
            PolicyConfig::default(),
            &Languages::new(vec![Arc::new(Descriptor { revision })]).unwrap(),
        )
        .unwrap()
    };
    let first = make("first");
    let second = make("second");
    assert_eq!(first.policy().id(), second.policy().id());
    assert_ne!(first.id(), second.id());
    let old = first.snapshot(workspace(), vec![]).unwrap();
    assert_ne!(old.id(), second.snapshot(workspace(), vec![]).unwrap().id());
    assert!(second.update(&old, vec![], &[]).is_err());
}
