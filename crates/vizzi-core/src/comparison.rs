//! Immutable review inventories and comparisons, independent of source-control systems.

use crate::model::{
    CapturePolicyId, ComparisonId, ContentId, Document, FileId, ModelError, ObservationId,
    Snapshot, WorkspaceId,
};
use serde::Serialize;
use std::sync::Arc;

/// Lossless workspace-relative path bytes with `/` separators and no dot components.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ReviewPath(Vec<u8>);

impl ReviewPath {
    /// Validates a relative path without normalising or decoding its bytes.
    pub fn parse(bytes: Vec<u8>) -> Result<Self, ModelError> {
        if bytes.contains(&0)
            || bytes
                .split(|byte| *byte == b'/')
                .any(|part| matches!(part, b"" | b"." | b".."))
        {
            return Err(ModelError(
                "expected a relative review path without dot components or NUL".into(),
            ));
        }
        Ok(Self(bytes))
    }
    /// Returns the original relative path bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
    /// Returns the path when the existing document model can represent it.
    pub fn document_path(&self) -> Option<&str> {
        std::str::from_utf8(&self.0)
            .ok()
            .filter(|path| !path.contains('\\'))
    }
    /// Returns readable Unicode or a losslessly escaped byte representation.
    pub fn display(&self) -> String {
        match std::str::from_utf8(&self.0) {
            Ok(text) if !text.chars().any(|ch| ch.is_control() || ch == '\\') => text.to_owned(),
            _ => self.0.escape_ascii().to_string(),
        }
    }
    /// Derives a path identity, compatible with ordinary document paths.
    pub fn file_id(&self, workspace: &WorkspaceId) -> FileId {
        FileId::derive(&[workspace.as_str().as_bytes(), &self.0])
    }
}

/// Entry metadata that can change independently of content bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum EntryKind {
    /// A regular file, with its executable bit.
    Regular {
        #[doc = "Whether the executable bit is set."]
        executable: bool,
    },
    /// A literal symlink target, never a followed path.
    Symlink,
    /// An opaque external-tree reference, such as a submodule pointer.
    Reference,
    /// A special entry for which ordinary byte reads are unsupported.
    Special,
}

/// Why bytes or the entry itself could not be assessed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Unavailability {
    /// Explicit capture exclusion.
    Excluded,
    /// A failed entry or content read.
    Unreadable,
    /// Sparse materialisation does not establish filesystem deletion.
    Sparse,
    /// The source document model cannot represent this path.
    UnsupportedPath,
    /// The entry cannot be read as ordinary source.
    UnsupportedKind,
    /// The input changed during capture.
    Changing,
}

/// Verified byte identity, independently of retained text and language analysis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum EntryContent {
    /// Digest and length supplied by verified acquisition, even without a text body.
    Verified {
        /// Identity of raw bytes or canonical reference/target bytes.
        id: ContentId,
        /// Number of hashed bytes.
        bytes: usize,
    },
    /// Known entry whose bytes could not be assessed.
    Unavailable(Unavailability),
}

impl EntryContent {
    /// Computes the same identity as captured source, without retaining its body.
    pub fn verified(bytes: &[u8]) -> Self {
        Self::Verified {
            id: ContentId::derive(&[bytes]),
            bytes: bytes.len(),
        }
    }
}

/// An entry known to exist, or a candidate whose presence could not be established.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum EntryState {
    /// Presence and metadata were observed, separately from content availability.
    Present {
        /// Entry type and executable metadata.
        kind: EntryKind,
        /// Verified identity or explicit content gap.
        content: EntryContent,
    },
    /// Neither presence nor absence is established; never infer addition/deletion.
    Unobserved(Unavailability),
}

/// One path in a completely enumerated review scope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InventoryEntry {
    /// Lossless logical workspace path.
    pub path: ReviewPath,
    /// Presence, metadata and content evidence.
    pub state: EntryState,
}

/// Immutable inventory plus analysis/source documents for one review observation.
#[derive(Debug)]
pub struct Observation {
    id: ObservationId,
    snapshot: Arc<Snapshot>,
    policy: CapturePolicyId,
    entries: Vec<InventoryEntry>,
}

