//! Current-observation adapter for metadata-only structural operations.
use crate::{companion::CompanionError, host::Host};
use serde::Deserialize;
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
}
/// One lazily constructed metadata projection; no retired snapshot is retained.
pub struct StructureHost {
    host: Arc<Host>,
    cache: Mutex<Option<Cached>>,
    definitions: Option<Arc<dyn crate::definitions::Provider>>,
}
impl StructureHost {
    /// Creates an idle projection adapter without source reads or parsing.
    pub fn new(host: Arc<Host>) -> Self {
        Self {
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
            *cache = Some(Cached {
                snapshot: request.snapshot.clone(),
                index: Structure::new(&snapshot),
            });
        }
        let index = &cache
            .as_ref()
            .ok_or_else(|| CompanionError::Unavailable("Structure is unavailable".into()))?
            .index;
        let value = match &request.query {
            Operation::Areas { search, offset } => serde_json::to_value(
                index
                    .areas(search, *offset)
                    .map_err(|e| CompanionError::Invalid(e.to_string()))?,
            ),
            Operation::Inventory {
                area,
                search,
                offset,
            } => serde_json::to_value(
                index
                    .inventory(area, search, *offset)
                    .map_err(|e| CompanionError::Invalid(e.to_string()))?,
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
                *cache = Some(Cached {
                    snapshot: request.snapshot.clone(),
                    index: Structure::new(&snapshot),
                });
            }
            cache
                .as_ref()
                .and_then(|c| c.index.call(&request.call))
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
                        let found = if let Some(doc) = snapshot
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
