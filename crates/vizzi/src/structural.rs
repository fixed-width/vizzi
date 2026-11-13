//! Current-observation adapter for metadata-only structural operations.

/// Coherent live working-set refresh.
pub mod live;
/// Captured caller/reference queries and observation-bound result paging.
pub mod references;
use crate::{companion::CompanionError, host::Host};
use serde::Deserialize;
pub mod editor;
use serde_json::Value;
use std::sync::{Arc, Mutex, TryLockError};
use vizzi_core::structure::Structure;
/// One structural operation bound to the visible native observation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Expected current observation.
    pub snapshot: String,
    /// Explicit operation; no natural-language intent is inferred.
    pub query: Operation,
}
/// Finite structural query operations.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    /// Decorate exact call identifiers in a captured source section.
    Actions {
        /// Declaration identity from this observation.
        symbol: String,
        /// First visible one-based line.
        line_start: usize,
        /// Visible line allowance, at most 96.
        line_count: usize,
        /// Continuation offset for the finite action page.
        offset: usize,
    },
    /// Find source directories/files.
    Areas {
        /// Literal name/path substring.
        search: String,
        /// Continuation offset.
        offset: usize,
    },
    /// Enumerate captured declarations in a source area.
    Inventory {
        /// Selected normalized source area.
        area: String,
        /// Literal declaration/path filter.
        search: String,
        /// Continuation offset.
        offset: usize,
        /// Restrict the result set to functions before ranking and paging.
        #[serde(default)]
        functions_only: bool,
    },
    /// Follow written invocations in one declaration.
    Calls {
        /// Opaque declaration identity from this observation.
        symbol: String,
        /// Selected source area.
        area: String,
        /// Continuation offset.
        offset: usize,
    },
    /// Inspect same-name candidate uses.
    Uses {
        /// Opaque declaration identity from this observation.
        symbol: String,
        /// Selected source area.
        area: String,
        /// Continuation offset.
        offset: usize,
    },
}
struct Cached {
    snapshot: String,
    index: Structure,
    modules: vizzi_core::modules::ModuleCache,
}
impl Cached {
    fn rebuild(snapshot: &vizzi_core::model::Snapshot, previous: Option<Self>) -> Self {
        let mut modules = previous.map(|c| c.modules).unwrap_or_default();
        let index = Structure::with_module_cache(snapshot, &mut modules);
        Self {
            snapshot: snapshot.id().as_str().into(),
            index,
            modules,
        }
    }
}
/// One lazily constructed metadata projection; no retired snapshot is retained.
pub struct StructureHost {
    reference_sets: Mutex<Vec<references::ReferenceSet>>,
    host: Arc<Host>,
    cache: Mutex<Option<Cached>>,
    definitions: Option<Arc<dyn crate::definitions::Provider>>,
}
impl StructureHost {
    pub(crate) fn dependency(
        &self,
        id: &str,
    ) -> Result<Option<Arc<crate::dependency_sources::Capture>>, CompanionError> {
        self.definitions
            .as_ref()
            .and_then(|p| p.dependencies())
            .map(|c| c.get(id).map_err(CompanionError::Unavailable))
            .transpose()
            .map(Option::flatten)
    }
    fn external(
        &self,
        id: &str,
    ) -> Result<Option<Arc<crate::dependency_sources::Capture>>, CompanionError> {
        let Some(catalog) = self.definitions.as_ref().and_then(|p| p.dependencies()) else {
            return Ok(None);
        };
        Ok(catalog
            .sources()
            .map_err(CompanionError::Unavailable)?
            .into_iter()
            .find(|s| s.index.resolve(None, id).is_some() || s.index.call(id).is_some()))
    }
    /// Reads a version-bound dependency window; no filesystem pathname is accepted.
    pub fn dependency_window(
        &self,
        snapshot: &str,
        file: &str,
        version: &str,
        query: &vizzi_core::projection::WindowQuery,
    ) -> Result<Value, CompanionError> {
        let source = self.dependency(snapshot)?.ok_or_else(|| {
            CompanionError::Conflict("Dependency capture expired; reopen it from its caller".into())
        })?;
        if source.doc().id().as_str() != file || source.doc().version().as_str() != version {
            return Err(CompanionError::Invalid(
                "Dependency source reference differs".into(),
            ));
        }
        let mut value = serde_json::to_value(
            source
                .doc()
                .window(query, None)
                .map_err(|e| CompanionError::Unavailable(e.to_string()))?,
        )
        .map_err(|e| CompanionError::Unavailable(e.to_string()))?;
        value["snapshot"] = snapshot.into();
        Ok(value)
    }
    /// Verifies an explicit dependency editor location against the captured bytes.
    pub fn dependency_location(
        &self,
        request: &crate::companion::OpenSource,
    ) -> Result<crate::editor::EditorLocation, CompanionError> {
        let source = self
            .dependency(&request.snapshot)?
            .ok_or_else(|| CompanionError::Conflict("Dependency capture expired".into()))?;
        if source.doc().id().as_str() != request.file
            || source.doc().version().as_str() != request.version
            || request.line == 0
            || request.line > source.doc().captured().line_count()
        {
            return Err(CompanionError::Invalid(
                "Invalid dependency editor reference".into(),
            ));
        }
        source.verify().map_err(CompanionError::Conflict)?;
        Ok(crate::editor::EditorLocation {
            path: source.path(),
            line: request.line,
        })
    }
    /// Creates an idle projection adapter without source reads or parsing.
    pub fn new(host: Arc<Host>) -> Self {
        Self {
            reference_sets: Mutex::new(Vec::new()),
            host,
            cache: Mutex::new(None),
            definitions: None,
        }
    }
    /// Runs one bounded operation, refusing stale captures before returning results.
    pub fn query(&self, request: &Request) -> Result<Value, CompanionError> {
        let snapshot = self
            .host
            .application()
            .snapshot()
            .ok_or_else(|| CompanionError::Unavailable("Wait for source indexing".into()))?;
        if snapshot.id().as_str() != request.snapshot {
            return Err(CompanionError::Conflict(
                "Source changed; choose the area again".into(),
            ));
        }
        let mut cache = match self.cache.try_lock() {
            Ok(g) => g,
            Err(TryLockError::Poisoned(e)) => e.into_inner(),
            Err(TryLockError::WouldBlock) => {
                return Err(CompanionError::Unavailable(
                    "Another structural query is running; retry".into(),
                ));
            }
        };
        if cache
            .as_ref()
            .is_none_or(|c| c.snapshot != request.snapshot)
        {
            let previous = cache.take();
            *cache = Some(Cached::rebuild(&snapshot, previous));
        }
        let index = &cache
            .as_ref()
            .ok_or_else(|| CompanionError::Unavailable("Structure is unavailable".into()))?
            .index;
        let foreign = match &request.query {
            Operation::Actions { symbol, .. }
            | Operation::Calls { symbol, .. }
            | Operation::Uses { symbol, .. }
                if index.resolve(None, symbol).is_none() =>
            {
                self.external(symbol)?
            }
            _ => None,
        };
        let index = foreign.as_ref().map_or(index, |s| &s.index);
        let mut value = match &request.query {
            Operation::Actions {
                symbol,
                line_start,
                line_count,
                offset,
            } => serde_json::to_value(
                index
                    .actions(symbol, *line_start, *line_count, *offset)
                    .map_err(|e| CompanionError::Invalid(e.to_string()))?,
            ),
            Operation::Areas { search, offset } => serde_json::to_value(
                index
                    .areas(search, *offset)
                    .map_err(|e| CompanionError::Invalid(e.to_string()))?,
            ),
            Operation::Inventory {
                area,
                search,
                offset,
                functions_only,
            } => serde_json::to_value(
                (if *functions_only {
                    index.functions(area, search, *offset)
                } else {
                    index.inventory(area, search, *offset)
                })
                .map_err(|e| CompanionError::Invalid(e.to_string()))?,
            ),
            Operation::Calls {
                symbol,
                area,
                offset,
            } if foreign.is_some() => serde_json::to_value(
                foreign
                    .as_ref()
                    .ok_or_else(|| CompanionError::Unavailable("Dependency source expired".into()))?
                    .calls(symbol, *offset)
                    .map_err(CompanionError::Unavailable)?,
            ),
            Operation::Calls {
                symbol,
                area,
                offset,
            } => serde_json::to_value(
                index
                    .outgoing(symbol, area, *offset)
                    .map_err(|e| CompanionError::Invalid(e.to_string()))?,
            ),
            Operation::Uses {
                symbol,
                area,
                offset,
            } => serde_json::to_value(
                index
                    .uses(symbol, area, *offset)
                    .map_err(|e| CompanionError::Invalid(e.to_string()))?,
            ),
        }
        .map_err(|e| CompanionError::Unavailable(e.to_string()))?;
        if foreign.is_some() {
            value["snapshot"] = request.snapshot.clone().into();
        }
        if self
            .host
            .application()
            .snapshot()
            .is_none_or(|current| current.id() != snapshot.id())
        {
            return Err(CompanionError::Conflict(
                "Source changed during exploration; choose the area again".into(),
            ));
        }
        Ok(value)
    }
}

