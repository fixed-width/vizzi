//! Bounded orientation from captured paths and lexical names, without source IO.
use crate::{
    comparison::ReviewPath,
    model::{Coverage, Document, DocumentVersion, FileId, Snapshot, SnapshotId, WorkspaceId},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Maximum areas or source matches returned per page.
pub const PAGE_SIZE: usize = 24;
/// Directory focus and optional name/path search, independent of editor state.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DiscoveryQuery {
    /// Normalised relative directory; empty means the repository root.
    #[serde(default)]
    pub path: String,
    /// Case-insensitive substring of captured declaration names or paths.
    #[serde(default)]
    pub search: String,
    /// Offset into the deterministic result order.
    #[serde(default)]
    pub offset: usize,
}
/// Exact location for an on-demand preview or one-click source capture.
#[derive(Debug, Serialize)]
pub struct Location {
    /// Opaque captured file identity.
    pub file: FileId,
    /// Required document version.
    pub version: DocumentVersion,
    /// One-based source line, independent of a display label.
    pub line: usize,
    /// Declaration ordinal when this is a name match.
    pub index: Option<usize>,
    /// Lexical declaration kind, or `file` for a file match.
    pub kind: String,
}
/// One directory area, source file or declaration match.
#[derive(Debug, Serialize)]
pub struct DiscoveryEntry {
    /// Bounded display name; paths and identities remain separate.
    pub label: String,
    /// Exact relative path for navigation and display.
    pub path: String,
    /// Captured files within this item.
    pub files: usize,
    /// Captured source bytes within this item.
    pub bytes: usize,
    /// File counts by assigned language; unassigned source is explicit.
    pub languages: BTreeMap<String, usize>,
    /// Up to three captured names as orientation clues, not inferred roles.
    pub examples: Vec<String>,
    /// Absent for a directory; present for source-backed entries.
    pub location: Option<Location>,
}
/// One finite metadata-only view of an immutable source scope.
#[derive(Debug, Serialize)]
pub struct Discovery {
    /// Workspace namespace for this view.
    pub workspace: WorkspaceId,
    /// Exact observation used for every returned location.
    pub snapshot: SnapshotId,
    /// Focused directory.
    pub path: String,
    /// Files in the complete captured source scope.
    pub total_files: usize,
    /// Files below the focused directory, before searching.
    pub scope_files: usize,
    /// Files without any analysis records in this directory.
    pub source_only: usize,
    /// Files with deferred analysis in this directory.
    pub deferred: usize,
    /// Language counts across the focused directory.
    pub languages: BTreeMap<String, usize>,
    /// All matching items, including those on later pages.
    pub matching: usize,
    /// At most one page of source areas or matches.
    pub entries: Vec<DiscoveryEntry>,
    /// Continuation offset, absent when the result is exhausted.
    pub next: Option<usize>,
}
/// Refused query bounds or a non-normalised directory.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct DiscoveryError(pub String);

fn label(value: &str) -> String {
    let mut end = value.len().min(128);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].into()
}
fn language(doc: &Document) -> &str {
    doc.classification()
        .assigned_language()
        .unwrap_or("unassigned")
}
fn file_entry(
    doc: &Document,
    name: &str,
    kind: &str,
    line: usize,
    index: Option<usize>,
) -> DiscoveryEntry {
    DiscoveryEntry {
        label: label(name),
        path: doc.path().into(),
        files: 1,
        bytes: doc.bytes(),
        languages: BTreeMap::from([(language(doc).into(), 1)]),
        examples: vec![],
        location: Some(Location {
            file: doc.id().clone(),
            version: doc.version().clone(),
            line,
            index,
            kind: label(kind),
        }),
    }
}
struct Page {
    offset: usize,
    matching: usize,
    entries: Vec<DiscoveryEntry>,
}
impl Page {
    fn push(&mut self, entry: impl FnOnce() -> DiscoveryEntry) {
        if self.matching >= self.offset && self.entries.len() < PAGE_SIZE {
            self.entries.push(entry());
        }
        self.matching += 1;
    }
}
impl Snapshot {
    /// Groups real source containment or searches raw lexical names, without reading text.
    pub fn discover(&self, query: &DiscoveryQuery) -> Result<Discovery, DiscoveryError> {
        if query.path.len() > 4096 || query.search.len() > 256 {
            return Err(DiscoveryError(
                "Discovery path or search exceeds its byte limit".into(),
            ));
        }
        if !query.path.is_empty() {
            ReviewPath::parse(query.path.as_bytes().to_vec())
                .map_err(|e| DiscoveryError(e.to_string()))?;
        }
        let prefix = if query.path.is_empty() {
            String::new()
        } else {
            format!("{}/", query.path)
        };
        let search = query.search.trim().to_lowercase();
        let mut page = Page {
            offset: query.offset,
            matching: 0,
            entries: vec![],
        };
        let mut areas = BTreeMap::<(bool, String), DiscoveryEntry>::new();
        let mut languages = BTreeMap::new();
        let (mut scope_files, mut source_only, mut deferred) = (0, 0, 0);
        for doc in self.documents() {
            let Some(relative) = doc.path().strip_prefix(&prefix) else {
                continue;
            };
            scope_files += 1;
            source_only += usize::from(doc.analyses().is_empty());
            deferred += usize::from(
                doc.analyses()
                    .iter()
                    .any(|a| a.coverage == Coverage::Deferred),
            );
            *languages.entry(language(doc).into()).or_default() += 1;
            if search.is_empty() {
                if let Some((child, _)) = relative.split_once('/') {
                    let path = format!("{prefix}{child}");
                    let area =
                        areas
                            .entry((false, path.clone()))
                            .or_insert_with(|| DiscoveryEntry {
                                label: label(child),
                                path,
                                files: 0,
                                bytes: 0,
                                languages: BTreeMap::new(),
                                examples: vec![],
                                location: None,
                            });
                    area.files += 1;
                    area.bytes += doc.bytes();
                    *area.languages.entry(language(doc).into()).or_default() += 1;
                    for fact in doc.analyses().iter().flat_map(|a| a.facts.iter()).take(3) {
                        let name = label(&fact.name);
                        if area.examples.len() < 3 && !area.examples.contains(&name) {
                            area.examples.push(name);
                        }
                    }
                } else {
                    areas.insert(
                        (true, doc.path().into()),
                        file_entry(doc, relative, "file", 1, None),
                    );
                }
            } else {
                if doc.path().to_lowercase().contains(&search) {
                    page.push(|| {
                        file_entry(
                            doc,
                            doc.path().rsplit('/').next().unwrap_or(doc.path()),
                            "file",
                            1,
                            None,
                        )
                    });
                }
                for (index, fact) in doc
                    .analyses()
                    .iter()
                    .flat_map(|a| a.facts.iter())
                    .enumerate()
                {
                    if fact.name.to_lowercase().contains(&search) {
                        page.push(|| {
                            file_entry(
                                doc,
                                &fact.name,
                                &fact.kind,
                                doc.lines(&fact.span).start,
                                Some(index),
                            )
                        });
                    }
                }
            }
        }
        for area in areas.into_values() {
            page.push(|| area);
        }
        let next = query
            .offset
            .checked_add(page.entries.len())
            .filter(|end| *end < page.matching);
        Ok(Discovery {
            workspace: self.workspace().clone(),
            snapshot: self.id().clone(),
            path: query.path.clone(),
            total_files: self.documents().len(),
            scope_files,
            source_only,
            deferred,
            languages,
            matching: page.matching,
            entries: page.entries,
            next,
        })
    }
}
