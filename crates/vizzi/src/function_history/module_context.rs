//! Bounded module evidence acquired from a selected Git commit.

use super::*;
use crate::review_source::CaptureLimits;
use vizzi_core::{model::CapturedSource, policy::SourcePolicy};

const LIMITS: CaptureLimits = CaptureLimits {
    max_bytes: 8 * 1024 * 1024,
    max_file_bytes: 1024 * 1024,
    max_paths: 32_768,
    max_discovery_bytes: 4 * 1024 * 1024,
};
const DOCUMENTS: usize = 8192;
const MANIFEST_BYTES: usize = 128 * 1024;
const PACKAGE_PATHS: usize = 64;
const PACKAGE_DISCOVERY_BYTES: usize = 256 * 1024;

pub(super) fn python_modules(
    repository: &GitRepository,
    commit: &str,
    policy: &SourcePolicy,
    path: &str,
) -> Option<ModuleIndex> {
    let mut paths = Vec::new();
    let mut ancestor = path;
    while let Some((parent, _)) = ancestor.rsplit_once('/') {
        let initializer = format!("{parent}/__init__.py");
        if policy.excludes(&initializer).ok()? {
            break;
        }
        if paths.len() == PACKAGE_PATHS {
            return None;
        }
        paths.push(initializer);
        ancestor = parent;
    }
    let initializers = repository
        .regular_files_at(commit, &paths, PACKAGE_DISCOVERY_BYTES)
        .ok()?;
    Some(ModuleIndex::from_python_context(
        repository.workspace(),
        path,
        &initializers,
    ))
}

pub(super) fn rust_modules(
    repository: &GitRepository,
    commit: &str,
    policy: &SourcePolicy,
    selected: &Arc<Document>,
) -> Option<ModuleIndex> {
    if selected.bytes() > LIMITS.max_file_bytes {
        return None;
    }
    let cancelled = || false;
    let mut revision = repository.pin(commit, LIMITS, &cancelled).ok()?;
    let mut bytes_left = LIMITS.max_bytes.checked_sub(selected.bytes())?;
    let mut documents = vec![selected.clone()];
    for path in revision.paths(LIMITS, &cancelled).ok()? {
        let Some(name) = path.document_path().filter(|&name| name != selected.path()) else {
            continue;
        };
        let Some(classification) = policy.classify(name).ok().flatten() else {
            continue;
        };
        let limit = if name.rsplit('/').next() == Some("Cargo.toml") {
            MANIFEST_BYTES
        } else if classification.assigned_language() == Some("rust") {
            LIMITS.max_file_bytes
        } else {
            continue;
        };
        let bytes = match revision
            .read(&path, limit.min(bytes_left), &cancelled)
            .ok()?
        {
            ReadEntry::Present {
                kind: EntryKind::Regular { .. },
                bytes,
            } => bytes,
            ReadEntry::Present { .. } => continue,
            _ => return None,
        };
        if documents.len() == DOCUMENTS {
            return None;
        }
        bytes_left -= bytes.len();
        let doc = Document::capture_source(
            repository.workspace().clone(),
            name.into(),
            CapturedSource::capture(bytes, None),
            vec![],
            classification,
        )
        .ok()?;
        documents.push(Arc::new(doc));
    }
    let context = Snapshot::new(
        repository.workspace().clone(),
        documents,
        policy.id().clone(),
    )
    .ok()?;
    Some(ModuleIndex::from_rust_context(&context, selected.path()))
}
