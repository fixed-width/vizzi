//! Bounded question search over immutable viewer documents, without reparsing source.
use crate::{companion::CompanionError, host::Host, review_application::clip};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, TryLockError},
};
use vizzi_core::model::SourceEncoding;
use vizzi_search::{Corpus, SearchRole, SourceScope};
/// Trial ordering; both choices use the same captured source corpus.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Selector {
    /// Role-specific source discovery.
    Focused,
    /// Original compact lexical matching.
    Original,
}
/// A question bound to the capture visible when submitted.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    /// Expected current native snapshot.
    pub snapshot: String,
    /// Reader question, at most 2,048 UTF-8 bytes.
    pub question: String,
    /// Explicit trial ordering.
    pub selector: Selector,
}
/// One finite source result with native preview/pin bindings.
#[derive(Serialize)]
pub struct SearchHit {
    /// Stable corpus excerpt identity.
    pub id: String,
    /// Bounded investigation title.
    pub title: String,
    /// Relative source path.
    pub path: String,
    /// Native file identity.
    pub file: String,
    /// Native document version.
    pub version: String,
    /// First represented line.
    pub line: usize,
    /// Last represented line.
    pub end_line: usize,
    /// Literal excerpt.
    pub excerpt: String,
    /// Suggested pool origin, not a semantic guarantee.
    pub suggested_role: Option<SearchRole>,
    /// Captured lexical scope at the excerpt start.
    pub scope: Option<SourceScope>,
}
/// Finite results over selected indexed source, with explicit omissions.
#[derive(Serialize)]
pub struct SearchResults {
    /// Native observation used for every result.
    pub snapshot: String,
    /// Reader question.
    pub question: String,
    /// Selected ordering.
    pub selector: Selector,
    /// All captured documents in the source index.
    pub available_files: usize,
    /// UTF-8 documents admitted to search.
    pub searched_files: usize,
    /// Files outside encoding/file/corpus allowances.
    pub omitted_files: usize,
    /// At most eight source cards/4,000 excerpt bytes.
    pub results: Vec<SearchHit>,
}
struct Binding {
    file: String,
    version: String,
}
struct Index {
    snapshot: String,
    corpus: Corpus,
    bindings: BTreeMap<String, Binding>,
    available: usize,
}
/// One lazily prepared corpus; no persistent Arc to a retired native snapshot.
pub struct QuestionSearch {
    host: Arc<Host>,
    index: Mutex<Option<Index>>,
}
impl QuestionSearch {
    /// Creates an idle trial without indexing, source IO or parsing.
    pub fn new(host: Arc<Host>) -> Self {
        Self {
            host,
            index: Mutex::new(None),
        }
    }
    /// Searches current captured source, refusing stale work before publication.
    pub fn search(&self, request: &SearchRequest) -> Result<SearchResults, CompanionError> {
        if request.question.trim().is_empty() || request.question.len() > 2048 {
            return Err(CompanionError::Invalid(
                "Enter a question of at most 2,048 UTF-8 bytes".into(),
            ));
        }
        let snapshot = self.host.application().snapshot().ok_or_else(|| {
            CompanionError::Unavailable("Wait for source indexing before searching".into())
        })?;
        if snapshot.id().as_str() != request.snapshot {
            return Err(CompanionError::Conflict(
                "Source changed; search again".into(),
            ));
        }
        let mut state = match self.index.try_lock() {
            Ok(g) => g,
            Err(TryLockError::Poisoned(e)) => e.into_inner(),
            Err(TryLockError::WouldBlock) => {
                return Err(CompanionError::Unavailable(
                    "Another source search is running; retry shortly".into(),
                ));
            }
        };
        if state.is_none() {
            *state = Some(Index {
                snapshot: String::new(),
                corpus: Corpus::new().map_err(|e| CompanionError::Unavailable(e.to_string()))?,
                bindings: BTreeMap::new(),
                available: 0,
            });
        }
        let index = state
            .as_mut()
            .ok_or_else(|| CompanionError::Unavailable("Source search is unavailable".into()))?;
        if index.snapshot != request.snapshot {
            let mut documents = vec![];
            let mut bindings = BTreeMap::new();
            let mut bytes = 0;
            for doc in snapshot.documents() {
                if doc.encoding() != SourceEncoding::Utf8
                    || doc.bytes() > 2 * 1024 * 1024
                    || documents.len() == 4096
                    || bytes + doc.bytes() > 32 * 1024 * 1024
                {
                    continue;
                }
                bytes += doc.bytes();
                documents.push(doc.clone());
                bindings.insert(
                    doc.path().to_owned(),
                    Binding {
                        file: doc.id().as_str().into(),
                        version: doc.version().as_str().into(),
                    },
                );
            }
            index.corpus.replace_documents(&documents).map_err(|e| {
                CompanionError::Unavailable(format!("Captured source could not be searched: {e}"))
            })?;
            index.bindings = bindings;
            index.available = snapshot.documents().len();
            index.snapshot = request.snapshot.clone();
        }
        let context = match request.selector {
            Selector::Focused => index.corpus.retrieve_search(&request.question, 8, 4000),
            Selector::Original => index.corpus.retrieve_compact(&request.question, 8, 4000),
        }
        .map_err(|e| CompanionError::Invalid(e.to_string()))?;
        let mut results = vec![];
        for (position, source) in context.evidence().iter().enumerate() {
            let binding = index.bindings.get(&source.path).ok_or_else(|| {
                CompanionError::Unavailable("Search source binding is unavailable".into())
            })?;
            let title = source
                .scope
                .as_ref()
                .map(|s| {
                    s.parents
                        .last()
                        .map(|p| format!("{p}::{}", s.name))
                        .unwrap_or_else(|| s.name.clone())
                })
                .unwrap_or_else(|| {
                    source
                        .path
                        .rsplit('/')
                        .next()
                        .unwrap_or(&source.path)
                        .into()
                });
            results.push(SearchHit {
                id: source.id.clone(),
                title: clip(&title, 128).0,
                path: source.path.clone(),
                file: binding.file.clone(),
                version: binding.version.clone(),
                line: source.start_line,
                end_line: source.end_line,
                excerpt: source.text.clone(),
                suggested_role: context.search_roles().get(position).copied(),
                scope: source.scope.clone(),
            });
        }
        if self
            .host
            .application()
            .snapshot()
            .is_none_or(|current| current.id() != snapshot.id())
        {
            return Err(CompanionError::Conflict(
                "Source changed while searching; search again".into(),
            ));
        }
        Ok(SearchResults {
            snapshot: request.snapshot.clone(),
            question: request.question.clone(),
            selector: request.selector,
            available_files: index.available,
            searched_files: index.bindings.len(),
            omitted_files: index.available - index.bindings.len(),
            results,
        })
    }
}
