//! Native evidence acquisition and durable investigation state, separate from source ownership.
use crate::{
    editor::{Editor, EditorError, EditorLocation},
    host::Host,
    review_source::{DirectoryInput, ReadEntry, ReviewInput},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};
use vizzi_core::{
    comparison::{EntryKind, ReviewPath},
    investigation::*,
    model::{ContentId, FileId, Snapshot},
    projection::WindowQuery,
};
/// Native application errors with distinct recovery meanings.
#[derive(Debug, thiserror::Error)]
pub enum CompanionError {
    /// Invalid bounded input.
    #[error("{0}")]
    Invalid(String),
    /// Obsolete edit/source binding.
    #[error("{0}")]
    Conflict(String),
    /// Source/index cannot currently supply evidence.
    #[error("{0}")]
    Unavailable(String),
    /// Persistent data could not be read or written.
    #[error("{0}")]
    Storage(String),
    /// Editor adapter refused the launch.
    #[error(transparent)]
    Editor(#[from] EditorError),
}
impl From<InvestigationError> for CompanionError {
    fn from(e: InvestigationError) -> Self {
        Self::Invalid(e.to_string())
    }
}
/// A current source match for explicit pinning.
#[derive(Serialize)]
pub struct SourceMatch {
    /// Snapshot used to resolve this match.
    pub snapshot: String,
    /// Exact file identity.
    pub file: String,
    /// Exact document version.
    pub version: String,
    /// Relative source path.
    pub path: String,
    /// Available captured source lines.
    pub lines: usize,
}
/// Explicit bounded source pin request; source bytes are never supplied by a caller.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinEvidence {
    /// Expected investigation revision.
    pub revision: u32,
    /// Captured snapshot identity.
    pub snapshot: String,
    /// File identity within that snapshot.
    pub file: String,
    /// Required captured document version.
    pub version: String,
    /// One-based first evidence line.
    pub line: usize,
    /// Reader-assigned title.
    pub title: String,
}
/// Reader-only edits, separated from source acquisition.
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum EditInvestigation {
    /// Replace the investigation question.
    Question {
        /// Expected revision.
        revision: u32,
        /// Reader question.
        text: String,
    },
    /// Replace a card's reader note.
    Annotate {
        /// Expected revision.
        revision: u32,
        /// Card identity.
        id: u32,
        /// Reader note.
        note: String,
    },
    /// Add a reader-authored connection.
    Connect {
        /// Expected revision.
        revision: u32,
        /// Origin card.
        from: u32,
        /// Related card.
        to: u32,
        /// Reader assertion.
        explanation: String,
    },
    /// Remove one reader-authored connection.
    Disconnect {
        /// Expected revision.
        revision: u32,
        /// Origin card.
        from: u32,
        /// Related card.
        to: u32,
    },
    /// Remove a card and its assertions.
    Remove {
        /// Expected revision.
        revision: u32,
        /// Card identity.
        id: u32,
    },
}
/// Freshness of saved source relative to the current complete index.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceState {
    /// Matching captured content is indexed.
    Current,
    /// Indexed content differs.
    Changed,
    /// Indexed path is absent.
    Missing,
    /// No complete live index is available.
    Unchecked,
}
/// One bounded investigation projection.
#[derive(Serialize)]
pub struct CompanionView {
    /// Validated portable board data.
    pub board: InvestigationData,
    /// Whether an external persistent file is configured.
    pub persistent: bool,
    /// Selected native editor adapter name.
    pub editor: String,
    /// Per-card current-index freshness, not a new source observation.
    pub freshness: Vec<(u32, SourceState)>,
}
/// One workspace's question/evidence state and editor integration.
pub struct Companion {
    host: Arc<Host>,
    board: Mutex<Investigation>,
    file: Option<PathBuf>,
    editor: Arc<dyn Editor>,
}
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}
impl Companion {
    /// Loads a validated external file or starts a bounded session investigation.
    pub fn new(
        host: Arc<Host>,
        file: Option<PathBuf>,
        editor: Arc<dyn Editor>,
    ) -> Result<Self, CompanionError> {
        let workspace = DirectoryInput::open(host.root())
            .map_err(|e| CompanionError::Unavailable(e.to_string()))?
            .workspace()
            .clone();
        let file = file
            .map(|p| crate::cache::external_path(&p, host.root()))
            .transpose()
            .map_err(|e| CompanionError::Storage(e.to_string()))?;
        let board = if let Some(path) = &file {
            match File::open(path) {
                Ok(file) => {
                    let mut data = vec![];
                    file.take(ENCODED_BYTES as u64 + 1)
                        .read_to_end(&mut data)
                        .map_err(|e| CompanionError::Storage(e.to_string()))?;
                    Investigation::decode(&data)?
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    Investigation::new(workspace.clone())
                }
                Err(e) => return Err(CompanionError::Storage(e.to_string())),
            }
        } else {
            Investigation::new(workspace.clone())
        };
        if board.data().workspace != workspace {
            return Err(CompanionError::Conflict(
                "Investigation belongs to another workspace".into(),
            ));
        }
        Ok(Self {
            host,
            board: Mutex::new(board),
            file,
            editor,
        })
    }
    fn snapshot(&self) -> Result<Arc<Snapshot>, CompanionError> {
        self.host.application().snapshot().ok_or_else(|| {
            CompanionError::Unavailable(
                "Start the source index before adding or opening evidence".into(),
            )
        })
    }
    /// Orients an unfamiliar reader using only captured paths and lexical metadata.
    pub fn discover(
        &self,
        query: &vizzi_core::discovery::DiscoveryQuery,
    ) -> Result<vizzi_core::discovery::Discovery, CompanionError> {
        self.snapshot()?
            .discover(query)
            .map_err(|e| CompanionError::Invalid(e.to_string()))
    }
    fn save(&self, board: &Investigation) -> Result<(), CompanionError> {
        let data = board.encode()?;
        if let Some(path) = &self.file {
            let parent = path.parent().ok_or_else(|| {
                CompanionError::Storage("Investigation file needs a parent directory".into())
            })?;
            std::fs::create_dir_all(parent).map_err(|e| CompanionError::Storage(e.to_string()))?;
            let mut file = tempfile::NamedTempFile::new_in(parent)
                .map_err(|e| CompanionError::Storage(e.to_string()))?;
            file.write_all(&data)
                .and_then(|_| file.as_file().sync_all())
                .map_err(|e| CompanionError::Storage(e.to_string()))?;
            file.persist(path)
                .map_err(|e| CompanionError::Storage(e.to_string()))?;
        }
        Ok(())
    }
    fn update<T>(
        &self,
        revision: u32,
        change: impl FnOnce(&mut Investigation) -> Result<T, CompanionError>,
    ) -> Result<T, CompanionError> {
        let mut current = lock(&self.board);
        if current.data().revision != revision {
            return Err(CompanionError::Conflict(
                "Investigation changed; reload before editing again".into(),
            ));
        }
        let mut next = current.clone();
        let result = change(&mut next)?;
        self.save(&next)?;
        *current = next;
        Ok(result)
    }
    /// Returns saved interpretation and separate current-source freshness.
    pub fn view(&self) -> CompanionView {
        let board = lock(&self.board).data().clone();
        let snapshot = self.host.application().snapshot();
        let freshness = board
            .evidence
            .iter()
            .map(|e| {
                (
                    e.id,
                    match &snapshot {
                        None => SourceState::Unchecked,
                        Some(s) => match s.file(e.source.file.as_str()) {
                            None => SourceState::Missing,
                            Some(d) if d.content() == &e.source.content => SourceState::Current,
                            Some(_) => SourceState::Changed,
                        },
                    },
                )
            })
            .collect();
        CompanionView {
            board,
            persistent: self.file.is_some(),
            editor: self.editor.name().into(),
            freshness,
        }
    }
    /// Resolves a normalised path through the current immutable index.
    pub fn lookup(&self, path: &str) -> Result<SourceMatch, CompanionError> {
        ReviewPath::parse(path.as_bytes().to_vec())
            .map_err(|e| CompanionError::Invalid(e.to_string()))?;
        let snapshot = self.snapshot()?;
        let id = FileId::derive(&[snapshot.workspace().as_str().as_bytes(), path.as_bytes()]);
        let doc = snapshot.file(id.as_str()).ok_or_else(|| {
            CompanionError::Unavailable("Path is not in the current source index".into())
        })?;
        Ok(SourceMatch {
            snapshot: snapshot.id().as_str().into(),
            file: id.as_str().into(),
            version: doc.version().as_str().into(),
            path: doc.path().into(),
            lines: doc.captured().line_count(),
        })
    }
    /// Keeps a search location only while its observation is current.
    pub fn pin_current(&self, request: PinEvidence) -> Result<u32, CompanionError> {
        if self.snapshot()?.id().as_str() != request.snapshot {
            return Err(CompanionError::Conflict(
                "Source changed; search again before keeping this location".into(),
            ));
        }
        self.pin(request)
    }
    /// Captures a finite literal excerpt at the requested exact observation/version.
    pub fn pin(&self, request: PinEvidence) -> Result<u32, CompanionError> {
        let snapshot = self
            .host
            .application()
            .pinned(&request.snapshot)
            .ok_or_else(|| {
                CompanionError::Conflict(
                    "Source observation expired; look up the location again".into(),
                )
            })?;
        let doc = snapshot.file(&request.file).ok_or_else(|| {
            CompanionError::Unavailable("File is absent from that observation".into())
        })?;
        if doc.version().as_str() != request.version {
            return Err(CompanionError::Conflict(
                "Source document version differs".into(),
            ));
        }
        if request.line == 0 || request.line > doc.captured().line_count() {
            return Err(CompanionError::Invalid(
                "Evidence line is outside the captured text".into(),
            ));
        }
        let window = doc
            .window(
                &WindowQuery {
                    line_start: Some(request.line),
                    line_count: Some(6),
                    outline_count: Some(1),
                    ..Default::default()
                },
                None,
            )
            .map_err(|e| CompanionError::Unavailable(e.to_string()))?;
        let mut excerpt = String::new();
        let mut end = request.line;
        let mut truncated = false;
        for row in window.lines {
            let allowance = EXCERPT_BYTES.saturating_sub(excerpt.len());
            let mut count = row.text.len().min(allowance);
            while !row.text.is_char_boundary(count) {
                count -= 1;
            }
            excerpt.push_str(&row.text[..count]);
            end = row.number;
            if count < row.text.len() || row.right {
                truncated = true;
                break;
            }
            if excerpt.len() < EXCERPT_BYTES {
                excerpt.push('\n');
            } else {
                truncated = true;
                break;
            }
        }
        truncated |= end < doc.captured().line_count();
        let source = SourceEvidence {
            workspace: snapshot.workspace().clone(),
            snapshot: snapshot.id().clone(),
            file: doc.id().clone(),
            version: doc.version().clone(),
            content: doc.content().clone(),
            path: doc.path().into(),
            line: request.line,
            end_line: end,
            excerpt,
            truncated,
        };
        self.update(request.revision, |b| Ok(b.add(request.title, source)?))
    }
    /// Applies a reader edit without acquiring source or changing anchors.
    pub fn edit(&self, edit: EditInvestigation) -> Result<(), CompanionError> {
        match edit {
            EditInvestigation::Question { revision, text } => {
                self.update(revision, |b| Ok(b.question(text)?))
            }
            EditInvestigation::Annotate { revision, id, note } => {
                self.update(revision, |b| Ok(b.annotate(id, note)?))
            }
            EditInvestigation::Connect {
                revision,
                from,
                to,
                explanation,
            } => self.update(revision, |b| Ok(b.connect(from, to, explanation)?)),
            EditInvestigation::Disconnect { revision, from, to } => {
                self.update(revision, |b| Ok(b.disconnect(from, to)?))
            }
            EditInvestigation::Remove { revision, id } => {
                self.update(revision, |b| Ok(b.remove(id)?))
            }
        }
    }
    /// Verifies current captured and working bytes before requesting a local editor location.
    pub fn open(&self, id: u32) -> Result<(), CompanionError> {
        let evidence = lock(&self.board)
            .evidence(id)
            .cloned()
            .ok_or_else(|| CompanionError::Invalid("Evidence is absent".into()))?;
        let snapshot = self.snapshot()?;
        let doc = snapshot
            .file(evidence.source.file.as_str())
            .ok_or_else(|| {
                CompanionError::Conflict("Saved source is absent from the current index".into())
            })?;
        if evidence.source.line > doc.captured().line_count()
            || doc.content() != &evidence.source.content
            || doc.path() != evidence.source.path
            || snapshot.workspace() != &evidence.source.workspace
        {
            return Err(CompanionError::Conflict("Saved source has changed. Keep the captured excerpt or add fresh evidence; the editor location was not opened.".into()));
        }
        let path = ReviewPath::parse(evidence.source.path.as_bytes().to_vec())
            .map_err(|e| CompanionError::Invalid(e.to_string()))?;
        let mut input = DirectoryInput::open(self.host.root())
            .map_err(|e| CompanionError::Unavailable(e.to_string()))?;
        let entry = input
            .read(&path, 16 * 1024 * 1024, &|| false)
            .map_err(|e| CompanionError::Unavailable(e.to_string()))?;
        match entry {
            ReadEntry::Present {
                kind: EntryKind::Regular { .. },
                bytes,
            } if ContentId::derive(&[&bytes]) == evidence.source.content => {}
            _ => return Err(CompanionError::Conflict(
                "Working source differs or cannot be verified; the saved location was not opened"
                    .into(),
            )),
        }
        self.editor.open(&EditorLocation {
            path: self.host.root().join(doc.path()),
            line: evidence.source.line,
        })?;
        Ok(())
    }
}
