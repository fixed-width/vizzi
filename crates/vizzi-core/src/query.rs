//! A bounded overview and source inspector computed from one snapshot.

use crate::{
    model::{
        BULK_SOURCE_LIMIT, CapturePolicyId, Context, Coverage, Document, DocumentVersion, EntityId,
        FileId, Snapshot, SnapshotId, SourceEncoding, SourceReadError,
    },
    policy::{Classification, SourceOnlyReason},
    projection::Summary,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod browse;
mod search;
pub use browse::{BrowsePage, DirectorySummary, Landmark, LandmarkKind};
pub use search::{Match, MatchPage};

const MAX_MAP_FILES: usize = 2000;

/// Selected-document projection; bulk views remain available for diagnostics.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Projection {
    /// Metadata only, with separately requested pinned windows.
    Summary,
}

/// Browser selection and overview bounds.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ViewQuery {
    /// Selected opaque file identity.
    pub file: Option<String>,
    /// Selected opaque declaration identity.
    pub entity: Option<String>,
    /// Source search text, or a case-insensitive path substring when not finding source.
    #[serde(default)]
    pub filter: String,
    /// Match declaration names as well as file paths.
    #[serde(default)]
    pub find: bool,
    /// Interpret the source search text as a regular expression.
    #[serde(default)]
    pub search_regex: bool,
    /// Match case in source search.
    #[serde(default)]
    pub search_case_sensitive: bool,
    /// Exact directory scope for source search. Absent means repository-wide.
    pub search_directory: Option<String>,
    /// Offset into the ranked search results, independently of the map page.
    #[serde(default)]
    pub results_offset: usize,
    /// Show the map page containing the explicitly selected search match.
    #[serde(default)]
    pub reveal_match: bool,
    /// Comma-separated map file types. A file may match any selected type.
    #[serde(default)]
    pub types: String,
    /// Maximum visible files, clamped to 1–2,000.
    pub limit: Option<usize>,
    /// Request schema-3 metadata without complete text/declarations.
    pub projection: Option<Projection>,
    /// Opt into directory browsing for large snapshots. `.` selects the repository root.
    pub directory: Option<String>,
    /// Requested offset in a directory or path search.
    #[serde(default)]
    pub offset: usize,
}
/// A rectangle in overview world coordinates.
#[derive(Clone, Debug, Serialize)]
pub struct Rect {
    /// Horizontal position.
    pub x: f64,
    /// Vertical position.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}
/// A file tile in the overview.
#[derive(Debug, Serialize)]
pub struct FileTile {
    /// Opaque file identity.
    pub id: FileId,
    /// Relative source path.
    pub path: String,
    /// Landmark role for this file, if recognized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landmark: Option<LandmarkKind>,
    /// Captured source size.
    pub bytes: usize,
    /// Combined syntax coverage.
    pub coverage: Coverage,
    /// Source selection and assignment evidence.
    pub classification: Classification,
    /// File type used by the map legend and filter.
    pub file_type: &'static str,
    /// World-space geometry supplied by the engine.
    pub rect: Rect,
}
/// A directory label preceding its file tiles.
#[derive(Debug, Serialize)]
pub struct Group {
    /// Horizontal coordinate of the directory card.
    pub x: f64,
    /// Card width in world coordinates.
    pub width: f64,
    /// Card height including its heading.
    pub height: f64,
    /// Relative directory.
    pub path: String,
    /// Vertical coordinate of the label.
    pub y: f64,
}
/// A selectable source-backed declaration.
#[derive(Debug, Serialize)]
pub struct Declaration {
    /// Opaque declaration identity.
    pub id: EntityId,
    /// Display name.
    pub name: String,
    /// Lexical declaration kind.
    pub kind: String,
    /// Inclusive one-based starting line.
    pub start_line: usize,
    /// Inclusive one-based ending line.
    pub end_line: usize,
}
/// Facts and coverage for one analysis context.
#[derive(Debug, Serialize)]
pub struct Outline {
    /// Analysis capability, currently raw syntax only.
    pub mode: &'static str,
    /// Build-configuration coverage, currently unknown.
    pub configuration_coverage: &'static str,
    /// Analyzer and configuration identity.
    pub context: Context,
    /// Syntax coverage.
    pub coverage: Coverage,
    /// Recovery or availability explanations.
    pub diagnostics: Vec<String>,
    /// Source-backed lexical declarations.
    pub declarations: Vec<Declaration>,
}
/// Selected source and all its configured outlines.
#[derive(Debug, Serialize)]
pub struct SourceView {
    /// Present only in schema-3 summary views.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Summary>,
    /// Captured document version, permitting reuse across unchanged source projections.
    pub version: DocumentVersion,
    /// Opaque file identity.
    pub id: FileId,
    /// Relative source path.
    pub path: String,
    /// Frozen UTF-8 source, absent when the encoding is unsupported.
    pub text: Option<String>,
    /// Encoding availability, separate from language assignment.
    pub encoding: SourceEncoding,
    /// Captured source classification and provenance.
    pub classification: Classification,
    /// Analysis contexts and declarations.
    pub outlines: Vec<Outline>,
}
/// Reconciled selection that is valid for this response.
#[derive(Debug, Serialize)]
pub struct Selection {
    /// Selected file, absent in an empty workspace/filter.
    pub file: Option<FileId>,
    /// Selected declaration, absent if deleted or ambiguous after an edit.
    pub entity: Option<EntityId>,
}
/// A coherent response suitable for both linked views.
#[derive(Debug, Serialize)]
pub struct View {
    /// Stable opaque workspace identity for local presentation preferences.
    pub workspace: crate::model::WorkspaceId,
    /// Files with at least one deferred analysis, independently of the overview bound.
    pub deferred: usize,
    /// Version of the linked-view response schema.
    pub schema: u32,
    /// Source and fact observation used throughout this response.
    pub snapshot: SnapshotId,
    /// Source policy used to discover this observation.
    pub policy: CapturePolicyId,
    /// Total files in the observation.
    pub total: usize,
    /// Captured files containing an analysis record, regardless of grammar recovery.
    pub analyzed: usize,
    /// Captured files with no analysis records.
    pub source_only: usize,
    /// Number of files matching the filter before limiting.
    pub matching: usize,
    /// File types available in the map scope before filtering or paging.
    pub file_types: Vec<&'static str>,
    /// File counts by type in the map scope before filtering or paging.
    pub file_type_counts: BTreeMap<&'static str, usize>,
    /// Scoped page for directory browsing or search in a large snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browse: Option<BrowsePage>,
    /// Ranked matches for the current source search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matches: Option<MatchPage>,
    /// Bounded overview tiles.
    pub files: Vec<FileTile>,
    /// Directory labels.
    pub groups: Vec<Group>,
    /// World-space height.
    pub height: f64,
    /// Reconciled selection.
    pub selection: Selection,
    /// Selected captured source.
    pub source: Option<SourceView>,
}

struct MapFilter<'a> {
    path: String,
    types: BTreeSet<&'a str>,
    file_type_counts: BTreeMap<&'static str, usize>,
}

