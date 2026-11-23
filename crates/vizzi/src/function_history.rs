//! Bounded historical function reading; never publishes a live snapshot or review pair.
use crate::{
    companion::CompanionError,
    host::Host,
    review_source::{GitRepository, ReadEntry, ReviewInput},
    structural::StructureHost,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard},
};
use vizzi_core::{
    comparison::{EntryKind, ReviewPath},
    model::{CapturePolicyId, Coverage, Document, EntityId, Snapshot},
    modules::ModuleIndex,
    projection::WindowQuery,
    structure::{Structure, Symbol},
};

const FILE_BYTES: usize = 2 * 1024 * 1024;
const TOTAL_BYTES: usize = 32 * 1024 * 1024;
const SESSIONS: usize = 24;
const PREPARED_SELECTIONS: usize = 4;
const PREPARED_BYTES: usize = PREPARED_SELECTIONS * FILE_BYTES;
const HISTORY_REVISIONS: usize = 64;
mod captured_calls;
mod comparison;
mod cross_file;
mod lineage;
mod module_context;
mod revision;
pub use captured_calls::{CapturedOwner, CapturedQuery, CapturedRequest, FrozenSource};
pub use comparison::{CompareRequest, ComparisonWindow, HistoryBinding};
pub use lineage::LineageRequest;
pub use revision::{ConfirmRevision, SelectRevision};

/// Exact current-source binding from which history starts.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Current native observation.
    pub snapshot: String,
    /// Native repository file identity.
    pub file: String,
    /// Exact immutable document version.
    pub version: String,
    /// Unique native declaration tracking identity.
    pub tracking: String,
}
/// Select an accepted version, or discover the next older version.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// Native history-session identity.
    pub token: String,
    /// Zero-based version, ordered from newer to older.
    pub index: usize,
}
#[derive(Clone)]
struct Capture {
    file: FileCapture,
    symbol: Symbol,
    span: std::ops::Range<usize>,
}
#[derive(Clone)]
struct FileCapture {
    snapshot: Arc<Snapshot>,
    modules: Arc<ModuleIndex>,
}
impl FileCapture {
    fn structure(&self) -> Structure {
        Structure::with_modules(&self.snapshot, &self.modules)
    }

    fn document(&self) -> &Document {
        &self.snapshot.documents()[0]
    }

    fn matches(&self, snapshot: &str, file: &str, version: &str) -> bool {
        self.snapshot.id().as_str() == snapshot
            && self.document().id().as_str() == file
            && self.document().version().as_str() == version
    }
}
struct Declaration {
    snapshot: Arc<Snapshot>,
    tracking: EntityId,
    span: std::ops::Range<usize>,
}
impl Declaration {
    fn bind(self, modules: Arc<ModuleIndex>) -> Result<Capture, CompanionError> {
        let symbol = Structure::with_modules(&self.snapshot, &modules)
            .resolve(Some(self.tracking.as_str()), "")
            .cloned()
            .ok_or_else(|| unavailable("Captured function has no readable source binding"))?;
        Ok(Capture {
            file: FileCapture {
                snapshot: self.snapshot,
                modules,
            },
            symbol,
            span: self.span,
        })
    }
}
struct Version {
    commit: String,
    snapshot: String,
    modules: Arc<ModuleIndex>,
    location: Location,
}
#[derive(Clone)]
struct Location {
    path: String,
    key: String,
    classification: vizzi_core::policy::Classification,
}
struct Session {
    token: String,
    repository: GitRepository,
    location: Location,
    language: String,
    tip: String,
    commits: Vec<String>,
    boundary: String,
    stopped: bool,
    cursor: usize,
    explored: usize,
    versions: Vec<Version>,
    lineage: Vec<lineage::Edge>,
    digest: blake3::Hash,
    selected: Option<Capture>,
    index: usize,
    files: cross_file::AdditionalFiles,
}

#[derive(Default)]
struct Sessions {
    retained: VecDeque<Session>,
    prepared: VecDeque<Session>,
}

impl Sessions {
    fn get(&self, token: &str) -> Option<&Session> {
        self.retained
            .iter()
            .chain(&self.prepared)
            .find(|session| session.token == token)
    }