/// Definition lookup accepts a native call identity, never a browser-supplied code position.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionRequest {
    /// Current captured observation.
    pub snapshot: String,
    /// Call identity from the structured call page.
    pub call: String,
}
/// Definition result precision, including known destinations outside captured source.
#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionState {
    /// One complete captured destination.
    Unique,
    /// Several semantic destinations or omitted/outside destinations.
    Ambiguous,
    /// No semantic definition was returned.
    Unresolved,
    /// No captured destination represents the known definition.
    Outside,
    /// Provider is absent or could not analyse this call.
    Unavailable,
}
/// Version-bound semantic selection or an explicitly weaker fallback.
#[derive(serde::Serialize)]
pub struct DefinitionResult {
    /// Exact source observation.
    pub snapshot: String,
    /// Result precision.
    pub state: DefinitionState,
    /// Original witness with semantic candidates when available.
    pub invocation: vizzi_core::structure::Invocation,
    /// Optional reason for fallback or unavailable source.
    pub detail: Option<String>,
}
impl StructureHost {
    /// Configures a host definition provider; ordinary metadata queries remain independent.
    pub fn with_definitions(
        host: Arc<Host>,
        provider: Arc<dyn crate::definitions::Provider>,
    ) -> Self {
        Self {
            reference_sets: Mutex::new(Vec::new()),
            host,
            cache: Mutex::new(None),
            definitions: Some(provider),
        }
    }
    /// Whether type-aware navigation is explicitly configured.
    pub fn definitions_available(&self) -> bool {
        self.definitions.is_some()
    }
    /// Resolves only the captured call, rejecting source changes before returning an action.
    pub fn definition(
        &self,
        request: &DefinitionRequest,
    ) -> Result<DefinitionResult, CompanionError> {
        let snapshot = self
            .host
            .application()
            .snapshot()
            .ok_or_else(|| CompanionError::Unavailable("Wait for source indexing".into()))?;
        if snapshot.id().as_str() != request.snapshot {
            return Err(CompanionError::Conflict(
                "Source changed before definition lookup".into(),
            ));
        }
        if request.call.len() != 64 {
            return Err(CompanionError::Invalid(
                "Invalid captured call identity".into(),
            ));
        }
        let call = {
            let mut cache = self
                .cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if cache
                .as_ref()
                .is_none_or(|c| c.snapshot != request.snapshot)
            {
                let previous = cache.take();
                *cache = Some(Cached::rebuild(&snapshot, previous));
            }
            let local = cache.as_ref().and_then(|c| c.index.call(&request.call));
            let foreign = if local.is_none() {
                self.external(&request.call)?
            } else {
                None
            };
            local
                .or_else(|| {
                    foreign
                        .as_ref()
                        .and_then(|s| s.index.call(&request.call))
                        .map(|mut call| {
                            if let Some(s) = &foreign {
                                call.invocation.candidates = call
                                    .invocation
                                    .candidates
                                    .into_iter()
                                    .map(|c| s.symbol(c))
                                    .collect();
                            }
                            call
                        })
                })
                .ok_or_else(|| {
                    CompanionError::Invalid("Call is absent from this observation".into())
                })?
        };
        let mut invocation = call.invocation.clone();
        let mut state = DefinitionState::Unavailable;
        let mut detail = None;
        if invocation.binding.is_some() {
            state = DefinitionState::Unique;
        } else if let Some(provider) = &self.definitions {
            match provider.resolve(snapshot.clone(), &call) {
                Err(e) => detail = Some(crate::review_application::clip(&e, 384).0),
                Ok(answer) => {
                    if answer.locations.len().saturating_add(answer.outside) > 64 {
                        return Err(CompanionError::Unavailable(
                            "Definition result exceeds its allowance".into(),
                        ));
                    }
                    if answer.context.len() != 64
                        || !answer
                            .context
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    {
                        return Err(CompanionError::Unavailable(
                            "Invalid semantic context identity".into(),
                        ));
                    }
                    let mut candidates = vec![];
                    let mut outside = answer.outside;
                    let cache = self
                        .cache
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let index = cache
                        .as_ref()
                        .filter(|c| c.snapshot == request.snapshot)
                        .ok_or_else(|| {
                            CompanionError::Conflict(
                                "Source changed during definition lookup".into(),
                            )
                        })?;
                    for location in answer.locations {
                        let found = if let Some(capture) = &location.capture {
                            if let Some(source) = self.dependency(capture)? {
                                if source.doc().path() != location.path {
                                    None
                                } else {
                                    let text = source.doc().resident_source().ok_or_else(|| {
                                        CompanionError::Unavailable(
                                            "Dependency text unavailable".into(),
                                        )
                                    })?;
                                    let byte =
                                        crate::definitions::byte_offset(text, location.position)
                                            .map_err(CompanionError::Unavailable)?;
                                    source
                                        .index
                                        .definition(&location.path, byte)
                                        .map(|symbol| source.symbol(symbol))
                                }
                            } else {
                                None
                            }
                        } else if let Some(doc) = snapshot
                            .documents()
                            .iter()
                            .find(|d| d.path() == location.path)
                        {
                            let text = doc
                                .captured()
                                .read_text(8 * 1024 * 1024)
                                .map_err(|e| {
                                    CompanionError::Unavailable(format!(
                                        "Cannot verify captured definition: {e}"
                                    ))
                                })?
                                .ok_or_else(|| {
                                    CompanionError::Unavailable(
                                        "Definition source is not UTF-8".into(),
                                    )
                                })?;
                            let byte = crate::definitions::byte_offset(&text, location.position)
                                .map_err(CompanionError::Unavailable)?;
                            index.index.definition(&location.path, byte)
                        } else {
                            None
                        };
                        if let Some(symbol) = found {
                            if !candidates
                                .iter()
                                .any(|s: &vizzi_core::structure::Symbol| s.id == symbol.id)
                            {
                                candidates.push(symbol);
                            }
                        } else {
                            outside += 1;
                        }
                    }
                    let total = candidates.len() + outside;
                    if total == 0 {
                        state = DefinitionState::Unresolved;
                        detail=Some("The language server could not establish a definition; these are name matches.".into());
                    } else {
                        state = if candidates.is_empty() {
                            DefinitionState::Outside
                        } else if total == 1 {
                            DefinitionState::Unique
                        } else {
                            DefinitionState::Ambiguous
                        };
                        invocation.binding =
                            Some(vizzi_core::calls::TargetBasis::LanguageServerDefinition);
                        invocation.semantic_context = Some(answer.context);
                        invocation.candidate_count = total;
                        invocation.candidates = candidates.into_iter().take(8).collect();
                        if outside > 0 {
                            detail = Some(format!(
                                "{outside} definition(s) are outside captured declarations."
                            ));
                        }
                    }
                }
            }
        } else {
            detail =
                Some("Type-aware navigation is not configured; these are name matches.".into());
        }
        if self
            .host
            .application()
            .snapshot()
            .is_none_or(|now| now.id() != snapshot.id())
        {
            return Err(CompanionError::Conflict(
                "Source changed during definition lookup".into(),
            ));
        }
        Ok(DefinitionResult {
            snapshot: request.snapshot.clone(),
            state,
            invocation,
            detail,
        })
    }
}
