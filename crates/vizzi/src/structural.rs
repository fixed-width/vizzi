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
}
impl StructureHost {
    /// Creates an idle projection adapter without source reads or parsing.
    pub fn new(host: Arc<Host>) -> Self {
        Self {
            host,
            cache: Mutex::new(None),
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