    fn prepare(&mut self, session: Session) {
        while self.prepared.len() >= PREPARED_SELECTIONS
            || self.prepared.iter().map(size).sum::<usize>() + size(&session) > PREPARED_BYTES
        {
            self.prepared.pop_front();
        }
        self.prepared.push_back(session);
    }
}
fn invalid(message: impl Into<String>) -> CompanionError {
    CompanionError::Invalid(message.into())
}
fn unavailable(message: impl std::fmt::Display) -> CompanionError {
    CompanionError::Unavailable(message.to_string())
}
fn conflict(message: &str) -> CompanionError {
    CompanionError::Conflict(message.into())
}
fn opaque(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Owns independently expiring function-history sessions and their selected files.
pub struct FunctionHistory {
    host: Arc<Host>,
    structure: Arc<StructureHost>,
    sessions: Mutex<Sessions>,
    comparisons: Mutex<VecDeque<comparison::Pair>>,
}
impl FunctionHistory {
    /// Shares the host's source identity, analysis policy and worker budget.
    pub fn new(host: Arc<Host>) -> Self {
        let structure = Arc::new(StructureHost::new(host.clone()));
        Self::with_structure(host, structure)
    }

    pub(crate) fn with_structure(host: Arc<Host>, structure: Arc<StructureHost>) -> Self {
        Self {
            host,
            structure,
            sessions: Mutex::default(),
            comparisons: Mutex::new(VecDeque::new()),
        }
    }
    fn lock(&self) -> Result<MutexGuard<'_, Sessions>, CompanionError> {
        self.sessions
            .try_lock()
            .map_err(|_| unavailable("Function history is busy; retry"))
    }
    /// Freezes local HEAD and prepares the first distinct earlier function version.
    pub fn start(&self, request: &Request) -> Result<Value, CompanionError> {
        let mut sessions = self.lock()?;
        let doc = self.observed_document(request, false)?;
        let mut session = self.session(&doc, &request.tracking, None)?;
        self.advance(&mut session)?;
        self.verify_current(&request.snapshot)?;
        let answer = frame(&session)?;
        admit(&mut sessions.retained, session);
        Ok(answer)
    }

    fn observed_document(
        &self,
        request: &Request,
        pinned: bool,
    ) -> Result<Arc<Document>, CompanionError> {
        if ![
            &request.snapshot,
            &request.file,
            &request.version,
            &request.tracking,
        ]
        .into_iter()
        .all(|s| opaque(s))
        {
            return Err(invalid("Invalid function history binding"));
        }
        let app = self.host.application();
        let snapshot = if pinned {
            app.pinned(&request.snapshot)
                .ok_or_else(|| conflict("Pinned source expired; follow live source"))?
        } else {
            app.snapshot()
                .ok_or_else(|| unavailable("Wait for current source"))?
        };
        if snapshot.id().as_str() != request.snapshot {
            return Err(conflict("Source changed before function history opened"));
        }
        snapshot
            .file(&request.file)
            .filter(|d| d.version().as_str() == request.version)
            .cloned()
            .ok_or_else(|| conflict("Function history source version differs"))
    }

    fn verify_current(&self, snapshot: &str) -> Result<(), CompanionError> {
        if self
            .host
            .application()
            .snapshot()
            .is_some_and(|s| s.id().as_str() == snapshot)
        {
            Ok(())
        } else {
            Err(conflict(
                "Source changed while function history was opening",
            ))
        }
    }

    fn session(
        &self,
        doc: &Document,
        tracking: &str,
        revision: Option<&str>,
    ) -> Result<Session, CompanionError> {
        if doc.bytes() > FILE_BYTES {
            return Err(unavailable("Function history file exceeds 2 MiB"));
        }
        let mut matches = doc.analyses().iter().flat_map(|a| {
            a.facts
                .iter()
                .filter(|f| {
                    f.unique && f.kind == "function" && doc.entity_id(a, f).as_str() == tracking
                })
                .map(move |f| (a, f))
        });
        let (analysis, fact) = matches
            .next()
            .ok_or_else(|| invalid("History needs an unambiguous function"))?;
        if matches.next().is_some() {
            return Err(invalid("Function history declaration is ambiguous"));
        }
        let digest = blake3::hash(
            &doc.captured()
                .reader()
                .read(fact.span.clone(), FILE_BYTES)
                .map_err(unavailable)?,
        );
        let repository = GitRepository::open(self.host.root()).map_err(|e| {
            unavailable(format!(
                "Function history requires a local Git worktree: {e}"
            ))
        })?;
        let tip = repository
            .resolve_commit(revision.unwrap_or("HEAD"))
            .map_err(|_| {
                if revision.is_some() {
                    invalid("That Git commit or ref is not available locally")
                } else {
                    unavailable("No local HEAD commit is available for function history")
                }
            })?;
        let (commits, boundary) =
            segment_history(&repository, &tip, doc.path(), HISTORY_REVISIONS)?;
        let mut random = [0; 32];
        getrandom::fill(&mut random).map_err(unavailable)?;
        let token = random.iter().map(|b| format!("{b:02x}")).collect();
        Ok(Session {
            token,
            repository,
            location: Location {
                path: doc.path().into(),
                key: fact.key.clone(),
                classification: doc.classification().clone(),
            },
            language: analysis.context.language.clone(),
            tip,
            commits,
            boundary,
            stopped: false,
            cursor: 0,
            explored: 0,
            versions: vec![],
            lineage: vec![],
            digest,
            selected: None,
            index: 0,
            files: cross_file::AdditionalFiles::default(),
        })
    }
    /// Revisits immutable commits or lazily discovers another distinct declaration.
    pub fn step(&self, request: &Step) -> Result<Value, CompanionError> {
        let mut sessions = self.lock()?;
        let retained = take_session(&mut sessions.retained, &request.token);
        let was_retained = retained.is_some();
        let mut session = retained
            .or_else(|| take_session(&mut sessions.prepared, &request.token))
            .ok_or_else(|| conflict("Function history expired; return to live"))?;
        let result = (|| {
            if request.index > session.versions.len() {
                return Err(invalid("Invalid function history position"));
            }
            if request.index == session.versions.len() {
                self.advance(&mut session)?;
            } else {
                let capture = self.restore(&session, request.index)?;
                session.selected = Some(capture);
                session.index = request.index;
            }
            frame(&session)
        })();
        if was_retained {
            admit(&mut sessions.retained, session);
        } else {
            sessions.prepare(session);
        }
        result
    }
    /// Pages an owned immutable version; never falls back to current bytes.
    pub fn window(
        &self,
        token: &str,
        snapshot: &str,
        file: &str,
        version: &str,
        query: &WindowQuery,
    ) -> Result<Value, CompanionError> {
        let sessions = self.lock()?;
        let session = sessions
            .get(token)
            .ok_or_else(|| conflict("Function history expired; return to live"))?;
        let restored;
        let capture = if let Some(extra) = session.files.find(None, snapshot) {
            extra
        } else if let Some(capture) = session
            .selected
            .as_ref()
            .filter(|c| c.file.snapshot.id().as_str() == snapshot)
        {
            &capture.file
        } else {
            let index = session
                .versions
                .iter()
                .position(|v| v.snapshot == snapshot)
                .ok_or_else(|| conflict("Function history source version differs"))?;
            restored = self.restore(session, index)?;
            &restored.file
        };
        let doc = capture
            .snapshot
            .file(file)
            .filter(|d| {
                d.version().as_str() == version && capture.snapshot.id().as_str() == snapshot
            })
            .ok_or_else(|| conflict("Function history source version differs"))?;
        window(&capture.snapshot, doc, query)
    }
    fn restore(&self, session: &Session, index: usize) -> Result<Capture, CompanionError> {
        if let Some(capture) = &session.selected
            && session.index == index
        {
            return Ok(capture.clone());
        }
        let version = &session.versions[index];
        let (declaration, _) = self.declaration_at(session, &version.location, &version.commit)?;
        if declaration.snapshot.id().as_str() != version.snapshot {
            return Err(conflict(
                "Historical source differs from the accepted capture",
            ));
        }
        declaration.bind(version.modules.clone())
    }

    fn declaration(
        &self,
        session: &Session,
        commit: &str,
    ) -> Result<(Declaration, blake3::Hash), CompanionError> {
        self.declaration_at(session, &session.location, commit)
    }

    fn declaration_at(
        &self,
        session: &Session,
        location: &Location,
        commit: &str,
    ) -> Result<(Declaration, blake3::Hash), CompanionError> {
        let snapshot = self.analyzed_revision_file(&session.repository, commit, &location.path, location.classification.clone(), FILE_BYTES)?
            .ok_or_else(|| invalid("Function unavailable at this revision; history stops at a deletion or file move"))?;
        let doc = snapshot
            .file_by_path(&location.path)
            .ok_or_else(|| unavailable("Historical source is unavailable"))?;
        if !doc.analyses().iter().any(|a| {
            a.context.language == session.language
                && matches!(a.coverage, Coverage::Syntax | Coverage::Partial)
        }) {
            return Err(unavailable(
                "Historical function analysis is unavailable; retry",
            ));
        }
        let mut matches = doc
            .analyses()
            .iter()
            .filter(|a| a.context.language == session.language)
            .flat_map(|a| {
                a.facts
                    .iter()
                    .filter(|f| f.unique && f.key == location.key && f.kind == "function")
                    .map(move |f| (a, f))
            });
        let (analysis,fact) = matches.next().ok_or_else(|| invalid("Function missing or ambiguous at this revision; history stops before a rename, move or ambiguous declaration"))?;
        if matches.next().is_some() {
            return Err(invalid("Function is ambiguous at this revision"));
        }
        let bytes = doc
            .captured()
            .reader()
            .read(fact.span.clone(), FILE_BYTES)
            .map_err(unavailable)?;
        let digest = blake3::hash(&bytes);
        let tracking = doc.entity_id(analysis, fact);
        let span = fact.span.clone();
        Ok((
            Declaration {
                snapshot,
                tracking,
                span,
            },
            digest,
        ))
    }

    fn analyzed_revision_file(
        &self,
        repository: &GitRepository,
        commit: &str,
        path: &str,
        classification: vizzi_core::policy::Classification,
        max_bytes: usize,
    ) -> Result<Option<Arc<Snapshot>>, CompanionError> {
        let relative = ReviewPath::parse(path.as_bytes().to_vec())
            .map_err(|error| invalid(error.to_string()))?;
        let mut input = repository.pin_file(commit, path).map_err(unavailable)?;
        let bytes = match input
            .read(&relative, max_bytes, &|| false)
            .map_err(unavailable)?
        {
            ReadEntry::Present {
                kind: EntryKind::Regular { .. },
                bytes,
            } => bytes,
            ReadEntry::Absent | ReadEntry::Present { .. } => return Ok(None),
            ReadEntry::Unobserved(_) | ReadEntry::Unavailable { .. } => {
                return Err(unavailable(format!(
                    "Could not read changed source {path} at {commit}"
                )));
            }
        };
        let doc = Document::capture_source(
            repository.workspace().clone(),
            path.into(),
            vizzi_core::model::CapturedSource::capture(bytes, None),
            vec![],
            classification,
        )
        .map_err(unavailable)?;
        let source = Snapshot::new(
            repository.workspace().clone(),
            vec![Arc::new(doc)],
            CapturePolicyId::derive(&[b"function-history-source-v1"]),
        )
        .map_err(unavailable)?;
        let analyzed = self.host.analyze_history(&source).map_err(unavailable)?;
        let snapshot = Snapshot::new(
            analyzed.workspace().clone(),
            analyzed.documents().to_vec(),
            CapturePolicyId::derive(&[
                b"function-history-revision-v1",
                analyzed.policy().as_str().as_bytes(),
                commit.as_bytes(),
            ]),
        )
        .map_err(unavailable)?;
        Ok(Some(Arc::new(snapshot)))
    }
    fn advance(&self, session: &mut Session) -> Result<(), CompanionError> {
        if session.versions.len() == HISTORY_REVISIONS {
            return Err(unavailable(
                "Function history limit reached (64 file revisions)",
            ));
        }
        if session.stopped {
            return Err(unavailable(&session.boundary));
        }
        let mut cursor = session.cursor;
        let mut chosen: Option<(usize, Declaration, blake3::Hash)> = None;
        while cursor < session.commits.len() {
            let (declaration, digest) = match self.declaration(session, &session.commits[cursor]) {
                Ok(value) => value,
                Err(CompanionError::Invalid(message)) => {
                    session.boundary = message;
                    session.stopped = true;
                    break;
                }
                Err(error) => return Err(error),
            };
            if let Some((_, _, expected)) = &chosen {
                if digest != *expected {
                    break;
                }
            } else if digest == session.digest {
                cursor += 1;
                continue;
            }
            chosen = Some((cursor, declaration, digest));
            cursor += 1;
        }
        if let Some((candidate, declaration, digest)) = chosen {
            let commit = session.commits[candidate].clone();
            self.accept(session, commit, declaration, digest)?;
            session.cursor = cursor;
            return Ok(());
        }
        session.stopped = true;
        Err(unavailable(&session.boundary))
    }

    fn accept(
        &self,
        session: &mut Session,
        commit: String,
        declaration: Declaration,
        digest: blake3::Hash,
    ) -> Result<(), CompanionError> {
        let (version, capture) =
            self.history_capture(session, session.location.clone(), commit, declaration)?;
        session.push(version, capture, digest);
        Ok(())
    }

    fn history_capture(
        &self,
        session: &Session,
        location: Location,
        commit: String,
        declaration: Declaration,
    ) -> Result<(Version, Capture), CompanionError> {
        let modules = Arc::new(module_context::for_commit(
            &session.repository,
            &commit,
            self.host.source_policy(),
            &declaration.snapshot.documents()[0],
            &session.language,
        ));
        let capture = declaration.bind(modules.clone())?;
        let version = Version {
            commit,
            snapshot: capture.file.snapshot.id().as_str().into(),
            modules,
            location,
        };
        Ok((version, capture))
    }
}

