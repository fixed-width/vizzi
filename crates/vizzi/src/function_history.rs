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
    modules::{ModuleCache, ModuleIndex},
    projection::WindowQuery,
    structure::{Structure, Symbol},
};

const FILE_BYTES: usize = 2 * 1024 * 1024;
const TOTAL_BYTES: usize = 32 * 1024 * 1024;
const SESSIONS: usize = 24;
mod captured_calls;
mod comparison;
mod module_context;
pub use captured_calls::{CapturedOwner, CapturedQuery, CapturedRequest, FrozenSource};
pub use comparison::{CompareRequest, ComparisonWindow, HistoryBinding};

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
    snapshot: Arc<Snapshot>,
    symbol: Symbol,
    span: std::ops::Range<usize>,
    modules: Arc<ModuleIndex>,
}
impl Capture {
    fn structure(&self) -> Structure {
        Structure::with_modules(&self.snapshot, &self.modules)
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
            snapshot: self.snapshot,
            symbol,
            span: self.span,
            modules,
        })
    }
}
struct Version {
    candidate: usize,
    snapshot: String,
    modules: Arc<ModuleIndex>,
}
struct Session {
    token: String,
    repository: GitRepository,
    path: String,
    key: String,
    language: String,
    classification: vizzi_core::policy::Classification,
    tip: String,
    commits: Vec<String>,
    boundary: String,
    stopped: bool,
    cursor: usize,
    versions: Vec<Version>,
    digest: blake3::Hash,
    selected: Option<Capture>,
    index: usize,
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
    sessions: Mutex<VecDeque<Session>>,
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
            sessions: Mutex::new(VecDeque::new()),
            comparisons: Mutex::new(VecDeque::new()),
        }
    }
    fn lock(&self) -> Result<MutexGuard<'_, VecDeque<Session>>, CompanionError> {
        self.sessions
            .try_lock()
            .map_err(|_| unavailable("Function history is busy; retry"))
    }
    /// Freezes local HEAD and prepares the first distinct earlier function version.
    pub fn start(&self, request: &Request) -> Result<Value, CompanionError> {
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
        let mut sessions = self.lock()?;
        let snapshot = self
            .host
            .application()
            .snapshot()
            .ok_or_else(|| unavailable("Wait for current source"))?;
        if snapshot.id().as_str() != request.snapshot {
            return Err(conflict("Source changed before function history opened"));
        }
        let doc = snapshot
            .file(&request.file)
            .filter(|d| d.version().as_str() == request.version)
            .ok_or_else(|| conflict("Function history source version differs"))?;
        if doc.bytes() > FILE_BYTES {
            return Err(unavailable("Function history file exceeds 2 MiB"));
        }
        let mut matches = doc.analyses().iter().flat_map(|a| {
            a.facts
                .iter()
                .filter(|f| {
                    f.unique
                        && f.kind == "function"
                        && doc.entity_id(a, f).as_str() == request.tracking
                })
                .map(move |f| (a, f))
        });
        let (analysis, fact) = matches
            .next()
            .ok_or_else(|| invalid("History needs an unambiguous current function"))?;
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
        let (tip, mut commits, shallow) =
            repository.file_history(doc.path()).map_err(unavailable)?;
        let boundary = if commits.len() > 64 {
            commits.truncate(64);
            "Function history limit reached (64 file revisions)"
        } else if shallow {
            "Earlier history is unavailable in this shallow checkout"
        } else {
            "Beginning of available function history"
        }
        .into();
        let mut random = [0; 32];
        getrandom::fill(&mut random).map_err(unavailable)?;
        let token = random.iter().map(|b| format!("{b:02x}")).collect();
        let mut session = Session {
            token,
            repository,
            path: doc.path().into(),
            key: fact.key.clone(),
            language: analysis.context.language.clone(),
            classification: doc.classification().clone(),
            tip,
            commits,
            boundary,
            stopped: false,
            cursor: 0,
            versions: vec![],
            digest,
            selected: None,
            index: 0,
        };
        self.advance(&mut session)?;
        if self
            .host
            .application()
            .snapshot()
            .is_none_or(|s| s.id() != snapshot.id())
        {
            return Err(conflict(
                "Source changed while function history was opening",
            ));
        }
        let answer = frame(&session)?;
        admit(&mut sessions, session);
        Ok(answer)
    }
    /// Revisits immutable commits or lazily discovers another distinct declaration.
    pub fn step(&self, request: &Step) -> Result<Value, CompanionError> {
        let mut sessions = self.lock()?;
        let position = sessions
            .iter()
            .position(|s| s.token == request.token)
            .ok_or_else(|| conflict("Function history expired; return to live"))?;
        let mut session = sessions
            .remove(position)
            .ok_or_else(|| conflict("Function history expired"))?;
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
        admit(&mut sessions, session);
        result
    }
    /// Pages an accepted immutable version; never falls back to current bytes.
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
            .iter()
            .find(|s| s.token == token)
            .ok_or_else(|| conflict("Function history expired; return to live"))?;
        let restored;
        let capture = if let Some(capture) = session
            .selected
            .as_ref()
            .filter(|c| c.snapshot.id().as_str() == snapshot)
        {
            capture
        } else {
            let index = session
                .versions
                .iter()
                .position(|v| v.snapshot == snapshot)
                .ok_or_else(|| conflict("Function history source version differs"))?;
            restored = self.restore(session, index)?;
            &restored
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
        let (declaration, _) = self.declaration(session, version.candidate)?;
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
        index: usize,
    ) -> Result<(Declaration, blake3::Hash), CompanionError> {
        let path = ReviewPath::parse(session.path.as_bytes().to_vec()).map_err(unavailable)?;
        let commit = &session.commits[index];
        let mut input = session
            .repository
            .pin_file(commit, &session.path)
            .map_err(unavailable)?;
        let ReadEntry::Present {
            kind: EntryKind::Regular { .. },
            bytes,
        } = input
            .read(&path, FILE_BYTES, &|| false)
            .map_err(unavailable)?
        else {
            return Err(invalid(
                "Function unavailable at this revision; history stops at a deletion or file move",
            ));
        };
        let doc = Document::capture(
            session.repository.workspace().clone(),
            session.path.clone(),
            &bytes,
            vec![],
            session.classification.clone(),
        )
        .map_err(unavailable)?;
        let source = Snapshot::new(
            session.repository.workspace().clone(),
            vec![Arc::new(doc)],
            CapturePolicyId::derive(&[b"function-history-source-v1"]),
        )
        .map_err(unavailable)?;
        let analyzed = self.host.analyze_history(&source).map_err(unavailable)?;
        let snapshot = Arc::new(
            Snapshot::new(
                analyzed.workspace().clone(),
                analyzed.documents().to_vec(),
                CapturePolicyId::derive(&[
                    b"function-history-revision-v1",
                    analyzed.policy().as_str().as_bytes(),
                    commit.as_bytes(),
                ]),
            )
            .map_err(unavailable)?,
        );
        let doc = snapshot
            .file_by_path(&session.path)
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
                    .filter(|f| f.unique && f.key == session.key && f.kind == "function")
                    .map(move |f| (a, f))
            });
        let (analysis,fact) = matches.next().ok_or_else(|| invalid("Function missing or ambiguous at this revision; history stops before a rename, move or ambiguous declaration"))?;
        if matches.next().is_some() {
            return Err(invalid("Function is ambiguous at this revision"));
        }
        let digest = blake3::hash(&bytes[fact.span.clone()]);
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
    fn advance(&self, session: &mut Session) -> Result<(), CompanionError> {
        if session.stopped {
            return Err(unavailable(&session.boundary));
        }
        let mut cursor = session.cursor;
        let mut chosen: Option<(usize, Declaration, blake3::Hash)> = None;
        while cursor < session.commits.len() {
            let (declaration, digest) = match self.declaration(session, cursor) {
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
            let commit = &session.commits[candidate];
            let policy = self.host.source_policy();
            let modules = match session.language.as_str() {
                "rust" => module_context::rust_modules(
                    &session.repository,
                    commit,
                    policy,
                    &declaration.snapshot.documents()[0],
                ),
                "python" => module_context::python_modules(
                    &session.repository,
                    commit,
                    policy,
                    &session.path,
                ),
                _ => Some(ModuleIndex::new(
                    &declaration.snapshot,
                    &mut ModuleCache::default(),
                )),
            }
            .unwrap_or_else(|| ModuleIndex::empty(declaration.snapshot.workspace()));
            let modules = Arc::new(modules);
            let capture = declaration.bind(modules.clone())?;
            session.cursor = cursor;
            session.digest = digest;
            session.versions.push(Version {
                candidate,
                snapshot: capture.snapshot.id().as_str().into(),
                modules,
            });
            session.index = session.versions.len() - 1;
            session.selected = Some(capture);
            return Ok(());
        }
        session.stopped = true;
        Err(unavailable(&session.boundary))
    }
}
fn size(session: &Session) -> usize {
    session.selected.as_ref().map_or(0, |c| {
        c.snapshot.documents().iter().map(|d| d.bytes()).sum()
    })
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
    let doc = capture
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
    let source = window(&capture.snapshot, doc, &query)?;
    Ok(
        json!({"token":session.token,"tip":session.tip,"commit":session.commits[session.versions[session.index].candidate],"index":session.index,"snapshot":capture.snapshot.id().as_str(),"symbol":capture.symbol,"origin":source["lines"][0],"window":source}),
    )
}
