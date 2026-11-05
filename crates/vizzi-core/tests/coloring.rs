//! Captured, bounded syntax styling independent of the renderer theme.
use vizzi_core::{
    analysis::Languages,
    coloring::{Coloring, Style},
    model::{Document, WorkspaceId},
    policy::{Classification, Provenance},
    projection::WindowQuery,
};

#[test]
fn immutable_ranges_clip_to_fragments_merge_adjacent_styles_and_survive_cache_roundtrips() {
    let index = Coloring::from_spans(
        [
            (0..3, Style::Keyword),
            (3..5, Style::Keyword),
            (8..15, Style::String),
        ],
        1024,
    );
    let runs = index.spans(2..11, 10).unwrap();
    assert_eq!(
        runs.iter()
            .map(|s| (s.start, s.end, s.kind))
            .collect::<Vec<_>>(),
        [(0, 3, Style::Keyword), (6, 9, Style::String)]
    );
    let encoded = serde_json::to_string(&index).unwrap();
    let decoded: Coloring = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, index);
    assert!(index.spans(0..15, 1).is_none());
    assert!(index.spans(5..8, 10).unwrap().is_empty());
    assert!(index.spans(2..2, 10).unwrap().is_empty());
    assert!(index.valid_for(15));
    assert!(!index.valid_for(14));
}

