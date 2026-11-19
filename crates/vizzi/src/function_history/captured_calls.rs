//! Metadata and source navigation within one exact retained file observation.
use super::*;
use vizzi_core::structure::Invocation;

/// Native lifetime owner of a captured file; no working pathname is accepted.
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapturedOwner {
    /// An accepted Git function-history version.
    History {
        /// History session token.
        token: String,
        /// Accepted version ordinal, preserving revision context through reversions.
        index: usize,
    },
    /// One independently owned side of a frozen comparison.
    Comparison {
        /// Pair lifetime token.
        token: String,
        /// Exact older or newer side.
        side: String,
    },
}
/// An exact declaration in an owned file observation.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenSource {
    /// Native capture lifetime and revision selection.
    pub owner: CapturedOwner,
    /// Exact file observation.
    pub snapshot: String,
    /// Native file identity.
    pub file: String,
    /// Immutable document version.
    pub version: String,
    /// Exact native declaration identity in this observation.
    pub symbol: String,
}
/// Read projections for captured-file call exploration.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapturedQuery {
    /// Identifier actions for a visible section.
    Actions {
        /// First one-based source line.
        line_start: usize,
        /// At most 96 visible source lines.
        line_count: usize,
        /// Native continuation offset.
        offset: usize,
    },
    /// Written invocations enclosed by the selected declaration.
    Calls {
        /// Native continuation offset; pages contain at most eight calls.
        offset: usize,
    },
    /// Resolve the exact written invocation against this captured file only.
    Definition {
        /// Opaque captured call identity.
        call: String,
    },
    /// Read an explicitly selected native candidate beside its caller.
    Open {
        /// Captured call witness.
        call: String,
        /// Candidate identity from this exact invocation.
        target: String,
    },
}
/// One bounded, idempotent captured-file query.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedRequest {
    /// Exact source binding.
    pub source: FrozenSource,
    /// Metadata or destination projection.
    pub query: CapturedQuery,
}
impl FunctionHistory {
    /// Projects captured call evidence without querying live source or language servers.
    pub fn captured_query(&self, request: &CapturedRequest) -> Result<Value, CompanionError> {
        let s = &request.source;
        if ![&s.snapshot, &s.file, &s.version, &s.symbol]
            .into_iter()
            .all(|s| opaque(s))
        {
            return Err(invalid("Invalid captured declaration binding"));
        }
        let project = |capture: &Capture, label: &str| {
            let index = capture.structure();
            let caller = index
                .resolve(None, &s.symbol)
                .ok_or_else(|| conflict("Captured declaration differs from the requested file"))?;
            if caller.source.file != s.file || caller.source.version != s.version {
                return Err(conflict("Captured declaration source differs"));
            }
            match &request.query {
                CapturedQuery::Actions {
                    line_start,
                    line_count,
                    offset,
                } => serde_json::to_value(
                    index
                        .actions(&s.symbol, *line_start, *line_count, *offset)
                        .map_err(|e| invalid(e.to_string()))?,
                )
                .map_err(unavailable),
                CapturedQuery::Calls { offset } => serde_json::to_value(
                    index
                        .outgoing(&s.symbol, &caller.source.path, *offset)
                        .map_err(|e| invalid(e.to_string()))?,
                )
                .map_err(unavailable),
                CapturedQuery::Definition { call } => {
                    let invocation = invocation(&index, call, &s.symbol)?;
                    let coverage = index
                        .outgoing(&s.symbol, &caller.source.path, 0)
                        .map_err(|e| invalid(e.to_string()))?
                        .coverage;
                    let complete = coverage.omitted_symbols == 0 && coverage.partial_files == 0;
                    Ok(
                        json!({"snapshot":s.snapshot,"complete":complete,"coverage":coverage,"state":if invocation.candidate_count==0{"unresolved"}else if invocation.candidate_count==1{"unique"}else{"ambiguous"},"invocation":invocation,"detail":"Only declarations in this captured file are considered; project type resolution is unavailable."}),
                    )
                }
                CapturedQuery::Open { call, target } => {
                    let invocation = invocation(&index, call, &s.symbol)?;
                    let target = invocation
                        .candidates
                        .iter()
                        .find(|c| {
                            c.id == *target
                                && c.source.file == s.file
                                && c.source.version == s.version
                        })
                        .ok_or_else(|| {
                            conflict("Destination is not a candidate of the captured call")
                        })?;
                    let doc = capture
                        .snapshot
                        .file(&s.file)
                        .ok_or_else(|| conflict("Captured source file differs"))?;
                    let source = &target.source;
                    let data = super::window(
                        &capture.snapshot,
                        doc,
                        &WindowQuery {
                            line_start: Some(source.line),
                            line_count: Some(96.min(source.end_line - source.line + 1)),
                            outline_count: Some(1),
                            reflow: Some(true),
                            ..WindowQuery::default()
                        },
                    )?;
                    let origin = super::window(
                        &capture.snapshot,
                        doc,
                        &WindowQuery {
                            line_start: Some(source.line),
                            line_count: Some(1),
                            outline_count: Some(1),
                            ..WindowQuery::default()
                        },
                    )?;
                    Ok(
                        json!({"invocation":invocation,"frame":{"snapshot":s.snapshot,"symbol":target,"captured":{"owner":s.owner,"label":label},"window":data,"origin":origin["lines"][0]}}),
                    )
                }
            }
        };
        match &s.owner {
            CapturedOwner::Comparison { token, side } => self.with_comparison_capture(
                &ComparisonWindow {
                    token: token.clone(),
                    side: side.clone(),
                    snapshot: s.snapshot.clone(),
                    file: s.file.clone(),
                    version: s.version.clone(),
                },
                project,
            ),
            CapturedOwner::History { token, index } => {
                let sessions = self.lock()?;
                let session = sessions
                    .iter()
                    .find(|v| v.token == *token)
                    .ok_or_else(|| conflict("Captured history expired; return to live"))?;
                let version = session
                    .versions
                    .get(*index)
                    .ok_or_else(|| conflict("Captured history version is unavailable"))?;
                if version.snapshot != s.snapshot {
                    return Err(conflict("Captured history observation differs"));
                }
                let restored;
                let capture = if let Some(capture) = session
                    .selected
                    .as_ref()
                    .filter(|c| c.snapshot.id().as_str() == s.snapshot)
                {
                    capture
                } else {
                    restored = self.restore(session, *index)?;
                    &restored
                };
                if capture.snapshot.id().as_str() != s.snapshot
                    || capture.symbol.source.file != s.file
                    || capture.symbol.source.version != s.version
                {
                    return Err(conflict("Captured history source differs"));
                }
                project(capture, &session.commits[version.candidate])
            }
        }
    }
}
fn invocation(index: &Structure, call: &str, caller: &str) -> Result<Invocation, CompanionError> {
    let invocation = index
        .call(call)
        .ok_or_else(|| conflict("Captured call is unavailable"))?
        .invocation;
    if invocation.owner.as_deref() != Some(caller) {
        return Err(conflict("Captured call belongs to another declaration"));
    }
    Ok(invocation)
}
