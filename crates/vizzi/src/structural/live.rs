//! One bounded, coherent working-set refresh over a current source observation.
use super::{Cached, StructureHost};
use crate::companion::CompanionError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use vizzi_core::{
    model::Document,
    projection::{SOURCE_ROWS, WindowQuery},
    structure::Symbol,
};

/// Source-line hint used only to preserve reading position, never to identify a function.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    /// Physical line offset from the earlier declaration start.
    pub offset: usize,
    /// Captured fragment-relative UTF-8 byte at the visible segment start.
    pub byte: usize,
    /// Earlier complete line text, at most 1024 bytes.
    pub text: String,
}
/// Native source binding and presentation position of one live bubble.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    /// Stable presentation identity.
    pub id: String,
    /// Native declaration tracking identity, absent in older saved layouts.
    pub tracking: Option<String>,
    /// Previous version-bound native symbol identity for exact migration.
    pub symbol: String,
    /// Previous captured document version.
    pub version: String,
    /// Earlier exact file identity for lexical-context recovery.
    #[serde(default)]
    pub file: Option<String>,
    /// Earlier analyser context for reconstructing the native tracking hash.
    #[serde(default)]
    pub context: Option<String>,
    /// Original source area used for candidate ordering.
    pub area: String,
    /// Previous section offset from the declaration start.
    pub section_offset: usize,
    /// Optional reading context.
    pub anchor: Option<Anchor>,
}
/// A retained relationship whose current written evidence must be revalidated.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    /// Existing presentation link identity.
    pub id: String,
    /// Source view identity.
    pub from: String,
    /// Destination view identity.
    pub to: String,
    /// Captured written callee spelling.
    pub spelling: String,
    /// Earlier evidence kind, not a fresh semantic assertion.
    pub kind: String,
}
/// A complete live working-set request; pinned views are omitted.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Expected current source observation.
    pub snapshot: String,
    /// At most 24 live bubbles.
    pub views: Vec<View>,
    /// At most 96 retained relationships.
    pub links: Vec<Link>,
}
/// Reading position in the refreshed declaration.
#[derive(Debug, Serialize)]
pub struct Position {
    /// First loaded physical line.
    pub section: usize,
    /// Restored top reading line, when matched.
    pub line: Option<usize>,
    /// Restored segment byte.
    pub byte: usize,
    /// Whether source changes required a reset to the declaration start.
    pub reset: bool,
}
fn opaque(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn invalid(message: &str) -> CompanionError {
    CompanionError::Invalid(message.into())
}
fn unavailable(error: impl ToString) -> CompanionError {
    CompanionError::Unavailable(error.to_string())
}
fn position(doc: &Document, symbol: &Symbol, view: &View) -> Result<Position, CompanionError> {
    let start = symbol.source.line;
    let end = symbol.source.end_line;
    if view.version == symbol.source.version {
        let section = start.saturating_add(view.section_offset).min(end);
        let line = view.anchor.as_ref().map(|a| {
            start
                .saturating_add(a.offset)
                .clamp(section, end.min(section + 95))
        });
        return Ok(Position {
            section,
            line,
            byte: view.anchor.as_ref().map_or(0, |a| a.byte),
            reset: false,
        });
    }
    if let Some(anchor) = &view.anchor
        && !anchor.text.trim().is_empty()
    {
        let expected = start.saturating_add(anchor.offset).min(end);
        let first = expected.saturating_sub(32).max(start);
        let nearby = doc
            .window(
                &WindowQuery {
                    line_start: Some(first),
                    line_count: Some(SOURCE_ROWS.min(end - first + 1)),
                    outline_count: Some(1),
                    ..Default::default()
                },
                None,
            )
            .map_err(unavailable)?;
        let matches = nearby
            .lines
            .iter()
            .filter(|l| !l.left && !l.right && l.number <= end && l.text == anchor.text)
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            let line = matches[0].number;
            let within = anchor.offset.saturating_sub(view.section_offset).min(95);
            let section = line.saturating_sub(within).max(start);
            return Ok(Position {
                section,
                line: Some(line),
                byte: anchor.byte.min(matches[0].text.len()),
                reset: false,
            });
        }
    }
    Ok(Position {
        section: start,
        line: None,
        byte: 0,
        reset: true,
    })
}
impl StructureHost {
    /// Resolves live declarations and prepares their source and call metadata atomically.
    pub fn refresh(&self, request: &Request) -> Result<Value, CompanionError> {
        if !opaque(&request.snapshot) || request.views.len() > 24 || request.links.len() > 96 {
            return Err(invalid("Invalid working-set allowance"));
        }
        let mut ids = BTreeSet::new();
        for view in &request.views {
            if view.id.len() > 8
                || !view.id.starts_with('v')
                || !view.id[1..].bytes().all(|b| b.is_ascii_digit())
                || !ids.insert(&view.id)
                || !opaque(&view.symbol)
                || !opaque(&view.version)
                || view.file.is_some() != view.context.is_some()
                || view.file.as_deref().is_some_and(|s| !opaque(s))
                || view.context.as_deref().is_some_and(|s| !opaque(s))
                || view.tracking.as_deref().is_some_and(|s| !opaque(s))
                || view.area.len() > 4096
                || view.section_offset > 100_000_000
                || view.anchor.as_ref().is_some_and(|a| {
                    a.offset > 100_000_000 || a.byte > 32768 || a.text.len() > 1024
                })
            {
                return Err(invalid("Invalid live view binding"));
            }
        }
        let mut link_ids = BTreeSet::new();
        for link in &request.links {
            if link.id.len() > 1024
                || !link_ids.insert(&link.id)
                || link.from.len() > 8
                || link.to.len() > 8
                || link.spelling.is_empty()
                || link.spelling.len() > 256
                || !matches!(link.kind.as_str(), "self" | "candidate" | "definition")
            {
                return Err(invalid("Invalid live relationship"));
            }
        }
        let snapshot = self
            .host
            .application()
            .snapshot()
            .ok_or_else(|| unavailable("Wait for source indexing"))?;
        if snapshot.id().as_str() != request.snapshot {
            return Err(CompanionError::Conflict(
                "Source changed before refresh".into(),
            ));
        }
        let mut cache = self
            .cache
            .try_lock()
            .map_err(|_| unavailable("Another structural operation is running; retry"))?;
        if cache
            .as_ref()
            .is_none_or(|c| c.snapshot != request.snapshot)
        {
            let previous = cache.take();
            *cache = Some(Cached::rebuild(&snapshot, previous));
        }
        let index = &cache
            .as_ref()
            .ok_or_else(|| unavailable("Structure unavailable"))?
            .index;
        let mut resolved = BTreeMap::new();
        let mut views = Vec::new();
        for view in &request.views {
            let symbol = index
                .resolve(view.tracking.as_deref(), &view.symbol)
                .or_else(|| {
                    index.resolve_context(
                        snapshot.file(view.file.as_deref()?)?,
                        view.tracking.as_deref()?,
                        view.context.as_deref()?,
                    )
                });
            let Some(symbol) = symbol else {
                views.push(json!({"id":view.id,"state":"unresolved"}));
                continue;
            };
            let doc = snapshot
                .file(&symbol.source.file)
                .ok_or_else(|| unavailable("Tracked source unavailable"))?;
            let reading = position(doc, symbol, view)?;
            let changed = view.version != symbol.source.version;
            let (window, origin) = if changed {
                let window = doc
                    .window(
                        &WindowQuery {
                            line_start: Some(reading.section),
                            line_count: Some(
                                SOURCE_ROWS.min(symbol.source.end_line - reading.section + 1),
                            ),
                            outline_count: Some(1),
                            reflow: Some(true),
                            ..Default::default()
                        },
                        None,
                    )
                    .map_err(unavailable)?;
                let header = doc
                    .window(
                        &WindowQuery {
                            line_start: Some(symbol.source.line),
                            line_count: Some(1),
                            outline_count: Some(1),
                            ..Default::default()
                        },
                        None,
                    )
                    .map_err(unavailable)?;
                (Some(window), header.lines.into_iter().next())
            } else {
                (None, None)
            };
            let mut actions = Vec::new();
            let mut offset = 0;
            loop {
                let page = index
                    .actions(
                        &symbol.id,
                        reading.section,
                        SOURCE_ROWS.min(symbol.source.end_line - reading.section + 1),
                        offset,
                    )
                    .map_err(unavailable)?;
                actions.extend(page.items);
                if let Some(next) = page.next {
                    offset = next;
                } else {
                    break;
                }
            }
            let calls = index
                .outgoing(
                    &symbol.id,
                    if view.area.is_empty() {
                        &symbol.source.path
                    } else {
                        &view.area
                    },
                    0,
                )
                .map_err(|e| invalid(&e.to_string()))?;
            views.push(json!({"id":view.id,"state":"current","symbol":symbol,"position":reading,"window":window,"origin":origin,"actions":actions,"calls":calls}));
            resolved.insert(view.id.as_str(), symbol);
        }
        let mut links = Vec::new();
        for link in &request.links {
            let check = match (
                resolved.get(link.from.as_str()),
                resolved.get(link.to.as_str()),
            ) {
                (Some(from), Some(to)) => Some(
                    index
                        .check_link(&from.id, &to.id, &link.spelling, &link.kind)
                        .map_err(unavailable)?,
                ),
                _ => None,
            };
            links.push(json!({"id":link.id,"check":check}));
        }
        if self
            .host
            .application()
            .snapshot()
            .is_none_or(|s| s.id() != snapshot.id())
        {
            return Err(CompanionError::Conflict(
                "Source changed during refresh".into(),
            ));
        }
        Ok(json!({"snapshot":request.snapshot,"views":views,"links":links}))
    }
}
