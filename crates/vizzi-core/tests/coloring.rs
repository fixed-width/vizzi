//! Captured, bounded syntax styling independent of the renderer theme.
use vizzi_core::{
    analysis::Languages,
    coloring::{Coloring, Style},
    model::{Document, DocumentVersion, WorkspaceId},
    policy::{Classification, PolicyConfig, Provenance, SourcePolicy},
    projection::{Window, WindowQuery},
};

fn source_window(path: &str, text: &[u8], query: WindowQuery) -> Window {
    let policy = SourcePolicy::new(PolicyConfig::default(), []).unwrap();
    let classification = policy.classify(path).unwrap().unwrap();
    assert!(classification.assigned_language().is_none());
    let doc = Document::capture(
        WorkspaceId::derive(&[b"source-coloring"]),
        path.into(),
        text,
        vec![],
        classification,
    )
    .unwrap();
    doc.window(&query, None).unwrap()
}

fn styled_text(window: &Window) -> Vec<(&str, Style)> {
    window
        .lines
        .iter()
        .flat_map(|line| {
            line.styles
                .iter()
                .map(|span| (&line.text[span.start..span.end], span.kind))
        })
        .collect()
}

#[test]
fn document_coloring_invalidates_legacy_versions_for_eligible_files() {
    let policy = SourcePolicy::new(PolicyConfig::default(), []).unwrap();
    let workspace = WorkspaceId::derive(&[b"legacy-coloring-versions"]);
    for (path, text, eligible) in [
        ("README.md", "# Title", true),
        ("Cargo.toml", "[package]", true),
        ("package.json", "{}", true),
        ("README", "Title", false),
        ("Cargo.lock", "# Generated file", false),
    ] {
        let classification = policy.classify(path).unwrap().unwrap();
        let doc = Document::capture(
            workspace.clone(),
            path.into(),
            text.as_bytes(),
            vec![],
            classification,
        )
        .unwrap();
        let legacy_metadata = serde_json::to_vec(&(doc.classification(), doc.analyses())).unwrap();
        let legacy = DocumentVersion::derive(&[
            b"document-v2",
            doc.id().as_str().as_bytes(),
            doc.content().as_str().as_bytes(),
            blake3::hash(&legacy_metadata).as_bytes(),
        ]);
        assert_eq!(doc.version() != &legacy, eligible, "{path}");
    }
}

#[test]
fn toml_manifests_and_config_files_color_keys_tables_and_values() {
    let text = "[[package.example]]\r\n\"true\".name = \"hi🙂#\" # note\r\nvalues = [true, -2.5, 0xff, 2026-11-24]\r\n";
    let expected = [
        ("[[package.example]]", Style::Type),
        ("\"true\".name", Style::Attribute),
        ("\"hi🙂#\"", Style::String),
        ("# note", Style::Comment),
        ("values", Style::Attribute),
        ("true", Style::Constant),
        ("-2.5", Style::Number),
        ("0xff", Style::Number),
        ("2026-11-24", Style::Constant),
    ];
    for path in ["Cargo.toml", "pyproject.toml", "settings.TOML"] {
        let window = source_window(path, text.as_bytes(), WindowQuery::default());
        assert_eq!(window.coloring, "ready", "{path}");
        assert_eq!(window.outline_total, 0);
        assert_eq!(styled_text(&window), expected, "{path}");
    }
}

#[test]
fn toml_coloring_recovers_after_an_unclosed_table_and_keeps_multiline_strings() {
    let text = "[incomplete\nkey = '''first\n# inside🙂\nlast'''\n";
    let window = source_window(
        ".cargo/config.toml",
        text.as_bytes(),
        WindowQuery::default(),
    );
    assert_eq!(window.coloring, "ready");
    assert_eq!(
        styled_text(&window),
        [
            ("[incomplete", Style::Type),
            ("key", Style::Attribute),
            ("'''first", Style::String),
            ("# inside🙂", Style::String),
            ("last'''", Style::String),
        ]
    );
}

#[test]
fn toml_coloring_admits_the_exact_depth_limit_and_discards_excess_depth() {
    for (open, close) in [("[", "]"), ("{ key = ", " }")] {
        for depth in [80, 81] {
            let nested = format!("{}42{}", open.repeat(depth), close.repeat(depth));
            let text = format!("first = {nested}\nsecond = {nested}");
            let window = source_window("settings.toml", text.as_bytes(), WindowQuery::default());
            assert_eq!(window.lines[0].text, format!("first = {nested}"));
            assert_eq!(window.lines[1].text, format!("second = {nested}"));
            if depth == 80 {
                assert_eq!(window.coloring, "ready");
                assert_eq!(
                    styled_text(&window)
                        .iter()
                        .filter(|&&span| span == ("42", Style::Number))
                        .count(),
                    2
                );
            } else {
                assert_eq!(window.coloring, "limited");
                assert!(styled_text(&window).is_empty());
            }
        }
    }
}

#[test]
fn toml_coloring_recovers_from_a_stray_closing_delimiter_at_the_depth_limit() {
    let text = format!("value = {}}}, 42{}", "[".repeat(80), "]".repeat(80));
    let window = source_window("settings.toml", text.as_bytes(), WindowQuery::default());
    assert_eq!(window.coloring, "ready");
    assert!(styled_text(&window).contains(&("42", Style::Number)));
}