impl Observation {
    /// Validates a complete inventory against its captured documents.
    ///
    /// Acquisition must finish enumeration before calling this constructor. Omitted
    /// paths mean verified absence; unknown candidates require `Unobserved` entries.
    /// `policy` identifies review scope and acquisition/exclusion configuration in
    /// addition to the snapshot's analysis policy.
    pub fn from_inventory(
        snapshot: Arc<Snapshot>,
        policy: CapturePolicyId,
        mut entries: Vec<InventoryEntry>,
    ) -> Result<Self, ModelError> {
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        if entries.windows(2).any(|pair| pair[0].path == pair[1].path) {
            return Err(ModelError("duplicate review path".into()));
        }
        for document in snapshot.documents() {
            let found = entries
                .binary_search_by(|entry| entry.path.bytes().cmp(document.path().as_bytes()))
                .ok()
                .map(|index| &entries[index]);
            let valid = match found.map(|entry| &entry.state) {
                Some(EntryState::Present {
                    kind: EntryKind::Regular { .. },
                    content: EntryContent::Verified { id, bytes },
                }) => id == document.content() && *bytes == document.bytes(),
                _ => false,
            };
            if !valid {
                return Err(ModelError(
                    "review inventory does not match captured document".into(),
                ));
            }
        }
        let mut hash = blake3::Hasher::new();
        hash.update(b"review-observation-v1");
        hash.update(snapshot.id().as_str().as_bytes());
        serde_json::to_writer(&mut hash, &(&policy, &entries))
            .map_err(|error| ModelError(error.to_string()))?;
        let id = ObservationId::derive(&[hash.finalize().as_bytes()]);
        Ok(Self {
            id,
            snapshot,
            policy,
            entries,
        })
    }
    /// Returns the content/policy-bound observation identity.
    pub fn id(&self) -> &ObservationId {
        &self.id
    }
    /// Returns immutable captured analysis and source documents.
    pub fn snapshot(&self) -> &Arc<Snapshot> {
        &self.snapshot
    }
    /// Returns review-scope/acquisition compatibility identity.
    pub fn policy(&self) -> &CapturePolicyId {
        &self.policy
    }
    /// Returns all observed/candidate paths in byte order.
    pub fn inventory(&self) -> &[InventoryEntry] {
        &self.entries
    }

    fn find(&self, path: &ReviewPath) -> Option<&InventoryEntry> {
        self.entries
            .binary_search_by(|entry| entry.path.cmp(path))
            .ok()
            .map(|index| &self.entries[index])
    }
}

/// Which immutable observation supplies source and context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Before source.
    Base,
    /// Observed after source.
    Working,
}

/// A comparison result for bytes or entry metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difference {
    /// Both values were verified equal.
    Equal,
    /// Both values were verified different.
    Different,
    /// At least one value was unavailable.
    Unknown,
    /// One side is absent, so no value comparison applies.
    NotApplicable,
}

/// Path-level change, without a semantic or rename inference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    /// Verified presence only on the working side.
    Added,
    /// Verified presence only on the base side.
    Removed,
    /// Content or metadata is verified different; other dimensions may be unknown.
    Modified,
    /// Content and metadata are verified equal.
    Unchanged,
    /// Available evidence cannot establish change or equality.
    Unassessed,
}

/// Borrowed change evidence, retaining each side's availability reasons.
#[derive(Debug)]
pub struct ComparedEntry<'a> {
    /// Logical path in either observation.
    pub path: &'a ReviewPath,
    /// Before entry, or verified absence.
    pub base: Option<&'a EntryState>,
    /// After entry, or verified absence.
    pub working: Option<&'a EntryState>,
    /// Overall path status.
    pub change: ChangeKind,
    /// Raw content comparison, separate from analysis coverage.
    pub content: Difference,
    /// Entry type/executable comparison.
    pub metadata: Difference,
}

/// An ordered immutable comparison with no native or SCM dependency.
#[derive(Debug)]
pub struct Comparison {
    id: ComparisonId,
    base: Arc<Observation>,
    working: Arc<Observation>,
}

