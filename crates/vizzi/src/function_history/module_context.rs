//! Bounded module evidence acquired from a selected Git commit.

use crate::review_source::{CaptureLimits, GitRepository, ReadEntry, ReviewInput};
use std::{collections::BTreeSet, sync::Arc};
use vizzi_core::{
    comparison::EntryKind,
    model::{CapturedSource, Document, Snapshot},
    modules::{ModuleEvidence, ModuleIndex},
    policy::{Classification, SourcePolicy},
};

const LIMITS: CaptureLimits = CaptureLimits {
    max_bytes: 8 * 1024 * 1024,
    max_file_bytes: 1024 * 1024,
    max_paths: 32_768,
    max_discovery_bytes: 4 * 1024 * 1024,
};
const DOCUMENTS: usize = 8192;
const PACKAGE_PATHS: usize = 64;
const PACKAGE_DISCOVERY_BYTES: usize = 256 * 1024;

pub(super) fn for_commit(
    repository: &GitRepository,
    commit: &str,
    policy: &SourcePolicy,
    selected: &Arc<Document>,
    language: &str,
) -> ModuleIndex {
    ModuleIndex::for_history(
        repository.workspace(),
        selected,
        language,
        &GitModuleEvidence {
            repository,
            commit,
            policy,
        },
    )
}

struct GitModuleEvidence<'a> {
    repository: &'a GitRepository,
    commit: &'a str,
    policy: &'a SourcePolicy,
}

impl ModuleEvidence for GitModuleEvidence<'_> {
    fn regular_paths(
        &self,
        candidates: &mut dyn Iterator<Item = String>,
    ) -> Option<BTreeSet<String>> {
        let mut paths = Vec::new();
        for path in candidates {
            if self.policy.excludes(&path).ok()? {
                break;
            }
            if paths.len() == PACKAGE_PATHS {
                return None;
            }
            paths.push(path);
        }
        self.repository
            .regular_files_at(self.commit, &paths, PACKAGE_DISCOVERY_BYTES)
            .ok()
    }

    fn captured_files(
        &self,
        selected: &Arc<Document>,
        file_limit: fn(&str, &Classification) -> Option<usize>,
    ) -> Option<Snapshot> {
        if selected.bytes() > LIMITS.max_file_bytes {
            return None;
        }
        let cancelled = || false;
        let mut revision = self.repository.pin(self.commit, LIMITS, &cancelled).ok()?;
        let mut bytes_left = LIMITS.max_bytes.checked_sub(selected.bytes())?;
        let mut documents = vec![selected.clone()];
        for path in revision.paths(LIMITS, &cancelled).ok()? {
            let Some(name) = path.document_path().filter(|&name| name != selected.path()) else {
                continue;
            };
            let Some(classification) = self.policy.classify(name).ok().flatten() else {
                continue;
            };
            let Some(limit) = file_limit(name, &classification) else {
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
                self.repository.workspace().clone(),
                name.into(),
                CapturedSource::capture(bytes, None),
                vec![],
                classification,
            )
            .ok()?;
            documents.push(Arc::new(doc));
        }
        Snapshot::new(
            self.repository.workspace().clone(),
            documents,
            self.policy.id().clone(),
        )
        .ok()
    }
}