impl MapFilter<'_> {
    fn collect_type(&mut self, doc: &Document) -> bool {
        let file_type = map_file_type(doc.classification());
        *self.file_type_counts.entry(file_type).or_default() += 1;
        self.types.is_empty() || self.types.contains(file_type)
    }

    fn collect_type_and_match(&mut self, doc: &Document) -> bool {
        self.collect_type(doc)
            && (self.path.is_empty() || doc.path().to_lowercase().contains(&self.path))
    }
}

fn map_file_type(classification: &Classification) -> &'static str {
    let (language, reason) = match classification {
        Classification::Assigned { language, .. } => (Some(language.as_str()), None),
        Classification::SourceOnly {
            language, reason, ..
        } => (language.as_deref(), Some(*reason)),
    };
    match (language, reason) {
        (Some("tsx"), _) => "typescript",
        (Some("markdown"), _) | (_, Some(SourceOnlyReason::Documentation)) => "documentation",
        (_, Some(SourceOnlyReason::ProjectMetadata)) => "metadata",
        (Some("c"), _) => "c",
        (Some("cpp"), _) => "cpp",
        (Some("cuda"), _) => "cuda",
        (Some("java"), _) => "java",
        (Some("javascript"), _) => "javascript",
        (Some("json"), _) => "json",
        (Some("python"), _) => "python",
        (Some("ruby"), _) => "ruby",
        (Some("rust"), _) => "rust",
        (Some("toml"), _) => "toml",
        (Some("typescript"), _) => "typescript",
        _ => "other",
    }
}

impl Document {
    /// Projects the source metadata needed to inspect this captured document.
    pub fn source_summary(&self, entity: Option<&str>) -> SourceView {
        let summary = Summary {
            lines: self.captured().line_count(),
            declarations: self.declaration_count(),
            active: entity
                .and_then(|id| self.entity_position(id))
                .and_then(|index| self.entry(index)),
        };
        self.source_view(Some(summary), None)
    }

