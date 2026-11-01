//! A bounded overview and source inspector computed from one snapshot.

use crate::model::{Context, Coverage, EntityId, FileId, Snapshot, SnapshotId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Browser selection and overview bounds.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ViewQuery {
    /// Selected opaque file identity.
    pub file: Option<String>,
    /// Selected opaque declaration identity.
    pub entity: Option<String>,
    /// Case-insensitive path substring.
    #[serde(default)]
    pub filter: String,
    /// Maximum visible files, clamped to 1–2,000.
    pub limit: Option<usize>,
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
    /// Captured source size.
    pub bytes: usize,
    /// Combined syntax coverage.
    pub coverage: Coverage,
    /// World-space geometry supplied by the engine.
    pub rect: Rect,
}
/// A directory label preceding its file tiles.
#[derive(Debug, Serialize)]
pub struct Group {
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
    /// Opaque file identity.
    pub id: FileId,
    /// Relative source path.
    pub path: String,
    /// Frozen UTF-8 source, absent when the encoding is unsupported.
    pub text: Option<String>,
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
    /// Version of the linked-view response schema.
    pub schema: u32,
    /// Source and fact observation used throughout this response.
    pub snapshot: SnapshotId,
    /// Total files in the observation.
    pub total: usize,
    /// Number of files matching the filter before limiting.
    pub matching: usize,
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

impl Snapshot {
    /// Computes a linked view without reading mutable source files.
    pub fn view(&self, query: &ViewQuery) -> View {
        let filter = query.filter.to_lowercase();
        let matching: Vec<_> = self
            .documents()
            .iter()
            .filter(|file| file.path().to_lowercase().contains(&filter))
            .collect();
        let count = matching.len();
        let limited: Vec<_> = matching
            .into_iter()
            .take(query.limit.unwrap_or(2000).clamp(1, 2000))
            .collect();
        let selected = query
            .file
            .as_ref()
            .and_then(|id| self.file(id))
            .or_else(|| limited.first().copied());
        let source = selected.map(|file| SourceView {
            id: file.id().clone(),
            path: file.path().into(),
            text: file.source().map(str::to_owned),
            outlines: file
                .analyses()
                .iter()
                .map(|analysis| Outline {
                    context: analysis.context.clone(),
                    coverage: analysis.coverage.clone(),
                    diagnostics: analysis.diagnostics.clone(),
                    declarations: analysis
                        .facts
                        .iter()
                        .map(|fact| {
                            let lines = file.lines(&fact.span);
                            Declaration {
                                id: file.entity_id(analysis, fact),
                                name: fact.name.clone(),
                                kind: fact.kind.clone(),
                                start_line: lines.start,
                                end_line: lines.end,
                            }
                        })
                        .collect(),
                })
                .collect(),
        });
        let entity = source
            .iter()
            .flat_map(|source| &source.outlines)
            .flat_map(|outline| &outline.declarations)
            .find(|fact| query.entity.as_deref() == Some(fact.id.as_str()))
            .map(|fact| fact.id.clone());
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
        let mut y = 0.0;
        for (path, documents) in directories {
            groups.push(Group {
                path: path.into(),
                y,
            });
            y += 30.0;
            for (i, file) in documents.iter().enumerate() {
                let coverage = if file.analyses().is_empty()
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
                    bytes: file.bytes(),
                    coverage,
                    rect: Rect {
                        x: (i % 8) as f64 * 178.0,
                        y: y + (i / 8) as f64 * 56.0,
                        width: 170.0,
                        height: 48.0,
                    },
                });
            }
            y += documents.len().div_ceil(8) as f64 * 56.0 + 18.0;
        }
        View {
            schema: 1,
            snapshot: self.id().clone(),
            total: self.documents().len(),
            matching: count,
            files,
            groups,
            height: y,
            selection: Selection {
                file: selected.map(|file| file.id().clone()),
                entity,
            },
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{Document, WorkspaceId},
        rust::RustAnalyzer,
    };
    use std::sync::Arc;
    fn snapshot(text: &str) -> Snapshot {
        let workspace = WorkspaceId::derive(&[b"t"]);
        let analysis = RustAnalyzer::new()
            .unwrap()
            .analyze(text.as_bytes())
            .unwrap();
        let a = Document::capture(
            workspace.clone(),
            "src/a.rs".into(),
            text.as_bytes(),
            vec![analysis],
        )
        .unwrap();
        let b = Document::capture(workspace.clone(), "other/b.rs".into(), b"", vec![]).unwrap();
        Snapshot::new(workspace, vec![Arc::new(a), Arc::new(b)]).unwrap()
    }
    #[test]
    fn selection_uses_frozen_source_and_survives_position_changes() {
        let first = snapshot("fn f() {}");
        let view = first.view(&ViewQuery {
            filter: "src".into(),
            ..Default::default()
        });
        let entity = view.source.as_ref().unwrap().outlines[0].declarations[0]
            .id
            .as_str()
            .to_owned();
        let query = ViewQuery {
            file: Some(view.selection.file.unwrap().as_str().into()),
            entity: Some(entity.clone()),
            ..Default::default()
        };
        let next = snapshot("\nfn f() {}").view(&query);
        assert_eq!(
            next.selection.entity.as_ref().map(EntityId::as_str),
            Some(entity.as_str())
        );
        assert_eq!(
            next.source.as_ref().unwrap().outlines[0].declarations[0].start_line,
            2
        );
        assert_eq!(
            first.view(&query).source.unwrap().text.as_deref(),
            Some("fn f() {}")
        );
        assert!(
            snapshot("fn g() {}")
                .view(&query)
                .selection
                .entity
                .is_none()
        );
    }
    #[test]
    fn duplicate_declaration_selection_falls_back_after_content_changes() {
        let view = snapshot("fn f() {} fn f() {}").view(&ViewQuery {
            filter: "src".into(),
            ..Default::default()
        });
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
        let next = snapshot("\nfn f() {} fn f() {}").view(&query);
        assert!(next.selection.file.is_some());
        assert!(next.selection.entity.is_none());
    }
    #[test]
    fn limits_filters_and_missing_selection_are_explicit() {
        let snapshot = snapshot("fn f() {}");
        let view = snapshot.view(&ViewQuery {
            file: Some("removed".into()),
            filter: "SRC".into(),
            limit: Some(0),
            ..Default::default()
        });
        assert_eq!(view.files.len(), 1);
        assert_eq!(view.matching, 1);
        assert_eq!(view.total, 2);
        assert_eq!(view.source.unwrap().path, "src/a.rs");
        let view = snapshot.view(&ViewQuery {
            filter: "absent".into(),
            ..Default::default()
        });
        assert!(view.source.is_none());
        assert!(view.selection.file.is_none());
        assert!(view.files.is_empty());
        let workspace = WorkspaceId::derive(&[b"empty"]);
        assert!(
            Snapshot::new(workspace, vec![])
                .unwrap()
                .view(&ViewQuery::default())
                .source
                .is_none()
        );
    }

    #[test]
    fn tiles_never_overlap_directories_or_each_other_and_stay_in_bounds() {
        let workspace = WorkspaceId::derive(&[b"layout"]);
        let mut files = Vec::new();
        for directory in ["first", "second"] {
            for i in 0..17 {
                files.push(Arc::new(
                    Document::capture(
                        workspace.clone(),
                        format!("{directory}/{i:02}.rs"),
                        b"abc",
                        vec![],
                    )
                    .unwrap(),
                ));
            }
        }
        let view = Snapshot::new(workspace, files)
            .unwrap()
            .view(&ViewQuery::default());
        assert_eq!(view.groups.len(), 2);
        assert!(
            view.height < (view.files.len() + view.groups.len()) as f64 * 80.0,
            "layout wastes excessive vertical space"
        );
        assert_eq!(view.files[0].rect.x, 0.0);
        assert_eq!(view.files[8].rect.x, 0.0);
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
            assert!(r.y >= group.y + 20.0);
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
                Document::capture(workspace.clone(), path.into(), bytes, vec![analysis]).unwrap(),
            ));
        }
        let snapshot = Snapshot::new(workspace.clone(), files).unwrap();
        let view = snapshot.view(&ViewQuery::default());
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
        let selected = view.files[2].id.as_str().to_owned();
        let limited = snapshot.view(&ViewQuery {
            file: Some(selected),
            filter: "good".into(),
            limit: Some(1),
            ..Default::default()
        });
        assert_eq!(limited.source.unwrap().path, "partial.rs");
        let files = (0..2003)
            .map(|i| {
                Arc::new(
                    Document::capture(workspace.clone(), format!("{i}.rs"), b"", vec![]).unwrap(),
                )
            })
            .collect();
        let view = Snapshot::new(workspace, files).unwrap().view(&ViewQuery {
            limit: Some(usize::MAX),
            ..Default::default()
        });
        assert_eq!(view.matching, 2003);
        assert_eq!(view.files.len(), 2000);
    }
}
