//! Native list/read acquisition of immutable review inputs, independently of analysis.

use std::{collections::BTreeMap, sync::Arc};
use vizzi_core::{
    comparison::{
        EntryContent, EntryKind, EntryState, InventoryEntry, Observation, ReviewPath,
        Unavailability,
    },
    model::{CapturePolicyId, CapturedSource, Document, ModelError, Snapshot, WorkspaceId},
    policy::{Classification, Provenance, SourceOnlyReason},
};

mod disk;
mod git;
pub use disk::DirectoryInput;
pub use git::{GitRepository, GitRevision, GitWorking};

const READ_CHUNK: usize = 64 * 1024;

/// Explicit bounds for one private acquisition, separate from pair-lifetime admission.
#[derive(Clone, Copy, Debug)]
pub struct CaptureLimits {
    /// Maximum acquired content bytes, including retained target/reference values.
    pub max_bytes: usize,
    /// Maximum bytes in one acquired entry.
    pub max_file_bytes: usize,
    /// Maximum enumerated candidates.
    pub max_paths: usize,
    /// Maximum bytes returned by one Git discovery command.
    pub max_discovery_bytes: usize,
}

impl Default for CaptureLimits {
    fn default() -> Self {
        Self {
            max_bytes: 2 * 1024 * 1024 * 1024,
            max_file_bytes: 32 * 1024 * 1024,
            max_paths: 1_000_000,
            max_discovery_bytes: 128 * 1024 * 1024,
        }
    }
}

/// Failed/cancelled acquisition never supplies a partial complete observation.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    /// Native acquisition IO failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Inventory/source validation failed.
    #[error(transparent)]
    Model(#[from] ModelError),
    /// Explicit resource allowance refused this capture.
    #[error("review capture exceeds {0} allowance")]
    Limit(&'static str),
    /// Cooperative cancellation discarded this capture.
    #[error("review capture cancelled")]
    Cancelled,
    /// Read-only Git acquisition failed or returned invalid output.
    #[error("Git acquisition failed: {0}")]
    Git(String),
    /// A state/platform needs additional acquisition support.
    #[error("unsupported review input: {0}")]
    Unsupported(&'static str),
}

#[cfg(unix)]
impl From<rustix::io::Errno> for CaptureError {
    fn from(error: rustix::io::Errno) -> Self {
        Self::Io(error.into())
    }
}

/// Raw entry evidence supplied by native acquisition before immutable capture.
#[derive(Clone, Debug)]
pub enum ReadEntry {
    /// Verified absence.
    Absent,
    /// Presence was not established.
    Unobserved(Unavailability),
    /// Known type and exact bytes; target/reference values are literal, never followed.
    Present {
        /// Entry type/executable bit.
        kind: EntryKind,
        /// Raw file, symlink target or canonical reference bytes.
        bytes: Vec<u8>,
    },
    /// Presence/type is known, but content could not be read.
    Unavailable {
        /// Observed entry metadata.
        kind: EntryKind,
        /// Why content was not captured.
        reason: Unavailability,
    },
}

/// Additional acquisition provenance, separate from content equality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    /// An unmerged index exists; after bytes still come from the actual file.
    UnmergedIndex,
    /// A nested reference is captured without recursively assessing its contents.
    NestedContentsNotCaptured,
}

/// A path-scoped acquisition notice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureNotice {
    /// Logical path to which the notice applies.
    pub path: ReviewPath,
    /// Provenance/coverage limitation.
    pub kind: NoticeKind,
}