#[test]
fn markdown_colors_literal_markup_and_carries_fenced_code_across_windows() {
    let text = "# Guide\n\n**bold** and *emphasis* with `🙂code` and [link](somewhere).\n\n~~~toml\n# inside the fence\n~~~\n\n<!-- comment -->\n";
    let window = source_window("README.md", text.as_bytes(), WindowQuery::default());
    assert_eq!(window.coloring, "ready");
    assert_eq!(window.outline_total, 0);
    assert_eq!(
        styled_text(&window),
        [
            ("# Guide", Style::Keyword),
            ("**bold**", Style::Attribute),
            ("*emphasis*", Style::Attribute),
            ("`🙂code`", Style::String),
            ("[link](somewhere)", Style::Function),
            ("~~~toml", Style::String),
            ("# inside the fence", Style::String),
            ("~~~", Style::String),
            ("<!-- comment -->", Style::Comment),
        ]
    );
    let middle = source_window(
        "docs/guide.markdown",
        text.as_bytes(),
        WindowQuery {
            line_start: Some(6),
            line_count: Some(1),
            ..WindowQuery::default()
        },
    );
    assert_eq!(
        styled_text(&middle),
        [("# inside the fence", Style::String)]
    );
}

#[test]
fn markdown_coloring_understands_task_lists_and_table_boundaries() {
    let text = "- [x] ~~done~~\n\n| A | B |\n| - | - |\n| *from | to* |\n\nUse <kbd>Enter</kbd>.\n";
    let window = source_window("notes.md", text.as_bytes(), WindowQuery::default());
    assert_eq!(
        styled_text(&window),
        [("[x]", Style::Attribute), ("~~done~~", Style::Attribute)]
    );
    assert_eq!(window.lines[6].text, "Use <kbd>Enter</kbd>.");
}

#[test]
fn markdown_comments_end_at_the_closing_delimiter() {
    let text = "<!-- note --> visible text\n\n<!-- long\r\n🙂 --> more text\n\nBefore <!-- inline --> after.\n\n<!-- unclosed\n";
    let window = source_window("notes.md", text.as_bytes(), WindowQuery::default());
    assert_eq!(
        styled_text(&window),
        [
            ("<!-- note -->", Style::Comment),
            ("<!-- long", Style::Comment),
            ("🙂 -->", Style::Comment),
            ("<!-- inline -->", Style::Comment),
            ("<!-- unclosed", Style::Comment),
        ]
    );
}

#[test]
fn json_colors_keys_and_values_in_nested_objects_and_arrays() {
    let text = "\u{feff}{\r\n  \"items\": [\"first\", {\"emoji🙂\" : \"escaped\\\" quote\"}, -1.25e+3, null, true, false],\r\n  \"empty\": {}\r\n}";
    for path in ["fixture.JSON", "package.json", "tsconfig.build.json"] {
        let window = source_window(path, text.as_bytes(), WindowQuery::default());
        assert_eq!(window.coloring, "ready", "{path}");
        assert_eq!(
            styled_text(&window),
            [
                ("\"items\"", Style::Attribute),
                ("\"first\"", Style::String),
                ("\"emoji🙂\"", Style::Attribute),
                ("\"escaped\\\" quote\"", Style::String),
                ("-1.25e+3", Style::Number),
                ("null", Style::Constant),
                ("true", Style::Constant),
                ("false", Style::Constant),
                ("\"empty\"", Style::Attribute),
            ],
            "{path}"
        );
    }
}

#[test]
fn json_colors_comments_and_keeps_text_readable_after_a_lexical_error() {
    let text = "{\n  // settings\n  \"name\" /* key comment */: \"demo\",\n  \"count\": -\n}";
    let window = source_window("tsconfig.json", text.as_bytes(), WindowQuery::default());
    assert_eq!(
        styled_text(&window),
        [
            ("// settings", Style::Comment),
            ("\"name\"", Style::Attribute),
            ("/* key comment */", Style::Comment),
            ("\"demo\"", Style::String),
            ("\"count\"", Style::Attribute),
        ]
    );
    assert_eq!(window.lines[3].text, "  \"count\": -");
}

#[test]
fn json_colors_tokens_after_escaped_lone_surrogates() {
    let text = r#"{"\uDEAD": ["\uD800", "\uDEAD\"🙂"], "after": true}"#;
    let window = source_window("fixture.json", text.as_bytes(), WindowQuery::default());
    assert_eq!(
        styled_text(&window),
        [
            (r#""\uDEAD""#, Style::Attribute),
            (r#""\uD800""#, Style::String),
            (r#""\uDEAD\"🙂""#, Style::String),
            (r#""after""#, Style::Attribute),
            ("true", Style::Constant),
        ]
    );
}

#[test]
fn json_string_recovery_does_not_cross_an_unescaped_newline() {
    let text = r#"{"broken": "\uDEAD
"after": true}"#;
    let window = source_window("fixture.json", text.as_bytes(), WindowQuery::default());
    assert_eq!(styled_text(&window), [(r#""broken""#, Style::Attribute)]);
}

#[test]
fn unsupported_and_oversized_document_coloring_preserves_plain_source_behavior() {
    let binary = source_window("notes.md", &[0xff], WindowQuery::default());
    assert_eq!(binary.coloring, "unavailable");
    assert!(binary.lines.is_empty());
    let mut text = format!("# Heading\n\n{}", "a".repeat(1024 * 1024));
    let large = source_window("notes.md", text.as_bytes(), WindowQuery::default());
    assert_eq!(large.coloring, "limited");
    assert_eq!(large.lines[0].text, "# Heading");
    assert!(styled_text(&large).is_empty());
    text.truncate(1024 * 1024);
    let within_cap = source_window("notes.md", text.as_bytes(), WindowQuery::default());
    assert_eq!(within_cap.coloring, "ready");
    assert_eq!(styled_text(&within_cap), [("# Heading", Style::Keyword)]);
}

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
