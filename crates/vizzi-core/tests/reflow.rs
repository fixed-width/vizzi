//! Display hints preserve source and keep language rules outside the renderer.
use vizzi_core::{
    analysis::LanguageAdapter,
    c_family::{CAdapter, CppAdapter},
    coloring::Style,
    model::{Document, WorkspaceId},
    policy::{Classification, Provenance},
    projection::WindowQuery,
    python::PythonAdapter,
    ruby::RubyAdapter,
    rust::RustAdapter,
};
fn document(language: &dyn LanguageAdapter, text: &str) -> Document {
    let analysis = language.create().unwrap().analyze(text.as_bytes()).unwrap();
    let name = analysis.context.language.clone();
    Document::capture(
        WorkspaceId::derive(&[b"reflow"]),
        "source".into(),
        text.as_bytes(),
        vec![analysis],
        Classification::Assigned {
            language: name,
            provenance: Provenance::Extension {
                extension: "fixture".into(),
            },
        },
    )
    .unwrap()
}
fn query() -> WindowQuery {
    WindowQuery {
        reflow: Some(true),
        ..Default::default()
    }
}
#[test]
fn rust_argument_breaks_align_with_the_opening_delimiter_without_reformatting() {
    let text =
        "fn f() {\n    let value = receiver.process(first, second, nested(third, fourth));\n}\n";
    let doc = document(&RustAdapter, text);
    let view = doc.window(&query(), None).unwrap();
    let line = &view.lines[1];
    let hints = line.reflow.as_ref().unwrap();
    let comma = line.text.find("first, ").unwrap() + 7;
    let point = hints.points.iter().find(|p| p.offset == comma).unwrap();
    assert_eq!(point.anchor, Some(line.text.find("(first").unwrap() + 1));
    assert!(point.priority < 8);
    assert_eq!(hints.leading, 4);
    assert_eq!(
        view.lines
            .iter()
            .map(|l| format!("{}{}", l.text, l.ending.unwrap()))
            .collect::<String>(),
        text
    );
    assert_eq!(view.version, doc.version().clone());
}
#[test]
fn parsed_literals_stay_indivisible_in_each_supported_language() {
    for adapter in [
        &RustAdapter as &dyn LanguageAdapter,
        &PythonAdapter,
        &CAdapter,
        &CppAdapter,
        &RubyAdapter,
    ] {
        let text = if adapter.context().language == "python" {
            "def f():\n    return transform(\"x, [ . ] and y\", 1.25, other)\n"
        } else if adapter.context().language == "ruby" {
            "def f\n    transform(\"x, [ . ] and y\", 1.25, other)\nend\n"
        } else {
            "int_or_fn f() { return transform(\"x, [ . ] and y\", 1.25, other); }\n"
        };
        let doc = document(adapter, text);
        let view = doc.window(&query(), None).unwrap();
        let mut protected = 0;
        for line in &view.lines {
            if let Some(hints) = &line.reflow {
                for style in &line.styles {
                    if matches!(style.kind, Style::String | Style::Number) {
                        protected += 1;
                        assert!(
                            hints
                                .points
                                .iter()
                                .all(|p| p.offset <= style.start || p.offset >= style.end),
                            "{}: {:?}",
                            adapter.context().language,
                            line
                        );
                    }
                }
            }
        }
        assert!(protected >= 2, "{}", adapter.context().language);
    }
}
#[test]
fn python_boolean_operators_get_preferred_breaks_without_becoming_rust_rules() {
    let python = document(
        &PythonAdapter,
        "def f():\n    return first and second or third\n",
    );
    let view = python.window(&query(), None).unwrap();
    let line = &view.lines[1];
    let at = line.text.find("and").unwrap();
    let points = &line.reflow.as_ref().unwrap().points;
    assert!(points.iter().any(|p| p.offset == at && p.priority < 8));
    let rust = document(&RustAdapter, "fn f() { let and = second; }\n");
    let view = rust.window(&query(), None).unwrap();
    let line = &view.lines[0];
    let at = line.text.find("and").unwrap();
    assert!(
        line.reflow
            .as_ref()
            .unwrap()
            .points
            .iter()
            .all(|p| p.offset != at || p.priority == 8)
    );
}
#[test]
fn source_windows_opt_in_and_preserve_crlf_and_tabs() {
    let text = "fn f() {\r\n\tcall(first, second);\r\n}\r\n";
    let doc = document(&RustAdapter, text);
    let plain = doc.window(&WindowQuery::default(), None).unwrap();
    assert!(
        plain
            .lines
            .iter()
            .all(|l| l.reflow.is_none() && l.ending.is_none())
    );
    let view = doc.window(&query(), None).unwrap();
    assert_eq!(
        view.lines
            .iter()
            .map(|l| format!("{}{}", l.text, l.ending.unwrap()))
            .collect::<String>(),
        text
    );
    let clipped = doc
        .window(
            &WindowQuery {
                column_line: Some(2),
                column: Some(3),
                ..query()
            },
            None,
        )
        .unwrap();
    assert!(clipped.lines[1].left);
    assert!(clipped.lines[1].reflow.is_none());
}
#[test]
fn hint_allowances_fall_back_atomically_without_hiding_source() {
    let text = format!("fn f() {{ {} }}\n", "call(a, b);".repeat(300));
    let doc = document(&RustAdapter, &text);
    let view = doc
        .window(
            &WindowQuery {
                line_count: Some(1),
                ..query()
            },
            None,
        )
        .unwrap();
    assert!(view.lines[0].reflow.is_none());
    assert_eq!(view.lines[0].text, text.trim_end());
    let text = format!(
        "fn f() {{\n{}\n}}",
        "    call(a, b, c, d, e, f, g, h, i, j, k, l);\n".repeat(90)
    );
    let doc = document(&RustAdapter, &text);
    let view = doc.window(&query(), None).unwrap();
    let count: usize = view
        .lines
        .iter()
        .filter_map(|l| l.reflow.as_ref())
        .map(|h| h.points.len())
        .sum();
    assert!(count <= 1024);
    assert!(view.lines.iter().any(|l| l.reflow.is_none()));
    assert_eq!(view.lines.len(), 93);
}
#[test]
fn outer_style_captures_cannot_expose_nested_literal_punctuation() {
    let doc = document(
        &RustAdapter,
        "#[doc = \"first, second\"]\nfn f() { call(r#\"a, b ( c )\"#, ','); }\n",
    );
    let view = doc.window(&query(), None).unwrap();
    for line in &view.lines {
        for span in &line.styles {
            if !matches!(span.kind, Style::Keyword | Style::Comment) {
                assert!(
                    line.reflow
                        .as_ref()
                        .unwrap()
                        .points
                        .iter()
                        .all(|p| p.offset <= span.start || p.offset >= span.end),
                    "{:?}: {:?}",
                    span,
                    line
                );
            }
        }
    }
}
#[test]
fn comments_wrap_at_words_and_boolean_operators_remain_structural() {
    let doc = document(
        &RustAdapter,
        "fn f() {\n    // alpha beta, gamma \"delta epsilon\"\n    let Ā = outer(inner(a, b), c) && ready || other;\n}\n",
    );
    let view = doc.window(&query(), None).unwrap();
    let comment = &view.lines[1];
    assert!(!comment.reflow.as_ref().unwrap().points.is_empty());
    assert!(
        comment
            .reflow
            .as_ref()
            .unwrap()
            .points
            .iter()
            .all(|p| p.priority == 4)
    );
    let line = &view.lines[2];
    for operator in ["&&", "||"] {
        assert!(
            line.reflow
                .as_ref()
                .unwrap()
                .points
                .iter()
                .any(|p| p.offset == line.text.find(operator).unwrap() && p.priority == 1)
        );
    }
    assert!(
        line.reflow
            .as_ref()
            .unwrap()
            .points
            .iter()
            .all(|p| line.text.is_char_boundary(p.offset))
    );
}
#[test]
fn unknown_language_and_limited_styling_keep_original_lines() {
    use vizzi_core::{coloring::Coloring, model::ContextId};
    let text = "fn f() { call(a, b); }\n";
    let mut analysis = RustAdapter
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    analysis.context.language = "custom".into();
    analysis.context.id = ContextId::derive(&[b"custom reflow fixture"]);
    let doc = Document::capture(
        WorkspaceId::derive(&[b"custom"]),
        "source".into(),
        text.as_bytes(),
        vec![analysis],
        Classification::Assigned {
            language: "custom".into(),
            provenance: Provenance::Extension {
                extension: "fixture".into(),
            },
        },
    )
    .unwrap();
    assert!(
        doc.window(&query(), None)
            .unwrap()
            .lines
            .iter()
            .all(|l| l.reflow.is_none())
    );
    let mut analysis = RustAdapter
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    analysis.coloring = Some(Coloring::from_spans([(0..text.len(), Style::String)], 0));
    let doc = Document::capture(
        WorkspaceId::derive(&[b"limited"]),
        "source".into(),
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
    let view = doc.window(&query(), None).unwrap();
    assert_eq!(view.coloring, "limited");
    assert!(view.lines.iter().all(|l| l.reflow.is_none()));
    assert!(
        document(&RustAdapter, text)
            .window(
                &WindowQuery {
                    reflow: Some(false),
                    ..Default::default()
                },
                None
            )
            .unwrap()
            .lines
            .iter()
            .all(|l| l.reflow.is_none() && l.ending.is_none())
    );
}
#[test]
fn empty_delimiter_pairs_do_not_create_spurious_continuation_rows() {
    let doc = document(&RustAdapter, "fn f() { call(); let x=[]; }\n");
    let view = doc.window(&query(), None).unwrap();
    let line = &view.lines[0];
    for prefix in ["fn f(", "fn f() { call(", "fn f() { call(); let x=["] {
        assert!(
            line.reflow
                .as_ref()
                .unwrap()
                .points
                .iter()
                .all(|p| p.offset != prefix.len())
        );
    }
}
#[test]
fn compound_operators_remain_atomic_across_language_profiles() {
    for (adapter, operators) in [
        (
            &RustAdapter as &dyn LanguageAdapter,
            vec![
                "&&", "||", "==", "!=", "<=", ">=", "=>", "->", "+=", "-=", "*=", "/=", "..=",
                "..", "=", "+", "-", "*", "/", "%", "<", ">", "&", "|", "^",
            ],
        ),
        (
            &PythonAdapter,
            vec!["**=", "//=", "**", "//", ":=", "<<", ">>"],
        ),
        (
            &CppAdapter,
            vec![
                "<<=", ">>=", "->*", "<=>", "++", "--", ".*", "...", "%=", "&=", "|=", "^=",
            ],
        ),
    ] {
        for op in operators {
            let text = if adapter.context().language == "python" {
                format!("def f():\n    return (first {op} second)\n")
            } else {
                format!("fn_or_type f() {{ let value = first {op} second; }}\n")
            };
            let doc = document(adapter, &text);
            let view = doc.window(&query(), None).unwrap();
            let line = view.lines.iter().find(|l| l.text.contains(op)).unwrap();
            let start = line.text.find(&format!("first {op}")).unwrap() + 6;
            let points = &line.reflow.as_ref().unwrap().points;
            assert!(
                points.iter().any(|p| p.offset == start && p.priority < 8),
                "{} {op}: {:?}",
                adapter.context().language,
                points
            );
            assert!(
                points
                    .iter()
                    .all(|p| p.offset <= start || p.offset >= start + op.len()),
                "{op}: {:?}",
                points
            );
        }
    }
}
#[test]
fn break_bounds_delimiter_nesting_chains_and_qualified_paths_keep_their_evidence() {
    let text = "fn f() {\n    *pointer = outer(inner(a, b), c);\n    let value = [ item(alpha, beta), last ];\n    let thing = Thing { field: nested(x, y), other: last };\n    let path = module::and; call::<Thing>();\n    outer(object.field, last);\n    outer([alpha, beta], final_arg);\n}\n";
    let doc = document(&RustAdapter, text);
    let view = doc.window(&query(), None).unwrap();
    for line in &view.lines {
        let hints = line.reflow.as_ref().unwrap();
        assert!(
            hints
                .points
                .iter()
                .all(|p| p.offset > hints.leading && p.offset < line.text.len())
        );
    }
    let line = &view.lines[1];
    let points = &line.reflow.as_ref().unwrap().points;
    let inner = line.text.find("inner(").unwrap() + 6;
    assert_eq!(
        points.iter().find(|p| p.offset == inner).unwrap().anchor,
        Some(inner)
    );
    let after_inner = line.text.find("c);").unwrap();
    assert_eq!(
        points
            .iter()
            .find(|p| p.offset == after_inner)
            .unwrap()
            .anchor,
        Some(line.text.find("outer(").unwrap() + 6)
    );
    let line = &view.lines[2];
    let points = &line.reflow.as_ref().unwrap().points;
    let bracket = line.text.find('[').unwrap() + 1;
    assert_eq!(
        points
            .iter()
            .find(|p| p.offset == line.text.find("item").unwrap())
            .unwrap()
            .anchor,
        Some(bracket)
    );
    assert_eq!(
        points
            .iter()
            .find(|p| p.offset == line.text.find("last").unwrap())
            .unwrap()
            .anchor,
        Some(bracket)
    );
    let line = &view.lines[3];
    let brace = line.text.find('{').unwrap() + 1;
    assert_eq!(
        line.reflow
            .as_ref()
            .unwrap()
            .points
            .iter()
            .find(|p| p.offset == line.text.find("other").unwrap())
            .unwrap()
            .anchor,
        Some(brace)
    );
    let line = &view.lines[4];
    assert!(
        line.reflow
            .as_ref()
            .unwrap()
            .points
            .iter()
            .any(|p| p.offset == line.text.find('<').unwrap() && p.priority < 8)
    );
    let line = &view.lines[5];
    let point = line
        .reflow
        .as_ref()
        .unwrap()
        .points
        .iter()
        .find(|p| p.offset == line.text.find('.').unwrap())
        .unwrap();
    assert_eq!(point.anchor, None);
    assert_eq!(point.priority, 2);
    let line = &view.lines[6];
    assert_eq!(
        line.reflow
            .as_ref()
            .unwrap()
            .points
            .iter()
            .find(|p| p.offset == line.text.find("final_arg").unwrap())
            .unwrap()
            .anchor,
        Some(line.text.find("outer(").unwrap() + 6)
    );

    let partial = document(&RustAdapter, "fn partial() { let value = path::=other; }\n");
    let view = partial.window(&query(), None).unwrap();
    let line = &view.lines[0];
    let colon = line.text.find("::").unwrap();
    assert!(
        line.reflow
            .as_ref()
            .unwrap()
            .points
            .iter()
            .all(|p| p.offset != colon + 1)
    );
    assert!(
        line.reflow
            .as_ref()
            .unwrap()
            .points
            .iter()
            .any(|p| p.offset == colon + 2)
    );
}
