//! Bounded projections over one immutable document, including pathological source.

use std::sync::Arc;
use vizzi_core::{
    model::{
        Analysis, CapturePolicyId, ContentId, Context, ContextId, Coverage, Document, Fact,
        Snapshot, WorkspaceId,
    },
    policy::{Classification, Provenance},
    projection::WindowQuery,
    query::{Projection, ViewQuery},
};

fn document(text: &str, names: &[String]) -> Arc<Document> {
    Arc::new(
        Document::capture(
            WorkspaceId::derive(&[b"projection"]),
            "a.rs".into(),
            text.as_bytes(),
            vec![Analysis {
                calls: None,
                coloring: None,
                content: ContentId::derive(&[text.as_bytes()]),
                context: Context {
                    language: "fixture".into(),
                    description: "raw fixture".into(),
                    id: ContextId::derive(&[b"fixture"]),
                },
                coverage: Coverage::Syntax,
                diagnostics: vec![],
                facts: names
                    .iter()
                    .map(|name| Fact {
                        name_span: None,
                        parent: None,
                        key: format!("key {name}"),
                        kind: "function".into(),
                        name: name.clone(),
                        span: 0..1,
                        unique: true,
                    })
                    .collect(),
            }],
            Classification::Assigned {
                language: "fixture".into(),
                provenance: Provenance::Extension {
                    extension: "rs".into(),
                },
            },
        )
        .unwrap(),
    )
}
#[test]
fn summary_omits_bulk_data_and_windows_bound_escaped_text_and_generated_names() {
    let text = format!("{}\nα\r\n", "\u{1}".repeat(1_000_000));
    let names: Vec<_> = (0..250)
        .map(|i| format!("{i}{}", "\u{1}".repeat(10000)))
        .collect();
    let file = document(&text, &names);
    let mut analyses = file.analyses().to_vec();
    analyses[0].coloring = Some(vizzi_core::coloring::Coloring::from_spans(
        (0..1024).map(|i| (i * 2..i * 2 + 1, vizzi_core::coloring::Style::String)),
        1024 * 1024,
    ));
    let file = Arc::new(
        Document::capture(
            WorkspaceId::derive(&[b"projection"]),
            "a.rs".into(),
            text.as_bytes(),
            analyses,
            file.classification().clone(),
        )
        .unwrap(),
    );
    let snapshot = Snapshot::new(
        WorkspaceId::derive(&[b"projection"]),
        vec![file.clone()],
        CapturePolicyId::derive(&[b"fixture"]),
    )
    .unwrap();
    let view = snapshot
        .view(&ViewQuery {
            projection: Some(Projection::Summary),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(view.schema, 3);
    assert!(view.source.as_ref().unwrap().text.is_none());
    assert!(
        view.source.as_ref().unwrap().outlines[0]
            .declarations
            .is_empty()
    );
    assert_eq!(
        view.source
            .as_ref()
            .unwrap()
            .summary
            .as_ref()
            .unwrap()
            .declarations,
        250
    );
    let window = file
        .window(
            &WindowQuery {
                line_count: Some(usize::MAX),
                outline_count: Some(usize::MAX),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert!(window.lines.len() <= 96);
    assert!(
        window
            .lines
            .iter()
            .map(|line| line.text.len())
            .sum::<usize>()
            <= 32768
    );
    assert_eq!(window.outline.len(), 200);
    assert!(
        window
            .outline
            .iter()
            .all(|e| e.name.len() <= 128 && e.name_truncated)
    );
    assert_eq!(
        window.lines.iter().map(|l| l.styles.len()).sum::<usize>(),
        1024
    );
    assert!(serde_json::to_vec(&window).unwrap().len() < 512 * 1024);
    assert!(window.lines[0].right);
    assert_eq!(window.lines[0].line_bytes, 1_000_000);
}
#[test]
fn line_fragments_preserve_utf8_crlf_empty_rows_and_large_offset_clamping() {
    let file = document("α\r\nβ\n", &[]);
    let window = file
        .window(
            &WindowQuery {
                line_start: Some(1),
                column_line: Some(1),
                column: Some(1),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert_eq!(
        window
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        ["α", "β", ""]
    );
    assert_eq!(window.lines[0].column, 0);
    assert_eq!(window.line_total, 3);
    let last = file
        .window(
            &WindowQuery {
                line_start: Some(usize::MAX),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert_eq!(last.line_start, 3);
    assert_eq!(last.lines[0].text, "");
}
#[test]
fn identity_lookup_and_old_focus_recovery_do_not_depend_on_window_membership() {
    let names: Vec<_> = (0..1000).map(|i| format!("item {i}")).collect();
    let old = document("x", &names);
    let end = old
        .window(
            &WindowQuery {
                outline_start: Some(990),
                cursor_index: Some(999),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert_eq!(end.cursor.as_ref().unwrap().index, 999);
    let id = end.cursor.unwrap().id;
    let next = document("x", &names[..999]);
    let result = next
        .window(
            &WindowQuery {
                cursor: Some(id.as_str().into()),
                cursor_index: Some(999),
                ..Default::default()
            },
            Some(&old),
        )
        .unwrap();
    assert_eq!(result.cursor.unwrap().index, 998);
}

#[test]
fn anchors_and_selected_metadata_follow_identity_after_insertions_and_removals() {
    let names: Vec<_> = (0..20).map(|i| format!("item {i}")).collect();
    let old = document("x", &names);
    let original = old
        .window(
            &WindowQuery {
                cursor_index: Some(10),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    let focus = original.cursor.unwrap();
    let anchor = old.entry(8).unwrap();
    let mut inserted = vec!["inserted".into()];
    inserted.extend(names.clone());
    let next = document("x", &inserted);
    let result = next
        .window(
            &WindowQuery {
                cursor: Some(focus.id.as_str().into()),
                anchor: Some(anchor.id.as_str().into()),
                anchor_offset: Some(2),
                ..Default::default()
            },
            Some(&old),
        )
        .unwrap();
    assert_eq!(result.cursor.unwrap().index, 11);
    assert_eq!(result.outline_start, 7);
    let snapshot = Snapshot::new(
        WorkspaceId::derive(&[b"projection"]),
        vec![next.clone()],
        CapturePolicyId::derive(&[b"fixture"]),
    )
    .unwrap();
    let view = snapshot
        .view(&ViewQuery {
            projection: Some(Projection::Summary),
            entity: Some(focus.id.as_str().into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(view.selection.entity, Some(focus.id.clone()));
    assert_eq!(
        view.source.unwrap().summary.unwrap().active.unwrap().index,
        11
    );
    let removed = document(
        "x",
        &names
            .iter()
            .filter(|name| **name != "item 10" && **name != "item 11")
            .cloned()
            .collect::<Vec<_>>(),
    );
    let result = removed
        .window(
            &WindowQuery {
                cursor: Some(focus.id.as_str().into()),
                ..Default::default()
            },
            Some(&old),
        )
        .unwrap();
    assert_eq!(result.cursor.unwrap().name, "item 12");
}

#[test]
fn empty_and_unsupported_documents_have_finite_windows_without_invented_rows() {
    let empty = document("", &[])
        .window(&WindowQuery::default(), None)
        .unwrap();
    assert_eq!(empty.lines.len(), 1);
    assert_eq!(empty.lines[0].text, "");
    assert!(empty.cursor.is_none());
    let binary = Document::capture(
        WorkspaceId::derive(&[b"binary"]),
        "bad.rs".into(),
        &[255],
        vec![],
        Classification::SourceOnly {
            language: None,
            provenance: Provenance::Extension {
                extension: "rs".into(),
            },
            reason: vizzi_core::policy::SourceOnlyReason::LanguageUnassigned,
        },
    )
    .unwrap();
    let window = binary
        .window(
            &WindowQuery {
                cursor: Some("missing".into()),
                outline_start: Some(usize::MAX),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert_eq!(window.line_total, 0);
    assert!(window.lines.is_empty());
    assert!(window.outline.is_empty());
    assert!(window.cursor.is_none());
    let unicode = document(&format!("{}\nlast\n", "🙂".repeat(10000)), &[]);
    let window = unicode
        .window(
            &WindowQuery {
                line_count: Some(96),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert!(window.lines[0].right);
    assert_eq!(window.lines[0].text.len() % 4, 0);
}

#[test]
fn fragments_fill_their_allowance_and_flags_distinguish_start_end_and_other_lines() {
    let file = document(&"x".repeat(100000), &["visible".into()]);
    let first = file
        .window(
            &WindowQuery {
                line_count: Some(1),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert_eq!(first.lines[0].text.len(), 32768);
    assert!(!first.lines[0].left);
    assert!(first.lines[0].right);
    assert_eq!(first.outline[0].name, "visible");
    assert_eq!(first.outline[0].kind, "function");
    let middle = file
        .window(
            &WindowQuery {
                line_count: Some(1),
                column_line: Some(1),
                column: Some(32768),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert_eq!(middle.lines[0].column, 32768);
    assert_eq!(middle.lines[0].text.len(), 32768);
    assert!(middle.lines[0].left);
    assert!(middle.lines[0].right);
    let lines = document("α\nβ\n", &[])
        .window(
            &WindowQuery {
                column_line: Some(2),
                column: Some(2),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert_eq!(lines.lines[0].text, "α");
    assert_eq!(lines.lines[1].text, "");
    assert!(lines.lines[1].left);
    assert!(!lines.lines[1].right);
    let blank = document("\n\r\n", &[])
        .window(&WindowQuery::default(), None)
        .unwrap();
    assert_eq!(
        blank
            .lines
            .iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>(),
        ["", "", ""]
    );
}

#[test]
fn flattened_ranges_preserve_analysis_membership_and_exact_label_boundaries() {
    let base = document("x", &["n".repeat(128), "short".into()]);
    let mut first = base.analyses()[0].clone();
    first.facts[0].kind = "k".repeat(32);
    let mut second = first.clone();
    second.context.id = ContextId::derive(&[b"second context"]);
    second.facts[0].kind = "k".repeat(33);
    let file = Document::capture(
        WorkspaceId::derive(&[b"projection"]),
        "a.rs".into(),
        b"x",
        vec![first, second],
        base.classification().clone(),
    )
    .unwrap();
    let rows = file.window(&WindowQuery::default(), None).unwrap();
    assert_eq!(rows.outline.len(), 4);
    assert!(!rows.outline[0].name_truncated);
    assert!(!rows.outline[0].kind_truncated);
    assert_eq!(rows.outline[0].name.len(), 128);
    assert_eq!(rows.outline[0].kind.len(), 32);
    assert_eq!(rows.outline[2].analysis, 1);
    assert_eq!(rows.outline[2].index, 2);
    assert!(rows.outline[2].kind_truncated);
    assert_eq!(rows.outline[2].kind.len(), 32);
    assert_eq!(rows.outline[3].name, "short");
}

#[test]
fn multibyte_labels_round_down_at_both_exact_display_boundaries() {
    let base = document("x", &[format!("{}🙂", "a".repeat(127))]);
    let mut analysis = base.analyses()[0].clone();
    analysis.facts[0].kind = format!("{}🙂", "k".repeat(31));
    let file = Document::capture_source(
        WorkspaceId::derive(&[b"projection"]),
        "a.rs".into(),
        base.captured().clone(),
        vec![analysis],
        base.classification().clone(),
    )
    .unwrap();
    let row = file.entry(0).unwrap();
    assert_eq!(row.name, "a".repeat(127));
    assert_eq!(row.kind, "k".repeat(31));
    assert!(row.name_truncated);
    assert!(row.kind_truncated);
}
