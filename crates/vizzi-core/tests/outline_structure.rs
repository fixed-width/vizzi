//! Source-backed declaration hierarchy without inferring ownership from names.
use vizzi_core::{
    analysis::Languages,
    model::{Document, WorkspaceId},
    policy::{Classification, Provenance},
    projection::WindowQuery,
};
fn document(language: &str, text: &str) -> Document {
    let analysis = Languages::builtin()
        .unwrap()
        .select(language)
        .unwrap()
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    Document::capture(
        WorkspaceId::derive(&[b"outline-structure"]),
        "source.txt".into(),
        text.as_bytes(),
        vec![analysis],
        Classification::Assigned {
            language: language.into(),
            provenance: Provenance::Extension {
                extension: "fixture".into(),
            },
        },
    )
    .unwrap()
}
#[test]
fn rust_scopes_distinguish_top_level_siblings_impls_and_nested_functions() {
    let doc = document(
        "rust",
        "mod outer { struct A; impl A { fn run(){fn nested(){}} } fn top(){} } fn outside(){}",
    );
    let facts = &doc.analyses()[0].facts;
    assert_eq!(
        facts.iter().map(|f| f.parent).collect::<Vec<_>>(),
        [None, Some(0), Some(0), Some(2), Some(3), Some(0), None]
    );
    let w = doc.window(&WindowQuery::default(), None).unwrap();
    assert_eq!(
        w.outline.iter().map(|e| e.depth).collect::<Vec<_>>(),
        [0, 1, 1, 2, 3, 1, 0]
    );
    assert_eq!(w.outline[3].scope.as_deref(), Some("A"));
    assert_eq!(w.outline[4].scope.as_deref(), Some("run"));
    assert!(w.outline[6].scope.is_none());
}
#[test]
fn all_builtin_languages_supply_only_their_actual_lexical_scope_links() {
    for (language, text, parents) in [
        (
            "python",
            "class Outer:\n    def run(self):\n        def nested(): pass\ndef outside(): pass\n",
            vec![None, Some(0), Some(1), None],
        ),
        (
            "cpp",
            "namespace n { class C { public: int run() { return 1; } }; using Alias = C; }",
            vec![None, Some(0), Some(1), Some(0)],
        ),
        (
            "c",
            "struct Outer { struct Inner { int value; }; }; int run(void) { return 1; }",
            vec![None, Some(0), None],
        ),
    ] {
        let doc = document(language, text);
        assert_eq!(
            doc.analyses()[0]
                .facts
                .iter()
                .map(|f| f.parent)
                .collect::<Vec<_>>(),
            parents,
            "{language}"
        );
    }
}
#[test]
fn hierarchy_survives_paging_deep_nesting_and_context_boundaries_with_bounded_labels() {
    let mut text = String::new();
    for i in 0..35 {
        text.push_str(&format!("mod level_{i} {{"));
    }
    text.push_str("fn leaf() {} ");
    text.push_str(&"}".repeat(35));
    let doc = document("rust", &text);
    let w = doc
        .window(
            &WindowQuery {
                outline_start: Some(34),
                outline_count: Some(2),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert_eq!(w.outline[0].depth, 34);
    assert_eq!(w.outline[1].depth, 35);
    assert_eq!(w.outline[1].scope.as_deref(), Some("level_34"));
    let long = document(
        "rust",
        "mod ExtremelyLongEnclosingModuleNameThatMustBeShortened {fn child(){}}",
    );
    assert!(long.entry(1).unwrap().scope.unwrap().len() <= 24);
    let text = "mod a {fn inner(){}}";
    let mut a = Languages::builtin()
        .unwrap()
        .select("rust")
        .unwrap()
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    let mut b = a.clone();
    b.context.id = vizzi_core::model::ContextId::derive(&[b"second-context"]);
    a.context.description = "first".into();
    let doc = Document::capture(
        WorkspaceId::derive(&[b"contexts"]),
        "x.rs".into(),
        text.as_bytes(),
        vec![a, b],
        Classification::Assigned {
            language: "rust".into(),
            provenance: Provenance::Extension {
                extension: "rs".into(),
            },
        },
    )
    .unwrap();
    assert_eq!(doc.entry(2).unwrap().depth, 0);
    assert_eq!(doc.entry(3).unwrap().depth, 1);
}
#[test]
fn invalid_parent_links_are_refused_before_publication() {
    let text = "mod a { fn child(){} } fn outside(){}";
    let doc = document("rust", text);
    for (child, parent) in [(1, 1), (0, 1), (2, 1)] {
        let mut analyses = doc.analyses().to_vec();
        analyses[0].facts[child].parent = Some(parent);
        assert!(
            Document::capture(
                WorkspaceId::derive(&[b"invalid"]),
                "x.rs".into(),
                text.as_bytes(),
                analyses,
                doc.classification().clone()
            )
            .is_err()
        );
    }
}

#[test]
fn escaped_hierarchy_labels_and_styles_still_fit_the_document_window_budget() {
    use vizzi_core::{
        coloring::{Coloring, Style},
        model::{Analysis, ContentId, Context, ContextId, Coverage, Fact},
    };
    let text = format!("{}\n", "\u{1}".repeat(341)).repeat(96);
    let facts = (0usize..201)
        .map(|i| Fact {
            parent: i.checked_sub(1),
            key: i.to_string(),
            kind: "\u{1}".repeat(32),
            name: "\u{1}".repeat(128),
            span: i..text.len() - i,
            unique: true,
        })
        .collect();
    let analysis = Analysis {
        calls: None,
        content: ContentId::derive(&[text.as_bytes()]),
        context: Context {
            language: "fixture".into(),
            description: "fixture".into(),
            id: ContextId::derive(&[b"fixture"]),
        },
        coverage: Coverage::Syntax,
        facts,
        diagnostics: vec![],
        coloring: Some(Coloring::from_spans(
            (0..1024).map(|i| (i * 2..i * 2 + 1, Style::Function)),
            1024 * 1024,
        )),
    };
    let doc = Document::capture(
        WorkspaceId::derive(&[b"escaped"]),
        "x.rs".into(),
        text.as_bytes(),
        vec![analysis],
        Classification::Assigned {
            language: "fixture".into(),
            provenance: Provenance::Extension {
                extension: "rs".into(),
            },
        },
    )
    .unwrap();
    let window = doc
        .window(
            &WindowQuery {
                outline_start: Some(1),
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert_eq!(window.outline.len(), 200);
    assert!(
        window
            .outline
            .iter()
            .all(|e| e.scope.as_ref().unwrap().len() <= 24)
    );
    assert!(serde_json::to_vec(&window).unwrap().len() < 512 * 1024);
}

#[test]
fn lexical_parent_ranges_allow_one_shared_boundary_but_reject_coextensive_declarations() {
    let text = "mod a { fn child(){} }";
    let doc = document("rust", text);
    let len = text.len();
    for (parent, child, valid) in [
        (0..len, 0..len - 1, true),
        (0..len, 1..len, true),
        (0..len, 0..len, false),
        (1..len, 0..len - 1, false),
    ] {
        let mut analyses = doc.analyses().to_vec();
        analyses[0].facts[0].span = parent;
        analyses[0].facts[1].span = child;
        analyses[0].facts[1].parent = Some(0);
        let result = Document::capture(
            WorkspaceId::derive(&[b"boundary"]),
            "x.rs".into(),
            text.as_bytes(),
            analyses,
            doc.classification().clone(),
        );
        assert_eq!(result.is_ok(), valid);
    }
}
#[test]
fn parent_scope_labels_keep_exact_limits_and_whole_unicode_characters() {
    let name = "界".repeat(8);
    let doc = document("rust", &format!("mod {name} {{fn child(){{}}}}"));
    assert_eq!(doc.entry(1).unwrap().scope.as_deref(), Some(name.as_str()));
    let name = "界".repeat(9);
    let doc = document("rust", &format!("mod {name} {{fn child(){{}}}}"));
    let scope = doc.entry(1).unwrap().scope.unwrap();
    assert_eq!(scope.len(), 24);
    assert!(scope.ends_with('…'));
    assert_eq!(scope.chars().count(), 8);
}