#[test]
fn index_admission_discards_partial_style_data_and_rejects_invalid_cache_encoding() {
    let limited = Coloring::from_spans([(0..3, Style::Keyword), (8..15, Style::String)], 8);
    assert!(!limited.available());
    assert!(limited.spans(0..15, 100).is_none());
    assert!(serde_json::from_str::<Coloring>(r#"{"data":"not base64!","limited":false}"#).is_err());
}

#[test]
fn all_builtin_languages_supply_source_bound_lexical_colours() {
    let languages = Languages::builtin().unwrap();
    for (language, text, keyword, comment, string) in [
        (
            "rust",
            "// note\nfn demo() { let n = 42; let s = \"hi🙂\"; }",
            "fn",
            "// note",
            "\"hi🙂\"",
        ),
        (
            "python",
            "# note\ndef demo():\n    n = 42\n    s = \"hi🙂\"\n",
            "def",
            "# note",
            "\"hi🙂\"",
        ),
        (
            "c",
            "/* note */\nint demo() { return 42; } const char *s = \"hi🙂\";",
            "return",
            "/* note */",
            "\"hi🙂\"",
        ),
        (
            "cpp",
            "// note\nclass Demo {}; int demo() { return 42; } const char *s = \"hi🙂\";",
            "class",
            "// note",
            "\"hi🙂\"",
        ),
    ] {
        let analysis = languages
            .select(language)
            .unwrap()
            .create()
            .unwrap()
            .analyze(text.as_bytes())
            .unwrap();
        let spans = analysis
            .coloring
            .as_ref()
            .unwrap()
            .spans(0..text.len(), 1024)
            .unwrap();
        for (expected, kind) in [
            (keyword, Style::Keyword),
            (comment, Style::Comment),
            (string, Style::String),
            ("42", Style::Number),
        ] {
            assert!(
                spans
                    .iter()
                    .any(|s| s.kind == kind && &text[s.start..s.end] == expected),
                "missing {kind:?} {expected:?} in {language}: {spans:?}"
            );
        }
    }
}

#[test]
fn multiline_and_unicode_styles_project_without_reparsing_or_crossing_fragment_boundaries() {
    let text = "/* first\n  🙂 comment\n*/\nfn demo() {}\n";
    let analysis = Languages::builtin()
        .unwrap()
        .select("rust")
        .unwrap()
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    let doc = Document::capture(
        WorkspaceId::derive(&[b"colour"]),
        "a.rs".into(),
        text.as_bytes(),
        vec![analysis],
        Classification::Assigned {
            language: "rust".into(),
            provenance: Provenance::Extension {
                extension: "rs".into(),
            },
        },
    )
    .unwrap();
    let window = doc
        .window(
            &WindowQuery {
                line_start: Some(2),
                line_count: Some(1),
                column_line: Some(2),
                column: Some(3),
                ..WindowQuery::default()
            },
            None,
        )
        .unwrap();
    let line = &window.lines[0];
    assert!(line.text.starts_with("🙂"));
    assert_eq!(line.styles[0].kind, Style::Comment);
    assert_eq!(line.styles[0].start, 0);
    assert_eq!(line.styles[0].end, line.text.len());
    let json = serde_json::to_value(&window).unwrap();
    assert_eq!(json["coloring"], "ready");
}

#[test]
fn style_limits_keep_complete_plain_source_and_never_serve_partial_colours() {
    let text = format!("fn demo() {{ let _ = ({}); }}", "1,".repeat(1100));
    let analysis = Languages::builtin()
        .unwrap()
        .select("rust")
        .unwrap()
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    let doc = Document::capture(
        WorkspaceId::derive(&[b"style-limit"]),
        "limit.rs".into(),
        text.as_bytes(),
        vec![analysis],
        Classification::Assigned {
            language: "rust".into(),
            provenance: Provenance::Extension {
                extension: "rs".into(),
            },
        },
    )
    .unwrap();
    let window = doc.window(&WindowQuery::default(), None).unwrap();
    assert_eq!(window.coloring, "limited");
    assert!(window.lines.iter().all(|line| line.styles.is_empty()));
    assert_eq!(window.lines[0].text, text);
}

#[test]
fn style_index_boundaries_empty_input_and_cached_metadata_are_validated() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let empty = Coloring::from_spans([], 0);
    assert!(empty.available());
    assert_eq!(empty.spans(0..20, 0), Some(vec![]));
    let exact = Coloring::from_spans(
        [
            (2..5, Style::Type),
            (5..9, Style::Function),
            (20..21, Style::Comment),
        ],
        48,
    );
    assert!(exact.available());
    assert_eq!(exact.spans(0..30, 3).unwrap().len(), 3);
    assert!(exact.spans(0..30, 2).is_none());
    assert_eq!(exact.spans(6..8, 1).unwrap()[0].kind, Style::Function);
    assert!(exact.spans(30..40, 0).unwrap().is_empty());
    let clipped = Coloring::from_spans(
        [
            (1..8, Style::String),
            (2..4, Style::Number),
            (10..10, Style::Keyword),
            (12..13, Style::Comment),
        ],
        32,
    );
    assert_eq!(clipped.spans(0..14, 10).unwrap().len(), 2);
    for (runs, limited) in [
        (vec![1_u64, 1], false),
        (vec![1, 33, 32], false),
        (vec![9, 16], false),
        (vec![1, 17], false),
        (vec![1, 16], true),
        (vec![0, 17, 32], false),
        (vec![1, 18], false),
        (vec![1, 17, 32], false),
        (vec![34, 17, 48], false),
        (vec![1, 2, 16], false),
    ] {
        let raw = runs
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>();
        let encoded = serde_json::json!({"data":STANDARD.encode(raw),"limited":limited});
        assert!(
            serde_json::from_value::<Coloring>(encoded).is_err(),
            "accepted {runs:?}, {limited}"
        );
    }
    let encoded = serde_json::json!({"data":STANDARD.encode([1,2,3]),"limited":false});
    assert!(serde_json::from_value::<Coloring>(encoded).is_err());
    let too_large = serde_json::json!({"data":"A".repeat(vizzi_core::coloring::INDEX_BYTES.div_ceil(3)*4+4),"limited":false});
    assert!(serde_json::from_value::<Coloring>(too_large).is_err());
}

#[test]
fn document_styling_admits_exactly_one_mebibyte_and_refuses_any_larger_index() {
    let captures = |count| (0..count).map(|i| (i * 2..i * 2 + 1, Style::Keyword));
    let exact = Coloring::from_spans(captures(65536), usize::MAX);
    assert!(exact.available());
    let decoded: Coloring = serde_json::from_value(serde_json::to_value(&exact).unwrap()).unwrap();
    assert_eq!(decoded, exact);
    assert_eq!(
        exact.spans(131070..131071, 1).unwrap()[0].kind,
        Style::Keyword
    );
    assert!(!Coloring::from_spans(captures(65537), usize::MAX).available());
}

#[test]
fn every_style_roundtrips_and_styles_cannot_split_source_characters() {
    for kind in [
        Style::Keyword,
        Style::Type,
        Style::Function,
        Style::String,
        Style::Comment,
        Style::Number,
        Style::Constant,
        Style::Attribute,
    ] {
        let index = Coloring::from_spans([(2..4, kind), (4..5, Style::Comment)], 128);
        let restored: Coloring =
            serde_json::from_value(serde_json::to_value(&index).unwrap()).unwrap();
        assert_eq!(restored.spans(3..5, 2).unwrap()[0].kind, kind);
    }
    let text = "🙂";
    let mut analysis = Languages::builtin()
        .unwrap()
        .select("rust")
        .unwrap()
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    analysis.coloring = Some(Coloring::from_spans([(1..3, Style::String)], 128));
    assert!(
        Document::capture(
            WorkspaceId::derive(&[b"bad-styling"]),
            "bad.rs".into(),
            text.as_bytes(),
            vec![analysis],
            Classification::Assigned {
                language: "rust".into(),
                provenance: Provenance::Extension {
                    extension: "rs".into()
                }
            }
        )
        .is_err()
    );
}

#[test]
fn all_semantic_free_categories_are_supplied_by_builtin_highlight_queries() {
    let text = "#[derive(Debug)]\npub struct Demo { value: usize }\nconst ENABLED: bool = true;\nfn go() { let n: f64 = 1.25; }\n";
    let analysis = Languages::builtin()
        .unwrap()
        .select("rust")
        .unwrap()
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    let spans = analysis
        .coloring
        .unwrap()
        .spans(0..text.len(), 1024)
        .unwrap();
    for kind in [
        Style::Attribute,
        Style::Type,
        Style::Constant,
        Style::Function,
        Style::Number,
    ] {
        assert!(
            spans.iter().any(|s| s.kind == kind),
            "missing {kind:?}: {spans:?}"
        );
    }
}

#[test]
fn style_segment_budget_is_shared_across_rows() {
    let text = format!(
        "fn go() {{\n{} }}",
        format!("let _ = ({});\n", "1,".repeat(20)).repeat(80)
    );
    let analysis = Languages::builtin()
        .unwrap()
        .select("rust")
        .unwrap()
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    let doc = Document::capture(
        WorkspaceId::derive(&[b"multi-row-style-limit"]),
        "many.rs".into(),
        text.as_bytes(),
        vec![analysis],
        Classification::Assigned {
            language: "rust".into(),
            provenance: Provenance::Extension {
                extension: "rs".into(),
            },
        },
    )
    .unwrap();
    let window = doc.window(&WindowQuery::default(), None).unwrap();
    assert_eq!(window.coloring, "limited");
    assert!(window.lines.iter().all(|l| l.styles.is_empty()));
    let small = doc
        .window(
            &WindowQuery {
                line_count: Some(1),
                ..WindowQuery::default()
            },
            None,
        )
        .unwrap();
    assert_eq!(small.coloring, "ready");
    assert!(!small.lines[0].styles.is_empty());
}
