//! Reference evidence mapped to immutable native source and exact written calls.
use super::{Cached, StructureHost};
use crate::{companion::CompanionError, definitions, dependency_sources::Capture};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use vizzi_core::{
    calls::TargetBasis,
    model::Document,
    projection::WindowQuery,
    structure::{Invocation, Source, Structure, Symbol},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// The reader's requested reference projection.
pub enum ReferenceKind {
    /// Written calls to the target.
    Callers,
    /// All supported references, including value uses.
    References,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Native target binding and optional cached result page.
pub struct ReferenceRequest {
    /// Current repository observation.
    pub snapshot: String,
    /// Captured declaration identity.
    pub symbol: String,
    /// Requested view of references.
    pub kind: ReferenceKind,
    /// Independent dependency capture.
    #[serde(default)]
    pub capture: Option<String>,
    /// Earlier native tracking identity for resumed layouts.
    #[serde(default)]
    pub tracking: Option<String>,
    /// Earlier file identity, paired with context.
    #[serde(default)]
    pub file: Option<String>,
    /// Earlier analysis context, paired with file.
    #[serde(default)]
    pub context: Option<String>,
    /// Query identity returned by an earlier page.
    #[serde(default)]
    pub query: Option<String>,
    /// Bounded result offset.
    #[serde(default)]
    pub offset: usize,
}
#[derive(Clone, Serialize)]
/// One source reference and optional exact caller witness.
pub struct ReferenceRow {
    /// Stable identity within the source observation.
    pub id: String,
    /// Captured location of the reference.
    pub source: Source,
    /// Source origin when the reference is in a captured dependency.
    pub origin: Option<vizzi_core::structure::SourceOrigin>,
    /// Containing admitted declaration, if any.
    pub symbol: Option<Symbol>,
    /// Captured written call, present only at an exact call identifier.
    pub site: Option<Invocation>,
    /// Bounded line context.
    pub snippet: String,
}
#[derive(Clone, Serialize)]
/// A finite observation-bound reference page.
pub struct ReferencePage {
    /// Current project observation.
    pub snapshot: String,
    /// Native current target after exact tracking recovery.
    pub target: Symbol,
    /// Requested reference projection.
    pub kind: ReferenceKind,
    /// Semantic or name_candidates.
    pub precision: &'static str,
    /// Native query reference for continuation.
    pub query: String,
    /// Total admitted result count.
    pub matching: usize,
    /// Known references outside admitted source.
    pub outside: usize,
    /// Results omitted by bounds.
    pub omitted: usize,
    /// Current finite rows.
    pub items: Vec<ReferenceRow>,
    /// Continuation offset.
    pub next: Option<usize>,
    /// Capability or fallback explanation.
    pub detail: Option<String>,
}
pub(super) struct ReferenceSet {
    page: ReferencePage,
    rows: Vec<ReferenceRow>,
}
fn opaque(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn unavailable(message: impl ToString) -> CompanionError {
    CompanionError::Unavailable(message.to_string())
}
fn page(set: &ReferenceSet, offset: usize) -> ReferencePage {
    let mut page = set.page.clone();
    page.items = set.rows.iter().skip(offset).take(8).cloned().collect();
    page.next = (offset + page.items.len() < set.rows.len()).then_some(offset + page.items.len());
    page
}

fn row(
    index: &Structure,
    doc: &Document,
    byte: usize,
    origin: Option<&Capture>,
    target: &Symbol,
    semantic: Option<&str>,
    kind: ReferenceKind,
) -> Result<Option<ReferenceRow>, CompanionError> {
    let mut site = index
        .call_at(doc.path(), byte)
        .map(|lookup| lookup.invocation);
    if kind == ReferenceKind::Callers && site.is_none() {
        return Ok(None);
    }
    if let Some(call) = &mut site {
        if semantic.is_none()
            && call.binding.is_some()
            && !call.candidates.iter().any(|c| c.id == target.id)
        {
            return Ok(None);
        }
        let includes = index.resolve(None, &target.id).is_some();
        call.candidate_count = if semantic.is_some() {
            1
        } else {
            call.candidate_count
                .saturating_add(usize::from(!includes))
                .max(1)
        };
        call.candidates = vec![target.clone()];
        if let Some(context) = semantic {
            call.binding = Some(TargetBasis::LanguageServerDefinition);
            call.semantic_context = Some(context.into());
        }
    }
    let symbol = site
        .as_ref()
        .and_then(|s| s.owner.as_deref())
        .and_then(|id| index.resolve(None, id))
        .cloned()
        .or_else(|| index.enclosing(doc.path(), byte))
        .map(|s| origin.map_or_else(|| s.clone(), |origin| origin.symbol(s.clone())));
    let lines = doc.lines(&(byte..byte.saturating_add(1)));
    let context = doc
        .analyses()
        .iter()
        .find(|a| {
            matches!(
                a.coverage,
                vizzi_core::model::Coverage::Syntax | vizzi_core::model::Coverage::Partial
            )
        })
        .ok_or_else(|| unavailable("Reference syntax is unavailable"))?
        .context
        .id
        .as_str();
    let source = Source {
        path: doc.path().into(),
        file: doc.id().as_str().into(),
        version: doc.version().as_str().into(),
        context: context.into(),
        line: lines.start,
        end_line: lines.start,
    };
    let column = doc
        .resident_source()
        .and_then(|text| text.get(..byte))
        .map(|prefix| byte - prefix.rfind('\n').map_or(0, |n| n + 1))
        .unwrap_or(0);
    let window = doc
        .window(
            &WindowQuery {
                line_start: Some(lines.start),
                line_count: Some(1),
                column_line: Some(lines.start),
                column: Some(column.saturating_sub(80)),
                outline_count: Some(1),
                ..Default::default()
            },
            None,
        )
        .map_err(unavailable)?;
    let snippet = window
        .lines
        .first()
        .map(|line| crate::review_application::clip(&line.text, 512).0)
        .unwrap_or_default();
    let id = blake3::hash(
        format!(
            "{}:{}:{}:{byte}",
            source.file, source.version, source.context
        )
        .as_bytes(),
    )
    .to_hex()
    .to_string();
    Ok(Some(ReferenceRow {
        id,
        source,
        origin: origin.map(|s| s.origin.clone()),
        symbol,
        site,
        snippet,
    }))
}

impl StructureHost {
    /// Resolves and pages source-backed references to a native declaration.
    pub fn references(&self, request: &ReferenceRequest) -> Result<ReferencePage, CompanionError> {
        if !opaque(&request.snapshot)
            || !opaque(&request.symbol)
            || request.offset > 256
            || request.capture.as_deref().is_some_and(|s| !opaque(s))
            || request.query.as_deref().is_some_and(|s| !opaque(s))
            || request.tracking.as_deref().is_some_and(|s| !opaque(s))
            || request.file.is_some() != request.context.is_some()
            || request.file.as_deref().is_some_and(|s| !opaque(s))
            || request.context.as_deref().is_some_and(|s| !opaque(s))
        {
            return Err(CompanionError::Invalid("Invalid reference request".into()));
        }
        let snapshot = self
            .host
            .application()
            .snapshot()
            .ok_or_else(|| unavailable("Wait for source indexing"))?;
        if snapshot.id().as_str() != request.snapshot {
            return Err(CompanionError::Conflict(
                "Source changed. Refresh references.".into(),
            ));
        }
        if let Some(query) = &request.query {
            let sets = self
                .reference_sets
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let set = sets
                .iter()
                .find(|s| {
                    s.page.query == *query
                        && s.page.snapshot == request.snapshot
                        && s.page.kind == request.kind
                        && s.page.target.id == request.symbol
                })
                .ok_or_else(|| {
                    CompanionError::Conflict("Reference results expired. Refresh the stack.".into())
                })?;
            return Ok(page(set, request.offset));
        }
        if request.offset != 0 {
            return Err(CompanionError::Invalid(
                "A continuation needs a reference query".into(),
            ));
        }
        let external = if let Some(capture) = &request.capture {
            self.dependency(capture)?
        } else {
            None
        };
        if request.capture.is_some() && external.is_none() {
            return Err(CompanionError::Conflict(
                "Dependency capture expired".into(),
            ));
        }
        let (target, identifier) = {
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
            let index = external
                .as_ref()
                .map(|s| &s.index)
                .or_else(|| cache.as_ref().map(|c| &c.index))
                .ok_or_else(|| unavailable("Structure unavailable"))?;
            let target = index
                .resolve(request.tracking.as_deref(), &request.symbol)
                .or_else(|| {
                    index.resolve_context(
                        external.as_ref().map(|s| s.doc()).or_else(|| {
                            snapshot.file(request.file.as_deref()?).map(AsRef::as_ref)
                        })?,
                        request.tracking.as_deref()?,
                        request.context.as_deref()?,
                    )
                })
                .cloned()
                .ok_or_else(|| {
                    CompanionError::Conflict(
                        "Target function is unavailable. Reopen it from current source.".into(),
                    )
                })?;
            let identifier = index
                .identifier(&target.id)
                .ok_or_else(|| unavailable("Target identifier is unavailable"))?;
            (
                external
                    .as_ref()
                    .map_or_else(|| target.clone(), |s| s.symbol(target.clone())),
                identifier,
            )
        };
        let semantic = self
            .definitions
            .as_ref()
            .map(|provider| provider.references(snapshot.clone(), &target, identifier));
        let (answer, detail) = match semantic {
            Some(Ok(answer)) => (Some(answer), None),
            Some(Err(error)) => (None, Some(crate::review_application::clip(&error, 384).0)),
            None => (
                None,
                Some(
                    "Name-based candidates. Reference analysis is unavailable for this language."
                        .into(),
                ),
            ),
        };
        let mut rows = vec![];
        let mut outside = 0;
        let mut omitted = 0;
        let mut seen = BTreeSet::new();
        let cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let index = &cache
            .as_ref()
            .filter(|c| c.snapshot == request.snapshot)
            .ok_or_else(|| CompanionError::Conflict("Source changed during references".into()))?
            .index;
        let precision = if let Some(answer) = answer {
            if !opaque(&answer.context)
                || answer.locations.len().saturating_add(answer.outside) > 4096
            {
                return Err(unavailable("Invalid reference provider response"));
            }
            outside = answer.outside;
            for location in answer.locations {
                let source = if let Some(capture) = &location.capture {
                    self.dependency(capture)?
                } else {
                    None
                };
                let doc = source
                    .as_ref()
                    .filter(|s| s.doc().path() == location.path)
                    .map(|s| s.doc())
                    .or_else(|| {
                        if location.capture.is_none() {
                            snapshot.file_by_path(&location.path).map(AsRef::as_ref)
                        } else {
                            None
                        }
                    });
                let Some(doc) = doc else {
                    outside += 1;
                    continue;
                };
                let Some(text) = doc.resident_source() else {
                    outside += 1;
                    continue;
                };
                let byte =
                    definitions::byte_offset(text, location.position).map_err(unavailable)?;
                let selected = source.as_ref().map_or(index, |s| &s.index);
                if let Some(result) = row(
                    selected,
                    doc,
                    byte,
                    source.as_deref(),
                    &target,
                    Some(&answer.context),
                    request.kind,
                )? && seen.insert(result.id.clone())
                {
                    if rows.len() < 256 {
                        rows.push(result);
                    } else {
                        omitted += 1;
                    }
                }
            }
            "semantic"
        } else {
            let captures = self
                .definitions
                .as_ref()
                .and_then(|p| p.dependencies())
                .map(|c| c.sources())
                .transpose()
                .map_err(unavailable)?
                .unwrap_or_default();
            let indices = std::iter::once((index, None))
                .chain(captures.iter().map(|s| (&s.index, Some(s.as_ref()))));
            for (selected, source) in indices {
                let mut offset = 0;
                loop {
                    let calls = selected
                        .name_uses(&target.name, offset)
                        .map_err(unavailable)?;
                    for site in calls.items {
                        let Some(lookup) = selected.call(&site.id) else {
                            continue;
                        };
                        let Some(span) = lookup.identifier else {
                            continue;
                        };
                        let doc = source
                            .map(|s| s.doc())
                            .or_else(|| snapshot.file(&site.source.file).map(AsRef::as_ref));
                        if let Some(doc) = doc
                            && let Some(result) = row(
                                selected,
                                doc,
                                span.start,
                                source,
                                &target,
                                None,
                                request.kind,
                            )?
                            && seen.insert(result.id.clone())
                        {
                            if rows.len() < 256 {
                                rows.push(result);
                            } else {
                                omitted += 1;
                            }
                        }
                    }
                    if let Some(next) = calls.next {
                        offset = next;
                    } else {
                        break;
                    }
                }
            }
            "name_candidates"
        };
        rows.sort_by(|a, b| {
            (&a.source.path, a.source.line, &a.id).cmp(&(&b.source.path, b.source.line, &b.id))
        });
        if self
            .host
            .application()
            .snapshot()
            .is_none_or(|s| s.id() != snapshot.id())
        {
            return Err(CompanionError::Conflict(
                "Source changed during references".into(),
            ));
        }
        let query = blake3::hash(
            format!(
                "{}:{}:{:?}:{precision}:{}",
                request.snapshot,
                target.id,
                request.kind,
                serde_json::to_string(&rows).map_err(unavailable)?
            )
            .as_bytes(),
        )
        .to_hex()
        .to_string();
        let set = ReferenceSet {
            page: ReferencePage {
                snapshot: request.snapshot.clone(),
                target,
                kind: request.kind,
                precision,
                query,
                matching: rows.len(),
                outside,
                omitted,
                items: vec![],
                next: None,
                detail,
            },
            rows,
        };
        let result = page(&set, 0);
        drop(cache);
        let mut sets = self
            .reference_sets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sets.retain(|s| s.page.query != set.page.query);
        if sets.len() == 4 {
            sets.remove(0);
        }
        sets.push(set);
        Ok(result)
    }
}