/// Source seam shared by directory, revision, working-tree and in-memory inputs.
pub trait ReviewInput {
    /// Logical workspace identity, independently of temporary storage paths.
    fn workspace(&self) -> &WorkspaceId;
    /// Completely enumerates candidates or fails; omissions cannot represent errors.
    fn paths(
        &mut self,
        limits: CaptureLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<ReviewPath>, CaptureError>;
    /// Reads one candidate within its byte allowance, without invoking project tooling.
    fn read(
        &mut self,
        path: &ReviewPath,
        limit: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ReadEntry, CaptureError>;
    /// Returns captured native provenance without changing portable comparison rules.
    fn notices(&self) -> Vec<CaptureNotice>;
}

/// Frozen review subtree and explicit content exclusions.
#[derive(Clone)]
pub struct CapturePolicy {
    scope: Option<ReviewPath>,
    exclusions: Vec<ReviewPath>,
}

impl CapturePolicy {
    /// Normalises exclusion ordering; `None` selects the logical workspace root.
    pub fn scope(scope: Option<ReviewPath>, mut exclusions: Vec<ReviewPath>) -> Self {
        exclusions.sort();
        exclusions.dedup();
        Self { scope, exclusions }
    }
}

/// Immutable source-only observation, with nonregular literal values and notices.
pub struct Acquired {
    /// Complete supplied source and inventory; no language analysers ran.
    pub observation: Arc<Observation>,
    /// Literal nonregular values, keyed by logical path.
    pub values: BTreeMap<ReviewPath, Vec<u8>>,
    /// Native provenance/coverage notices.
    pub notices: Vec<CaptureNotice>,
    /// Number of verified acquired bytes, excluding metadata/allocator overhead.
    pub bytes: usize,
}

/// Captures complete source-only inputs; global owner/admission limits remain the host's job.
pub fn acquire(
    input: &mut dyn ReviewInput,
    policy: &CapturePolicy,
    limits: CaptureLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Acquired, CaptureError> {
    check(cancelled)?;
    let mut paths = input.paths(limits, cancelled)?;
    if paths.len() > limits.max_paths {
        return Err(CaptureError::Limit("candidate count"));
    }
    paths.sort();
    if paths.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CaptureError::Unsupported("duplicate input path"));
    }
    let mut documents = vec![];
    let mut inventory = vec![];
    let mut values = BTreeMap::new();
    let mut bytes = 0usize;
    for path in paths {
        check(cancelled)?;
        if !policy
            .scope
            .as_ref()
            .is_none_or(|scope| within(&path, scope))
        {
            continue;
        }
        let state = if policy
            .exclusions
            .iter()
            .any(|excluded| within(&path, excluded))
        {
            EntryState::Unobserved(Unavailability::Excluded)
        } else {
            let allowance = (limits.max_bytes - bytes).min(limits.max_file_bytes);
            match input.read(&path, allowance, cancelled)? {
                ReadEntry::Absent => continue,
                ReadEntry::Unobserved(reason) => EntryState::Unobserved(reason),
                ReadEntry::Unavailable { kind, reason } => EntryState::Present {
                    kind,
                    content: EntryContent::Unavailable(reason),
                },
                ReadEntry::Present { kind, bytes: data } => {
                    if data.len() > allowance {
                        return Err(CaptureError::Limit("content bytes"));
                    }
                    bytes += data.len();
                    let content = if matches!(kind, EntryKind::Regular { .. }) {
                        let source = CapturedSource::capture(data, None);
                        let content = EntryContent::Verified {
                            id: source.content().clone(),
                            bytes: source.len(),
                        };
                        if let Some(name) = path.document_path() {
                            documents.push(Arc::new(Document::capture_source(
                                input.workspace().clone(),
                                name.to_owned(),
                                source,
                                vec![],
                                Classification::SourceOnly {
                                    language: None,
                                    provenance: Provenance::ExplicitScope { scope: ".".into() },
                                    reason: SourceOnlyReason::AnalysisNotRequested,
                                },
                            )?));
                        }
                        content
                    } else {
                        let content = EntryContent::verified(&data);
                        values.insert(path.clone(), data);
                        content
                    };
                    EntryState::Present { kind, content }
                }
            }
        };
        inventory.push(InventoryEntry { path, state });
    }
    check(cancelled)?;
    let snapshot = Arc::new(Snapshot::new(
        input.workspace().clone(),
        documents,
        CapturePolicyId::derive(&[b"native-review-source-only-v1"]),
    )?);
    let encoded =
        serde_json::to_vec(&(&policy.scope, &policy.exclusions)).map_err(std::io::Error::other)?;
    let observation = Arc::new(Observation::from_inventory(
        snapshot,
        CapturePolicyId::derive(&[b"native-review-scope-v1", &encoded]),
        inventory,
    )?);
    let notices = input
        .notices()
        .into_iter()
        .filter(|notice| {
            policy
                .scope
                .as_ref()
                .is_none_or(|scope| within(&notice.path, scope))
        })
        .collect();
    check(cancelled)?;
    Ok(Acquired {
        observation,
        values,
        notices,
        bytes,
    })
}

fn within(path: &ReviewPath, prefix: &ReviewPath) -> bool {
    path == prefix
        || path
            .bytes()
            .strip_prefix(prefix.bytes())
            .is_some_and(|tail| tail.starts_with(b"/"))
}

fn check(cancelled: &dyn Fn() -> bool) -> Result<(), CaptureError> {
    if cancelled() {
        Err(CaptureError::Cancelled)
    } else {
        Ok(())
    }
}

fn read_bytes(
    reader: &mut dyn std::io::Read,
    limit: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<u8>, CaptureError> {
    let mut data = vec![];
    let mut buffer = [0u8; READ_CHUNK];
    loop {
        check(cancelled)?;
        let count = buffer
            .len()
            .min(limit.saturating_sub(data.len()).saturating_add(1));
        let count = reader.read(&mut buffer[..count])?;
        if count == 0 {
            break;
        }
        if count > limit - data.len() {
            return Err(CaptureError::Limit("content bytes"));
        }
        data.extend_from_slice(&buffer[..count]);
    }
    check(cancelled)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn byte_allowances_hold_across_multiple_read_chunks() {
        let bytes = vec![b'x'; 70_001];
        assert_eq!(
            read_bytes(&mut std::io::Cursor::new(&bytes[..70_000]), 70_000, &|| {
                false
            })
            .unwrap()
            .len(),
            70_000
        );
        assert!(matches!(
            read_bytes(&mut std::io::Cursor::new(&bytes), 70_000, &|| false),
            Err(CaptureError::Limit(_))
        ));
        assert!(
            read_bytes(&mut std::io::Cursor::new([]), 0, &|| false)
                .unwrap()
                .is_empty()
        );
    }
}
