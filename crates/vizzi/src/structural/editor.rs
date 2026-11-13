//! Source-checked editor positions become observation-bound call identities.
use super::{Cached, StructureHost};
use crate::{companion::CompanionError, definitions::Position};
use serde::{Deserialize, Serialize};
use std::sync::TryLockError;
use vizzi_core::structure::{Invocation, Symbol};

/// A handoff bound to the current repository observation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Current source observation.
    pub snapshot: String,
    /// Position-to-call mapping or retrieval of an already bound call.
    pub query: Query,
}
/// The editor sends a saved-source witness; the tab subsequently uses only native IDs.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Query {
    /// Select one captured written-call identifier.
    Position {
        /// Normalized repository-relative source path.
        path: String,
        /// Complete saved source, bounded to 4 MiB.
        text: String,
        /// Zero-based UTF-16 editor position.
        position: Position,
    },
    /// Retrieve the enclosing declaration and call for the workspace.
    Invocation {
        /// Observation-bound native call ID.
        call: String,
    },
}
/// The workspace opens the caller before using its ordinary call-follow operation.
#[derive(Serialize)]
pub struct Handoff {
    /// Current observation shared by both identities.
    pub snapshot: String,
    /// Captured enclosing declaration.
    pub caller: Symbol,
    /// Exact selected invocation, with existing candidate/binding semantics.
    pub invocation: Invocation,
}
impl StructureHost {
    /// Maps exact saved editor source to a call without performing name-based lookup.
    pub fn editor_call(&self, request: &Request) -> Result<Handoff, CompanionError> {
        let invalid = |message: &str| CompanionError::Invalid(message.into());
        let snapshot = self
            .host
            .application()
            .snapshot()
            .ok_or_else(|| CompanionError::Unavailable("Wait for source indexing".into()))?;
        if snapshot.id().as_str() != request.snapshot {
            return Err(CompanionError::Conflict(
                "Source changed; retry from the editor".into(),
            ));
        }
        let position = if let Query::Position {
            path,
            text,
            position,
        } = &request.query
        {
            if text.len() > 4 * 1024 * 1024 {
                return Err(invalid("Editor source exceeds the handoff allowance"));
            }
            // Only paths already published in this observation are addressable.
            let doc=snapshot.file_by_path(path).ok_or_else(||invalid("This file is outside Vizzi's captured source; check the project root and source scopes"))?;
            let captured = doc
                .captured()
                .read_text(4 * 1024 * 1024)
                .map_err(|e| CompanionError::Unavailable(e.to_string()))?
                .ok_or_else(|| invalid("Editor source is not captured UTF-8 text"))?;
            if captured.as_ref() != text {
                return Err(CompanionError::Conflict(
                    "Saved editor source has not been captured yet; wait for source refresh".into(),
                ));
            }
            Some(crate::definitions::byte_offset(&captured, *position).map_err(|e| invalid(&e))?)
        } else {
            None
        };
        let mut cache = match self.cache.try_lock() {
            Ok(cache) => cache,
            Err(TryLockError::Poisoned(error)) => error.into_inner(),
            Err(TryLockError::WouldBlock) => {
                return Err(CompanionError::Unavailable(
                    "Another source query is running; retry".into(),
                ));
            }
        };
        if cache
            .as_ref()
            .is_none_or(|cache| cache.snapshot != request.snapshot)
        {
            let previous = cache.take();
            *cache = Some(Cached::rebuild(&snapshot, previous));
        }
        let index = &cache
            .as_ref()
            .ok_or_else(|| invalid("Captured structure is unavailable"))?
            .index;
        let call = match &request.query {
            Query::Position { path, .. } => index
                .call_at(
                    path,
                    position.ok_or_else(|| invalid("Editor position is unavailable"))?,
                )
                .ok_or_else(|| invalid("Place the cursor on a captured call name"))?,
            Query::Invocation { call } => index
                .call(call)
                .ok_or_else(|| invalid("The selected call is absent from this observation"))?,
        };
        let caller = call
            .invocation
            .owner
            .as_deref()
            .and_then(|owner| index.resolve(None, owner))
            .cloned()
            .ok_or_else(|| invalid("This call has no captured enclosing declaration"))?;
        if self
            .host
            .application()
            .snapshot()
            .is_none_or(|current| current.id() != snapshot.id())
        {
            return Err(CompanionError::Conflict(
                "Source changed during editor navigation; retry".into(),
            ));
        }
        Ok(Handoff {
            snapshot: request.snapshot.clone(),
            caller,
            invocation: call.invocation,
        })
    }
}