impl Comparison {
    /// Rejects incompatible logical workspaces or capture/analysis policies.
    pub fn between(base: Arc<Observation>, working: Arc<Observation>) -> Result<Self, ModelError> {
        if base.snapshot.workspace() != working.snapshot.workspace()
            || base.policy != working.policy
            || base.snapshot.policy() != working.snapshot.policy()
            || base.snapshot.capture_plan() != working.snapshot.capture_plan()
        {
            return Err(ModelError(
                "incompatible review workspace or capture/analysis policy".into(),
            ));
        }
        let id = ComparisonId::derive(&[
            b"inventory-comparison-v1",
            base.id.as_str().as_bytes(),
            working.id.as_str().as_bytes(),
        ]);
        Ok(Self { id, base, working })
    }
    /// Returns the ordered pair/policy identity, not a host publication token.
    pub fn id(&self) -> &ComparisonId {
        &self.id
    }
    /// Returns the requested immutable side.
    pub fn observation(&self, side: Side) -> &Arc<Observation> {
        match side {
            Side::Base => &self.base,
            Side::Working => &self.working,
        }
    }
    /// Merges inventories in path order, borrowing evidence without copying bodies.
    pub fn entries(&self) -> impl Iterator<Item = ComparedEntry<'_>> {
        let mut base = self.base.entries.iter().peekable();
        let mut working = self.working.entries.iter().peekable();
        std::iter::from_fn(move || {
            use std::cmp::Ordering;
            let (before, after) = match (base.peek(), working.peek()) {
                (Some(before), Some(after)) => match before.path.cmp(&after.path) {
                    Ordering::Less => (base.next(), None),
                    Ordering::Equal => (base.next(), working.next()),
                    Ordering::Greater => (None, working.next()),
                },
                (Some(_), None) => (base.next(), None),
                (None, Some(_)) => (None, working.next()),
                (None, None) => return None,
            };
            compared(before, after)
        })
    }
    /// Looks up one path; `None` means verified absent on both sides.
    pub fn entry(&self, path: &ReviewPath) -> Option<ComparedEntry<'_>> {
        compared(self.base.find(path), self.working.find(path))
    }
    /// Returns retained source only from the named side; never reads mutable input.
    pub fn document(&self, side: Side, path: &ReviewPath) -> Option<&Arc<Document>> {
        self.observation(side)
            .snapshot
            .file_by_path(path.document_path()?)
    }
}

fn compared<'a>(
    base: Option<&'a InventoryEntry>,
    working: Option<&'a InventoryEntry>,
) -> Option<ComparedEntry<'a>> {
    use ChangeKind::*;
    use Difference::*;
    let path = &base.or(working)?.path;
    let base = base.map(|entry| &entry.state);
    let working = working.map(|entry| &entry.state);
    let (change, content, metadata) = match (base, working) {
        (None, Some(EntryState::Present { .. })) => (Added, NotApplicable, NotApplicable),
        (Some(EntryState::Present { .. }), None) => (Removed, NotApplicable, NotApplicable),
        (
            Some(EntryState::Present {
                kind: before_kind,
                content: before_content,
            }),
            Some(EntryState::Present {
                kind: after_kind,
                content: after_content,
            }),
        ) => {
            let metadata = if before_kind == after_kind {
                Equal
            } else {
                Different
            };
            let content = match (before_content, after_content) {
                (
                    EntryContent::Verified {
                        id: before,
                        bytes: before_len,
                    },
                    EntryContent::Verified {
                        id: after,
                        bytes: after_len,
                    },
                ) => {
                    if before == after && before_len == after_len {
                        Equal
                    } else {
                        Different
                    }
                }
                _ => Unknown,
            };
            let change = match (content, metadata) {
                (Different, _) | (_, Different) => Modified,
                (Equal, Equal) => Unchanged,
                _ => Unassessed,
            };
            (change, content, metadata)
        }
        _ => (Unassessed, Unknown, Unknown),
    };
    Some(ComparedEntry {
        path,
        base,
        working,
        change,
        content,
        metadata,
    })
}