impl Session {
    fn push(&mut self, version: Version, capture: Capture, digest: blake3::Hash) {
        self.digest = digest;
        self.index = self.versions.len();
        self.versions.push(version);
        self.selected = Some(capture);
    }

    fn metadata(&self, index: usize) -> Value {
        json!({"token":self.token,"tip":self.tip,"commit":self.versions[index].commit,"index":index,"lineage":self.lineage})
    }
}

fn segment_history(
    repository: &GitRepository,
    tip: &str,
    path: &str,
    limit: usize,
) -> Result<(Vec<String>, String), CompanionError> {
    let (mut commits, shallow) = repository.file_history(tip, path).map_err(unavailable)?;
    let boundary = if commits.len() > limit {
        commits.truncate(limit);
        "Function history limit reached (64 file revisions)"
    } else if shallow {
        "Earlier history is unavailable in this shallow checkout"
    } else {
        "Beginning of available function history"
    };
    Ok((commits, boundary.into()))
}

fn take_session(queue: &mut VecDeque<Session>, token: &str) -> Option<Session> {
    let position = queue.iter().position(|session| session.token == token)?;
    queue.remove(position)
}

fn size(session: &Session) -> usize {
    session
        .selected
        .as_ref()
        .map_or(0, |c| c.file.document().bytes())
        + session.files.bytes()
}
fn admit(sessions: &mut VecDeque<Session>, session: Session) {
    while sessions.len() >= SESSIONS
        || sessions.iter().map(size).sum::<usize>() + size(&session) > TOTAL_BYTES
    {
        if sessions.pop_front().is_none() {
            break;
        }
    }
    sessions.push_back(session);
}
fn window(
    snapshot: &Snapshot,
    doc: &Document,
    query: &WindowQuery,
) -> Result<Value, CompanionError> {
    let mut window =
        serde_json::to_value(doc.window(query, None).map_err(unavailable)?).map_err(unavailable)?;
    window["snapshot"] = json!(snapshot.id().as_str());
    Ok(window)
}
fn frame(session: &Session) -> Result<Value, CompanionError> {
    let capture = session
        .selected
        .as_ref()
        .ok_or_else(|| unavailable("No earlier function change is available"))?;
    let s = &capture.symbol.source;
    let capture_file = &capture.file;
    let doc = capture_file
        .snapshot
        .file(&s.file)
        .ok_or_else(|| unavailable("Historical source is unavailable"))?;
    let query = WindowQuery {
        line_start: Some(s.line),
        line_count: Some(96.min(s.end_line - s.line + 1)),
        outline_count: Some(1),
        reflow: Some(true),
        ..WindowQuery::default()
    };
    let source = window(&capture_file.snapshot, doc, &query)?;
    let mut result = session.metadata(session.index);
    result["snapshot"] = json!(capture_file.snapshot.id().as_str());
    result["symbol"] = json!(capture.symbol);
    result["origin"] = source["lines"][0].clone();
    result["window"] = source;
    Ok(result)
}
