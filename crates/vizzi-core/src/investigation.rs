//! Bounded reader-authored investigations over immutable source anchors.
use crate::model::{ContentId, DocumentVersion, FileId, SnapshotId, WorkspaceId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
/// Maximum source cards in one investigation.
pub const EVIDENCE_LIMIT: usize = 32;
/// Maximum reader-authored connections.
pub const CONNECTION_LIMIT: usize = 64;
/// Maximum literal captured excerpt bytes per card.
pub const EXCERPT_BYTES: usize = 4096;
/// Maximum encoded investigation accepted from storage.
pub const ENCODED_BYTES: usize = 512 * 1024;
/// Exact captured source location and a finite literal copy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEvidence {
    /// Logical workspace namespace.
    pub workspace: WorkspaceId,
    /// Originating immutable observation.
    pub snapshot: SnapshotId,
    /// Workspace/path identity.
    pub file: FileId,
    /// Captured source/analysis version.
    pub version: DocumentVersion,
    /// Complete captured byte identity.
    pub content: ContentId,
    /// Normalised workspace-relative display path.
    pub path: String,
    /// One-based first source line.
    pub line: usize,
    /// Last line represented by the excerpt.
    pub end_line: usize,
    /// Literal UTF-8 source excerpt, without HTML interpretation.
    pub excerpt: String,
    /// Additional source exists beyond the stored excerpt.
    pub truncated: bool,
}
/// A source card with reader-supplied interpretation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// Stable local card identity, never reused after removal.
    pub id: u32,
    /// Reader-supplied role/title.
    pub title: String,
    /// Reader-supplied interpretation, separate from source.
    pub note: String,
    /// Immutable source anchor and excerpt.
    pub source: SourceEvidence,
}
/// A reader assertion, never an inferred dependency or call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    /// Origin card.
    pub from: u32,
    /// Related card.
    pub to: u32,
    /// Reader-authored explanation.
    pub explanation: String,
}
/// Versioned, bounded portable storage representation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvestigationData {
    /// Storage schema version.
    pub schema: u8,
    /// Workspace owning every source anchor.
    pub workspace: WorkspaceId,
    /// Optimistic edit revision.
    pub revision: u32,
    /// Next unused source-card identity.
    pub next_id: u32,
    /// Question the reader is investigating.
    pub question: String,
    /// Source evidence in reader order.
    pub evidence: Vec<Evidence>,
    /// Explicit reader-authored relationships.
    pub connections: Vec<Connection>,
}
/// Refused mutation or invalid persisted data.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct InvestigationError(pub String);
/// A validated investigation; hosts own persistence and source acquisition.
#[derive(Clone, Debug)]
pub struct Investigation {
    data: InvestigationData,
}
fn require(ok: bool, message: &str) -> Result<(), InvestigationError> {
    if ok {
        Ok(())
    } else {
        Err(InvestigationError(message.into()))
    }
}
fn title(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 128
}
fn relative(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && !path.contains(['\\', '\0'])
        && path.split('/').all(|p| !matches!(p, "" | "." | ".."))
}
impl Investigation {
    /// Starts an empty question workspace without source owners or platform state.
    pub fn new(workspace: WorkspaceId) -> Self {
        Self {
            data: InvestigationData {
                schema: 1,
                workspace,
                revision: 0,
                next_id: 1,
                question: String::new(),
                evidence: vec![],
                connections: vec![],
            },
        }
    }
    /// Returns immutable presentation/storage data.
    pub fn data(&self) -> &InvestigationData {
        &self.data
    }
    /// Resolves one card without changing its source anchor.
    pub fn evidence(&self, id: u32) -> Option<&Evidence> {
        self.data.evidence.iter().find(|e| e.id == id)
    }
    fn update(
        &mut self,
        change: impl FnOnce(&mut InvestigationData),
    ) -> Result<(), InvestigationError> {
        let mut next = self.data.clone();
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or_else(|| InvestigationError("Investigation revision exhausted".into()))?;
        change(&mut next);
        let bytes = serde_json::to_vec(&next).map_err(|e| InvestigationError(e.to_string()))?;
        require(
            bytes.len() <= ENCODED_BYTES,
            "Investigation exceeds 512 KiB",
        )?;
        self.data = next;
        Ok(())
    }
    fn validate_source(&self, source: &SourceEvidence) -> Result<(), InvestigationError> {
        require(
            source.workspace == self.data.workspace
                && relative(&source.path)
                && source.line > 0
                && source.end_line >= source.line
                && source.excerpt.len() <= EXCERPT_BYTES,
            "Invalid source evidence or workspace",
        )
    }
    /// Replaces the question while preserving source evidence.
    pub fn question(&mut self, value: String) -> Result<(), InvestigationError> {
        require(value.len() <= 2048, "Question exceeds 2,048 UTF-8 bytes")?;
        self.update(|data| data.question = value)
    }
    /// Adds a verified source copy with a reader title.
    pub fn add(
        &mut self,
        value: String,
        source: SourceEvidence,
    ) -> Result<u32, InvestigationError> {
        require(
            self.data.evidence.len() < EVIDENCE_LIMIT && title(&value),
            "Evidence limit reached or invalid title",
        )?;
        self.validate_source(&source)?;
        let id = self.data.next_id;
        let next = id
            .checked_add(1)
            .ok_or_else(|| InvestigationError("Evidence identity exhausted".into()))?;
        self.update(|data| {
            data.evidence.push(Evidence {
                id,
                title: value,
                note: String::new(),
                source,
            });
            data.next_id = next;
        })?;
        Ok(id)
    }
    /// Edits the reader's note, never its captured source.
    pub fn annotate(&mut self, id: u32, note: String) -> Result<(), InvestigationError> {
        require(note.len() <= 2048, "Note exceeds 2,048 UTF-8 bytes")?;
        let index = self
            .data
            .evidence
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| InvestigationError("Evidence is absent".into()))?;
        self.update(|data| data.evidence[index].note = note)
    }
    /// Connects two existing cards with an explicit reader assertion.
    pub fn connect(
        &mut self,
        from: u32,
        to: u32,
        explanation: String,
    ) -> Result<(), InvestigationError> {
        require(
            self.data.connections.len() < CONNECTION_LIMIT
                && from != to
                && self.evidence(from).is_some()
                && self.evidence(to).is_some()
                && !explanation.trim().is_empty()
                && explanation.len() <= 512
                && !self
                    .data
                    .connections
                    .iter()
                    .any(|c| c.from == from && c.to == to),
            "Invalid, duplicate or excessive connection",
        )?;
        self.update(|data| {
            data.connections.push(Connection {
                from,
                to,
                explanation,
            })
        })
    }
    /// Removes a reader assertion without disturbing its source evidence.
    pub fn disconnect(&mut self, from: u32, to: u32) -> Result<(), InvestigationError> {
        require(
            self.data
                .connections
                .iter()
                .any(|c| c.from == from && c.to == to),
            "Connection is absent",
        )?;
        self.update(|data| data.connections.retain(|c| c.from != from || c.to != to))
    }
    /// Removes a card and its connections without reusing its identity.
    pub fn remove(&mut self, id: u32) -> Result<(), InvestigationError> {
        require(self.evidence(id).is_some(), "Evidence is absent")?;
        self.update(|data| {
            data.evidence.retain(|e| e.id != id);
            data.connections.retain(|c| c.from != id && c.to != id);
        })
    }
    /// Encodes validated state for an external store or export.
    pub fn encode(&self) -> Result<Vec<u8>, InvestigationError> {
        serde_json::to_vec(&self.data).map_err(|e| InvestigationError(e.to_string()))
    }
    /// Admits bounded persisted state before exposing an investigation.
    pub fn decode(bytes: &[u8]) -> Result<Self, InvestigationError> {
        require(
            bytes.len() <= ENCODED_BYTES,
            "Investigation file exceeds 512 KiB",
        )?;
        let data: InvestigationData =
            serde_json::from_slice(bytes).map_err(|e| InvestigationError(e.to_string()))?;
        let board = Self { data };
        let d = board.data();
        require(
            d.schema == 1
                && d.next_id > 0
                && d.question.len() <= 2048
                && d.evidence.len() <= EVIDENCE_LIMIT
                && d.connections.len() <= CONNECTION_LIMIT,
            "Unsupported or excessive investigation data",
        )?;
        let mut ids = BTreeSet::new();
        for e in &d.evidence {
            require(
                e.id > 0
                    && e.id < d.next_id
                    && ids.insert(e.id)
                    && title(&e.title)
                    && e.note.len() <= 2048,
                "Invalid evidence identity or text",
            )?;
            board.validate_source(&e.source)?;
        }
        let mut links = BTreeSet::new();
        for c in &d.connections {
            require(
                c.from != c.to
                    && ids.contains(&c.from)
                    && ids.contains(&c.to)
                    && links.insert((c.from, c.to))
                    && !c.explanation.trim().is_empty()
                    && c.explanation.len() <= 512,
                "Invalid connection",
            )?;
        }
        Ok(board)
    }
}
