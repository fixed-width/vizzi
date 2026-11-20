//! Package cues from captured JavaScript and TypeScript manifests.

use super::{
    MANIFEST_BYTES, Module, ModuleEvidence, ModuleIndex, ModuleKind, ModuleRule, TOTAL_BYTES,
    parent,
};
use crate::{
    analysis::ECMASCRIPT_LANGUAGES,
    model::{Document, Snapshot, WorkspaceId},
    policy::{Classification, PACKAGE_METADATA_LANGUAGE, SourceOnlyReason},
};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::Arc};

pub(super) const RULE: ModuleRule = ModuleRule {
    languages: ECMASCRIPT_LANGUAGES,
    kind: ModuleKind {
        language: "javascript",
        basis: "package_json",
        tooltip: "JavaScript and TypeScript package",
    },
    fact: |index, doc, _, _| {
        index
            .packages
            .get(doc.path())
            .map(|module| module.as_ref().clone())
    },
    history: Some(history),
};

fn manifests(path: &str) -> impl Iterator<Item = String> {
    std::iter::successors(Some(parent(path)), |dir| {
        (!dir.is_empty()).then(|| parent(dir))
    })
    .take(64)
    .map(|dir| {
        if dir.is_empty() {
            "package.json".into()
        } else {
            format!("{dir}/package.json")
        }
    })
}

fn is_metadata(classification: &Classification) -> bool {
    matches!(classification, Classification::SourceOnly {
        language: Some(language),
        reason: SourceOnlyReason::ProjectMetadata,
        ..
    } if language == PACKAGE_METADATA_LANGUAGE)
}

fn valid_name(name: &str) -> bool {
    let part = |value: &str| {
        !value.is_empty()
            && !matches!(value, "." | "..")
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    };
    name.len() <= 512
        && if let Some(scoped) = name.strip_prefix('@') {
            scoped
                .split_once('/')
                .is_some_and(|(scope, leaf)| part(scope) && part(leaf))
        } else {
            part(name)
        }
}

fn from_manifest(workspace: &WorkspaceId, doc: &Document, budget: &mut usize) -> Option<Module> {
    #[derive(Deserialize)]
    struct Manifest {
        name: String,
    }
    if !is_metadata(doc.classification()) {
        return None;
    }
    let text = doc
        .resident_source()
        .filter(|text| text.len() <= MANIFEST_BYTES)?;
    *budget = budget.checked_sub(text.len())?;
    if !text.trim_start().starts_with('{') {
        return None;
    }
    let Manifest { name } = serde_json::from_str(text).ok()?;
    if !valid_name(&name) {
        return None;
    }
    let root = doc.path();
    let workspace = workspace.as_str();
    let id = blake3::hash(format!("{workspace}\0{}\0{root}", RULE.kind.basis).as_bytes());
    Some(
        RULE.kind
            .describe(id.to_hex().to_string(), name, root.into()),
    )
}

fn nearest_manifest<'a>(snapshot: &'a Snapshot, path: &str) -> Option<&'a Document> {
    manifests(path).find_map(|candidate| snapshot.file_by_path(&candidate).map(AsRef::as_ref))
}

pub(super) fn bindings<'a>(
    snapshot: &Snapshot,
    documents: impl Iterator<Item = &'a Document>,
) -> BTreeMap<String, Arc<Module>> {
    let mut parsed = BTreeMap::new();
    let mut budget = TOTAL_BYTES;
    documents
        .filter_map(|doc| {
            let language = doc.classification().assigned_language()?;
            if !ECMASCRIPT_LANGUAGES.contains(&language) {
                return None;
            }
            let manifest = nearest_manifest(snapshot, doc.path())?;
            let module = parsed.entry(manifest.path()).or_insert_with(|| {
                from_manifest(snapshot.workspace(), manifest, &mut budget).map(Arc::new)
            });
            Some((doc.path().into(), Arc::clone(module.as_ref()?)))
        })
        .collect()
}

fn history(
    workspace: &WorkspaceId,
    selected: &Arc<Document>,
    evidence: &dyn ModuleEvidence,
) -> Option<ModuleIndex> {
    let paths = manifests(selected.path()).collect::<Vec<_>>();
    let regular = evidence.regular_paths(&mut paths.iter().cloned())?;
    let closest = paths.iter().find(|path| regular.contains(*path))?;
    let manifest = evidence.captured_file(closest, MANIFEST_BYTES)?;
    let mut index = ModuleIndex::empty(workspace);
    let mut budget = MANIFEST_BYTES;
    index.packages.insert(
        selected.path().into(),
        Arc::new(from_manifest(workspace, &manifest, &mut budget)?),
    );
    Some(index)
}