    fn source_view(&self, summary: Option<Summary>, text: Option<String>) -> SourceView {
        let declaration_limit = if summary.is_some() { 0 } else { usize::MAX };
        SourceView {
            summary,
            text,
            version: self.version().clone(),
            id: self.id().clone(),
            path: self.path().into(),
            encoding: self.encoding(),
            classification: self.classification().clone(),
            outlines: self
                .analyses()
                .iter()
                .map(|analysis| Outline {
                    mode: "raw_syntax",
                    configuration_coverage: "unknown",
                    context: analysis.context.clone(),
                    coverage: analysis.coverage.clone(),
                    diagnostics: analysis.diagnostics.clone(),
                    declarations: analysis
                        .facts
                        .iter()
                        .take(declaration_limit)
                        .map(|fact| {
                            let lines = self.lines(&fact.span);
                            Declaration {
                                id: self.entity_id(analysis, fact),
                                name: fact.name.clone(),
                                kind: fact.kind.clone(),
                                start_line: lines.start,
                                end_line: lines.end,
                            }
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

impl Snapshot {
    /// Computes a linked view without reading mutable source files.
    pub fn view(&self, query: &ViewQuery) -> Result<View, SourceReadError> {
        self.view_with_limit(query, BULK_SOURCE_LIMIT)
    }
    /// Computes a view with an explicit bulk-source allowance; summaries perform no source IO.
    pub fn view_with_limit(
        &self,
        query: &ViewQuery,
        source_limit: usize,
    ) -> Result<View, SourceReadError> {
        let mut filter = MapFilter {
            path: query.filter.to_lowercase(),
            types: query
                .types
                .split(',')
                .filter(|kind| !kind.is_empty())
                .collect(),
            file_type_counts: BTreeMap::new(),
        };
        let mut matches = None;
        let search_text = if query.search_regex {
            query.filter.as_str()
        } else {
            query.filter.trim()
        };
        let (count, limited, browse) = match &query.directory {
            _ if query.find && !search_text.is_empty() => {
                let path = query.search_directory.as_deref().unwrap_or(".");
                let (results, mut files) = search::project(
                    browse::exact_scope(self.documents(), path),
                    &mut filter,
                    query,
                );
                matches = Some(results);
                let count = files.len();
                if self.documents().len() > MAX_MAP_FILES {
                    let reveal = query
                        .reveal_match
                        .then_some(query.file.as_deref())
                        .flatten();
                    let (page, files) = browse::file_page(path, files, query.offset, reveal);
                    (count, files, Some(page))
                } else {
                    files.truncate(query.limit.unwrap_or(MAX_MAP_FILES).clamp(1, MAX_MAP_FILES));
                    (count, files, None)
                }
            }
            Some(path) if self.documents().len() > MAX_MAP_FILES => {
                let (page, documents) = browse::project(self, path, &mut filter, query.offset);
                (page.files, documents, Some(page))
            }
            _ => {
                let matching: Vec<_> = self
                    .documents()
                    .iter()
                    .map(std::sync::Arc::as_ref)
                    .filter(|file| filter.collect_type_and_match(file))
                    .collect();
                let count = matching.len();
                let mut limited = matching;
                limited.truncate(query.limit.unwrap_or(MAX_MAP_FILES).clamp(1, MAX_MAP_FILES));
                (count, limited, None)
            }
        };
        let selected = query
            .file
            .as_ref()
            .and_then(|id| self.file(id).map(std::sync::Arc::as_ref))
            .or_else(|| limited.first().copied());
        let source = selected
            .map(|file| -> Result<SourceView, SourceReadError> {
                if query.projection == Some(Projection::Summary) {
                    return Ok(file.source_summary(query.entity.as_deref()));
                }
                let text = file
                    .captured()
                    .read_text(source_limit)?
                    .map(|text| text.into_owned());
                Ok(file.source_view(None, text))
            })
            .transpose()?;
        let entity = source
            .as_ref()
            .and_then(|source| source.summary.as_ref())
            .and_then(|summary| summary.active.as_ref())
            .map(|entry| entry.id.clone())
            .or_else(|| {
                source
                    .iter()
                    .flat_map(|source| &source.outlines)
                    .flat_map(|outline| &outline.declarations)
                    .find(|fact| query.entity.as_deref() == Some(fact.id.as_str()))
                    .map(|fact| fact.id.clone())
            });
        let mut directories = BTreeMap::<&str, Vec<_>>::new();
        for file in limited {
            directories
                .entry(
                    file.path()
                        .rsplit_once('/')
                        .map_or(".", |(parent, _)| parent),
                )
                .or_default()
                .push(file);
        }
        let mut files = Vec::new();
        let mut groups = Vec::new();
        let area = directories
            .values()
            .map(|docs| {
                let columns = ((docs.len() as f64 * 56.0 / 178.0).sqrt().ceil() as usize).max(1);
                columns as f64 * 178.0 * (48.0 + docs.len().div_ceil(columns) as f64 * 56.0)
            })
            .sum::<f64>();
        let columns = ((area.sqrt() / 178.0).ceil() as usize).clamp(1, 64);
        let target_width = columns as f64 * 178.0;
        let (mut x, mut y, mut shelf_height) = (0.0, 0.0, 0.0_f64);
        for (path, documents) in directories {
            let cols =
                ((documents.len() as f64 * 56.0 / 178.0).sqrt().ceil() as usize).clamp(1, columns);
            let width = cols as f64 * 178.0;
            let height = 30.0 + documents.len().div_ceil(cols) as f64 * 56.0 + 18.0;
            if x > 0.0 && x + width > target_width {
                y += shelf_height;
                x = 0.0;
                shelf_height = 0.0;
            }
            groups.push(Group {
                path: path.into(),
                x,
                y,
                width,
                height,
            });
            for (i, file) in documents.iter().enumerate() {
                let coverage = if file
                    .analyses()
                    .iter()
                    .any(|analysis| analysis.coverage == Coverage::Deferred)
                {
                    Coverage::Deferred
                } else if file.analyses().is_empty()
                    || file
                        .analyses()
                        .iter()
                        .any(|analysis| analysis.coverage == Coverage::Unavailable)
                {
                    Coverage::Unavailable
                } else if file
                    .analyses()
                    .iter()
                    .any(|analysis| analysis.coverage == Coverage::Partial)
                {
                    Coverage::Partial
                } else {
                    Coverage::Syntax
                };
                files.push(FileTile {
                    id: file.id().clone(),
                    path: file.path().into(),
                    landmark: LandmarkKind::for_document(file),
                    bytes: file.bytes(),
                    coverage,
                    classification: file.classification().clone(),
                    file_type: map_file_type(file.classification()),
                    rect: Rect {
                        x: x + (i % cols) as f64 * 178.0,
                        y: y + 30.0 + (i / cols) as f64 * 56.0,
                        width: 170.0,
                        height: 48.0,
                    },
                });
            }
            x += width;
            shelf_height = shelf_height.max(height);
        }
        if browse.is_some() {
            files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
        }
        Ok(View {
            workspace: self.workspace().clone(),
            deferred: self.deferred_files(),
            schema: if query.projection.is_some() { 3 } else { 2 },
            snapshot: self.id().clone(),
            policy: self.policy().clone(),
            total: self.documents().len(),
            analyzed: self.analyzed_files(),
            source_only: self.documents().len() - self.analyzed_files(),
            matching: count,
            file_types: filter.file_type_counts.keys().copied().collect(),
            file_type_counts: filter.file_type_counts,
            browse,
            matches,
            files,
            groups,
            height: y + shelf_height,
            selection: Selection {
                file: selected.map(|file| file.id().clone()),
                entity,
            },
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::WorkspaceId, rust::RustAnalyzer};
    use std::sync::Arc;
    fn snapshot(text: &str) -> Snapshot {
        let workspace = WorkspaceId::derive(&[b"t"]);
        let analysis = RustAnalyzer::new()
            .unwrap()
            .analyze(text.as_bytes())
            .unwrap();
        let a = crate::model::fixture_document(
            workspace.clone(),
            "src/a.rs".into(),
            text.as_bytes(),
            vec![analysis],
        )
        .unwrap();
        let b = crate::model::fixture_document(workspace.clone(), "other/b.rs".into(), b"", vec![])
            .unwrap();
        crate::model::fixture_snapshot(workspace, vec![Arc::new(a), Arc::new(b)]).unwrap()
    }

    #[test]
    fn source_search_projects_ranked_declarations_and_distinct_matching_files() {
        let snapshot = snapshot("fn read_extra() {}\nmod reader { fn read() {} }");
        let view = |query| {
            let query = serde_json::from_value(query).unwrap();
            serde_json::to_value(snapshot.view(&query).unwrap()).unwrap()
        };
        let qualified = view(serde_json::json!({"filter": "READER.read", "find": true}));
        assert_eq!(qualified["matching"], 1);
        assert_eq!(qualified["files"][0]["path"], "src/a.rs");
        let row = &qualified["matches"]["items"][0];
        assert_eq!(row["name"], "reader::read");
        assert_eq!(row["line"], 2);
        let picked = view(serde_json::json!({
            "projection": "summary", "file": row["file"], "entity": row["entity"]
        }));
        assert_eq!(picked["selection"]["entity"], row["entity"]);
        assert_eq!(picked["source"]["summary"]["active"]["name"], "read");

        let ranked = view(serde_json::json!({"filter": "read", "find": true}));
        assert_eq!(ranked["matching"], 1);
        assert_eq!(ranked["matches"]["total"], 3);
        assert_eq!(ranked["matches"]["items"][0]["name"], "reader::read");
        assert!(
            ranked["matches"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["kind"] == "module")
        );

        let path = view(serde_json::json!({"filter": "other/b", "find": true}));
        assert_eq!(path["matches"]["total"], 1);
        assert_eq!(path["matches"]["items"][0]["kind"], "file");
        let scoped = view(serde_json::json!({
            "filter": "read", "find": true, "search_directory": "other"
        }));
        assert_eq!(scoped["matches"]["total"], 0);
        assert_eq!(scoped["matching"], 0);
        let filtered = view(serde_json::json!({"filter": "read", "find": true, "types": "python"}));
        assert_eq!(filtered["matches"]["total"], 0);
        assert_eq!(filtered["matching"], 0);
    }

    #[test]
    fn source_search_matches_case_in_literal_names_qualification_and_paths() {
        let snapshot = snapshot("mod Reader { fn read() {} fn Read() {} }");
        let results = |filter, case_sensitive| {
            let query = serde_json::from_value(serde_json::json!({
                "find": true, "filter": filter, "search_case_sensitive": case_sensitive
            }))
            .unwrap();
            snapshot
                .view(&query)
                .unwrap()
                .matches
                .unwrap()
                .items
                .into_iter()
                .map(|hit| hit.name)
                .collect::<Vec<_>>()
        };
        assert!(results("READER#read", false).contains(&"Reader::Read".into()));
        assert_eq!(results("Reader.read", true), ["Reader::read"]);
        assert!(results("reader::read", true).is_empty());
        assert_eq!(results("src/a.rs", true), ["a.rs"]);
        assert!(results("SRC/a.rs", true).is_empty());
    }

    #[test]
    fn source_search_regex_matches_names_and_paths_with_independent_case_and_scope() {
        let snapshot = snapshot("mod Reader { fn read() {} fn Read() {} }");
        let pattern = r"^Reader::Read$|^OTHER/b\.rs$|^a\.rs$";
        let view = |params| {
            let query = serde_json::from_value(params).unwrap();
            serde_json::to_value(snapshot.view(&query).unwrap()).unwrap()
        };
        let insensitive =
            view(serde_json::json!({"find": true, "filter": pattern, "search_regex": true}));
        assert_eq!(insensitive["matching"], 2);
        assert_eq!(insensitive["matches"]["total"], 4);

        let sensitive = view(serde_json::json!({
            "find": true, "filter": pattern, "search_regex": true, "search_case_sensitive": true
        }));
        assert_eq!(sensitive["matching"], 1);
        assert_eq!(sensitive["matches"]["total"], 2);
        assert_eq!(sensitive["matches"]["items"][0]["name"], "a.rs");
        assert_eq!(sensitive["matches"]["items"][1]["name"], "Reader::Read");

        let scoped = view(serde_json::json!({
            "find": true, "filter": pattern, "search_regex": true, "search_directory": "other"
        }));
        assert_eq!(scoped["matches"]["total"], 1);
        assert_eq!(scoped["files"][0]["path"], "other/b.rs");

        let whitespace = view(serde_json::json!({
            "find": true, "filter": " ", "search_regex": true
        }));
        assert_eq!(whitespace["matches"]["total"], 0);
    }

    #[test]
    fn source_search_reports_invalid_regex_and_keeps_the_selected_source() {
        let snapshot = snapshot("fn read() {}");
        let file = snapshot.documents()[0].id();
        let view = |params| {
            let query = serde_json::from_value(params).unwrap();
            serde_json::to_value(snapshot.view(&query).unwrap()).unwrap()
        };
        let invalid = view(serde_json::json!({
            "find": true, "filter": "[", "search_regex": true, "file": file
        }));
        assert!(
            invalid["matches"]["error"]
                .as_str()
                .unwrap()
                .contains("unclosed character class")
        );
        assert_eq!(invalid["files"], serde_json::json!([]));
        assert_eq!(invalid["matches"]["total"], 0);
        assert_eq!(invalid["source"]["id"], serde_json::json!(file));
        let literal = view(serde_json::json!({"find": true, "filter": "["}));
        assert!(literal["matches"]["error"].is_null());
        let path_only = view(serde_json::json!({
            "filter": "SRC", "search_regex": true, "search_case_sensitive": true
        }));
        assert!(path_only["matches"].is_null());
        assert_eq!(path_only["files"][0]["path"], "src/a.rs");
    }

    #[test]
    fn directory_browsing_counts_all_captured_paths_and_pages_flat_children() {
        let workspace = WorkspaceId::derive(&[b"browse"]);
        let paths = (0..2001)
            .map(|number| format!("bulk/file-{number:04}.rs"))
            .chain(["late/README.md".into(), "late/src/main.rs".into()])
            .chain((0..99).map(|number| format!("wide/dir-{number:02}/lib.rs")))
            .chain([
                "wide/Cargo.toml".into(),
                "wide/README.md".into(),
                "wide/z.rs".into(),
            ]);
        let documents = paths
            .map(|path| {
                let (source, analyses): (&[u8], _) = if path == "late/src/main.rs" {
                    let source = b"fn main() {}";
                    let analysis = RustAnalyzer::new().unwrap().analyze(source).unwrap();
                    (source, vec![analysis])
                } else {
                    (b"", vec![])
                };
                Arc::new(
                    crate::model::fixture_document(workspace.clone(), path, source, analyses)
                        .unwrap(),
                )
            })
            .collect();
        let snapshot = crate::model::fixture_snapshot(workspace, documents).unwrap();
        let view = |params| {
            let query: ViewQuery = serde_json::from_value(params).unwrap();
            serde_json::to_value(snapshot.view(&query).unwrap()).unwrap()
        };

        let root = view(serde_json::json!({"directory": "."}));
        assert_eq!(
            root["browse"]["directories"],
            serde_json::json!([
                {"path": "bulk", "files": 2001}, {"path": "late", "files": 2},
                {"path": "wide", "files": 102}
            ])
        );
        assert_eq!(root["files"].as_array().unwrap().len(), 0);
        let root_landmarks = root["browse"]["landmarks"].as_array().unwrap();
        assert_eq!(root_landmarks.len(), 1);
        assert_eq!(root_landmarks[0]["path"], "late/src/main.rs");
        assert_eq!(root_landmarks[0]["kind"], "entry_point");
        let last = view(serde_json::json!({"directory": "bulk", "offset": 2000}));
        assert_eq!(last["browse"]["offset"], 2000);
        assert_eq!(last["browse"]["entries"], 2001);
        assert_eq!(last["files"][0]["path"], "bulk/file-2000.rs");
        assert_eq!(last["browse"]["landmarks"], serde_json::json!([]));
        let late = view(serde_json::json!({"directory": "late", "offset": 2000}));
        assert_eq!(late["browse"]["offset"], 0);
        assert_eq!(late["browse"]["landmarks"][0]["path"], "late/README.md");
        assert_eq!(late["browse"]["directories"][0]["path"], "late/src");
        let wide = view(serde_json::json!({"directory": "wide"}));
        assert_eq!(wide["browse"]["directories"].as_array().unwrap().len(), 99);
        assert_eq!(wide["files"].as_array().unwrap().len(), 1);
        assert_eq!(wide["browse"]["landmarks"][1]["path"], "wide/Cargo.toml");
        let next_wide = view(serde_json::json!({"directory": "wide", "offset": 100}));
        assert_eq!(next_wide["files"][0]["path"], "wide/README.md");
        assert_eq!(next_wide["files"][1]["path"], "wide/z.rs");
        let fallback = view(serde_json::json!({"directory": "late/removed"}));
        assert_eq!(fallback["browse"]["directory"], "late");
        let root_fallback = view(serde_json::json!({"directory": "missing"}));
        assert_eq!(root_fallback["browse"]["directory"], ".");
        let searched = view(serde_json::json!({"directory": "bulk", "filter": "main.rs"}));
        assert_eq!(searched["files"][0]["path"], "late/src/main.rs");
        let search_page =
            view(serde_json::json!({"directory": "late", "filter": "FILE-", "offset": 1999}));
        assert_eq!(search_page["files"][0]["path"], "bulk/file-1900.rs");
        let ranked = view(serde_json::json!({
            "find": true, "filter": "file-", "search_directory": "bulk",
            "results_offset": 1999
        }));
        assert_eq!(ranked["matches"]["offset"], 1992);
        assert_eq!(ranked["matches"]["total"], 2001);
        assert_eq!(ranked["browse"]["offset"], 0);
        let selected = ranked["matches"]["items"]
            .as_array()
            .unwrap()
            .last()
            .unwrap();
        let revealed = view(serde_json::json!({
            "find": true, "filter": "file-", "search_directory": "bulk",
            "results_offset": 1999, "file": selected["file"], "reveal_match": true
        }));
        assert_eq!(revealed["browse"]["offset"], 2000);
        assert_eq!(revealed["files"][0]["path"], "bulk/file-2000.rs");
        assert_eq!(revealed["matches"]["items"], ranked["matches"]["items"]);
    }
    #[test]
    fn selection_uses_frozen_source_and_survives_position_changes() {
        let first = snapshot("fn f() {}");
        let view = first
            .view(&ViewQuery {
                filter: "src".into(),
                ..Default::default()
            })
            .unwrap();
        let entity = view.source.as_ref().unwrap().outlines[0].declarations[0]
            .id
            .as_str()
            .to_owned();
        let query = ViewQuery {
            file: Some(view.selection.file.unwrap().as_str().into()),
            entity: Some(entity.clone()),
            ..Default::default()
        };
        let next = snapshot("\nfn f() {}").view(&query).unwrap();
        assert_eq!(
            next.selection.entity.as_ref().map(EntityId::as_str),
            Some(entity.as_str())
        );
        assert_eq!(
            next.source.as_ref().unwrap().outlines[0].declarations[0].start_line,
            2
        );
        assert_eq!(
            first.view(&query).unwrap().source.unwrap().text.as_deref(),
            Some("fn f() {}")
        );
        assert!(
            snapshot("fn g() {}")
                .view(&query)
                .unwrap()
                .selection
                .entity
                .is_none()
        );
    }
    #[test]
    fn duplicate_declaration_selection_falls_back_after_content_changes() {
        let view = snapshot("fn f() {} fn f() {}")
            .view(&ViewQuery {
                filter: "src".into(),
                ..Default::default()
            })
            .unwrap();
        let query = ViewQuery {
            file: Some(view.selection.file.unwrap().as_str().into()),
            entity: Some(
                view.source.unwrap().outlines[0].declarations[0]
                    .id
                    .as_str()
                    .into(),
            ),
            ..Default::default()
        };
        let next = snapshot("\nfn f() {} fn f() {}").view(&query).unwrap();
        assert!(next.selection.file.is_some());
        assert!(next.selection.entity.is_none());
    }
    #[test]
    fn limits_filters_and_missing_selection_are_explicit() {
        let snapshot = snapshot("fn f() {}");
        let view = snapshot
            .view(&ViewQuery {
                file: Some("removed".into()),
                filter: "SRC".into(),
                limit: Some(0),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(view.files.len(), 1);
        assert_eq!(view.matching, 1);
        assert_eq!(view.total, 2);
        assert_eq!(view.source.unwrap().path, "src/a.rs");
        let view = snapshot
            .view(&ViewQuery {
                filter: "absent".into(),
                ..Default::default()
            })
            .unwrap();
        assert!(view.source.is_none());
        assert!(view.selection.file.is_none());
        assert!(view.files.is_empty());
        let workspace = WorkspaceId::derive(&[b"empty"]);
        assert!(
            crate::model::fixture_snapshot(workspace, vec![])
                .unwrap()
                .view(&ViewQuery::default())
                .unwrap()
                .source
                .is_none()
        );
    }

    fn classified_snapshot(paths: impl IntoIterator<Item = String>) -> Snapshot {
        let workspace = WorkspaceId::derive(&[b"map-types"]);
        let policy = crate::policy::SourcePolicy::new(
            crate::policy::PolicyConfig {
                c_family_roots: vec!["include".into()],
                ..Default::default()
            },
            [],
        )
        .unwrap();
        let documents = paths
            .into_iter()
            .map(|path| {
                let classification = policy.classify(&path).unwrap().unwrap();
                Arc::new(
                    Document::capture(workspace.clone(), path, b"", vec![], classification)
                        .unwrap(),
                )
            })
            .collect();
        crate::model::fixture_snapshot(workspace, documents).unwrap()
    }

    #[test]
    fn map_tiles_expose_classified_types_for_legend_colors() {
        let expected = [
            ("Example.java", "java"),
            ("README", "documentation"),
            ("config.toml", "toml"),
            ("data.json", "json"),
            ("guide.markdown", "documentation"),
            ("include/a.c", "c"),
            ("include/b.cc", "cpp"),
            ("include/c.cuh", "cuda"),
            ("include/d.h", "other"),
            ("main.js", "javascript"),
            ("main.py", "python"),
            ("main.rb", "ruby"),
            ("main.rs", "rust"),
            ("main.ts", "typescript"),
            ("main.tsx", "typescript"),
            ("package.json", "metadata"),
        ];
        let snapshot = classified_snapshot(expected.map(|(path, _)| path.to_owned()));
        let view = snapshot.view(&ViewQuery::default()).unwrap();
        let tiles: BTreeMap<_, _> = view
            .files
            .iter()
            .map(|tile| (tile.path.as_str(), tile.file_type))
            .collect();
        assert_eq!(tiles, BTreeMap::from(expected));
    }

    #[test]
    fn map_type_union_combines_with_path_search_and_preserves_legend_choices() {
        let snapshot = classified_snapshot(
            [
                "src/Cargo.toml",
                "src/README.txt",
                "src/example.tsx",
                "src/guide.md",
                "src/program.py",
                "src/settings.toml",
                "tests/example.ts",
            ]
            .map(str::to_owned),
        );
        let query = serde_json::from_value(serde_json::json!({
            "types": "typescript,documentation",
            "filter": "SRC/"
        }))
        .unwrap();
        let view = serde_json::to_value(snapshot.view(&query).unwrap()).unwrap();
        assert_eq!(view["matching"], 3);
        let tiles = view["files"].as_array().unwrap();
        assert_eq!(
            tiles
                .iter()
                .map(|tile| (
                    tile["path"].as_str().unwrap(),
                    tile["file_type"].as_str().unwrap()
                ))
                .collect::<Vec<_>>(),
            [
                ("src/README.txt", "documentation"),
                ("src/example.tsx", "typescript"),
                ("src/guide.md", "documentation"),
            ]
        );
        assert_eq!(
            view["file_types"],
            serde_json::json!(["documentation", "metadata", "python", "toml", "typescript"])
        );
        assert_eq!(
            view["file_type_counts"],
            serde_json::json!({"documentation": 2, "metadata": 1, "python": 1, "toml": 1, "typescript": 2})
        );
    }

    #[test]
    fn directory_type_filter_runs_before_paging_and_counts_matching_descendants() {
        let snapshot = classified_snapshot(
            (0..2001)
                .map(|i| format!("bulk/file-{i:04}.rs"))
                .chain(["bulk/z.py", "mixed/README.md", "mixed/main.rs"].map(str::to_owned)),
        );
        let view = |params| {
            let query = serde_json::from_value(params).unwrap();
            serde_json::to_value(snapshot.view(&query).unwrap()).unwrap()
        };
        let root = view(serde_json::json!({"directory": ".", "types": "python"}));
        assert_eq!(root["browse"]["files"], 1);
        assert_eq!(
            root["browse"]["directories"],
            serde_json::json!([{"path": "bulk", "files": 1}])
        );
        let bulk =
            view(serde_json::json!({"directory": "bulk", "types": "python", "offset": 2000}));
        assert_eq!(bulk["browse"]["offset"], 0);
        assert_eq!(bulk["files"][0]["path"], "bulk/z.py");
        assert_eq!(bulk["file_types"], serde_json::json!(["python", "rust"]));
        assert_eq!(
            bulk["file_type_counts"],
            serde_json::json!({"python": 1, "rust": 2001})
        );
        let mixed = view(serde_json::json!({"directory": "mixed", "types": "rust"}));
        assert_eq!(mixed["browse"]["landmarks"], serde_json::json!([]));
        let searched =
            view(serde_json::json!({"directory": "mixed", "types": "python", "filter": "bulk/"}));
        assert_eq!(searched["browse"]["entries"], 1);
        assert_eq!(searched["files"][0]["path"], "bulk/z.py");
    }

    #[test]
    fn tiles_never_overlap_directories_or_each_other_and_stay_in_bounds() {
        let workspace = WorkspaceId::derive(&[b"layout"]);
        let mut files = Vec::new();
        for directory in ["first", "second"] {
            for i in 0..17 {
                files.push(Arc::new(
                    crate::model::fixture_document(
                        workspace.clone(),
                        format!("{directory}/{i:02}.rs"),
                        b"abc",
                        vec![],
                    )
                    .unwrap(),
                ));
            }
        }
        let view = crate::model::fixture_snapshot(workspace, files)
            .unwrap()
            .view(&ViewQuery::default())
            .unwrap();
        assert_eq!(view.groups.len(), 2);
        assert!(
            view.height < (view.files.len() + view.groups.len()) as f64 * 80.0,
            "layout wastes excessive vertical space"
        );
        assert_eq!(view.files[0].rect.x, 0.0);
        assert!(view.groups.iter().all(|g| g.width > 0.0 && g.height > 0.0));
        for file in &view.files {
            let r = &file.rect;
            assert_eq!(file.bytes, 3);
            assert_eq!(file.coverage, Coverage::Unavailable);
            assert!(r.x >= 0.0 && r.width > 0.0 && r.height > 0.0 && r.x + r.width <= 1424.0);
            assert!(r.y + r.height < view.height);
            let group = view
                .groups
                .iter()
                .find(|group| file.path.starts_with(&group.path))
                .unwrap();
            assert!(r.x >= group.x && r.x + r.width <= group.x + group.width);
            assert!(r.y >= group.y + 20.0 && r.y + r.height <= group.y + group.height);
            for other in &view.files {
                if file.id == other.id {
                    continue;
                }
                let b = &other.rect;
                assert!(
                    r.x + r.width <= b.x
                        || b.x + b.width <= r.x
                        || r.y + r.height <= b.y
                        || b.y + b.height <= r.y,
                    "overlap: {} and {}",
                    file.path,
                    other.path
                );
            }
        }
    }

    #[test]
    fn large_maps_balance_directory_cards_in_both_dimensions() {
        for variant in 0..3 {
            let workspace = WorkspaceId::derive(&[b"balanced-layout"]);
            let paths: Vec<_> = match variant {
                0 => (0..600).map(|i| format!("src/{i}.rs")).collect(),
                1 => (0..600).map(|i| format!("d{i}/a.rs")).collect(),
                _ => (0..64)
                    .flat_map(|d| (0..(d * 17 % 31 + 1)).map(move |i| format!("d{d}/{i}.rs")))
                    .collect(),
            };
            let files = paths
                .into_iter()
                .map(|path| {
                    Arc::new(
                        crate::model::fixture_document(workspace.clone(), path, b"a", vec![])
                            .unwrap(),
                    )
                })
                .collect();
            let view = crate::model::fixture_snapshot(workspace, files)
                .unwrap()
                .view(&ViewQuery::default())
                .unwrap();
            let width = view
                .groups
                .iter()
                .map(|g| g.x + g.width)
                .fold(0.0, f64::max);
            assert!(
                width / view.height > 0.4 && width / view.height < 2.5,
                "unbalanced {width} x {}",
                view.height
            );
            for a in &view.groups {
                for b in &view.groups {
                    if a.path != b.path {
                        assert!(
                            a.x + a.width <= b.x
                                || b.x + b.width <= a.x
                                || a.y + a.height <= b.y
                                || b.y + b.height <= a.y
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn coverage_and_absolute_limits_are_preserved() {
        let workspace = WorkspaceId::derive(&[b"coverage"]);
        let mut parser = RustAnalyzer::new().unwrap();
        let mut files = Vec::new();
        for (path, bytes) in [
            ("good.rs", b"fn good() {}".as_slice()),
            ("partial.rs", b"fn broken( {".as_slice()),
            ("invalid.rs", &[0xff]),
        ] {
            let analysis = parser.analyze(bytes).unwrap();
            files.push(Arc::new(
                crate::model::fixture_document(
                    workspace.clone(),
                    path.into(),
                    bytes,
                    vec![analysis],
                )
                .unwrap(),
            ));
        }
        let snapshot = crate::model::fixture_snapshot(workspace.clone(), files).unwrap();
        let view = snapshot.view(&ViewQuery::default()).unwrap();
        assert_eq!(
            view.files
                .iter()
                .map(|file| &file.coverage)
                .collect::<Vec<_>>(),
            [
                &Coverage::Syntax,
                &Coverage::Unavailable,
                &Coverage::Partial
            ]
        );
        assert_eq!(view.workspace, workspace);
        assert_eq!(view.analyzed, 3);
        assert_eq!(view.source_only, 0);
        let selected = view.files[2].id.as_str().to_owned();
        let limited = snapshot
            .view(&ViewQuery {
                file: Some(selected),
                filter: "good".into(),
                limit: Some(1),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(limited.source.unwrap().path, "partial.rs");
        let files = (0..2003)
            .map(|i| {
                Arc::new(
                    crate::model::fixture_document(
                        workspace.clone(),
                        format!("{i}.rs"),
                        b"",
                        vec![],
                    )
                    .unwrap(),
                )
            })
            .collect();
        let view = crate::model::fixture_snapshot(workspace, files)
            .unwrap()
            .view(&ViewQuery {
                limit: Some(usize::MAX),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(view.matching, 2003);
        assert_eq!(view.files.len(), 2000);
    }
}
