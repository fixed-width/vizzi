//! Java source selection and analysis through the built-in adapters.
use vizzi_core::{
    analysis::{Analyzer, Languages},
    capture::CapturePlan,
    coloring::Style,
    model::{Coverage, Document, WorkspaceId},
    policy::{Classification, PolicyConfig, SourcePolicy},
    structure::Structure,
};

fn parser() -> Box<dyn Analyzer> {
    Languages::builtin()
        .unwrap()
        .select("java")
        .expect("Java adapter is registered")
        .create()
        .unwrap()
}

#[test]
fn selects_java_by_default_and_honors_exclusions() {
    let languages = Languages::builtin().unwrap();
    let policy = SourcePolicy::new(
        PolicyConfig {
            exclusions: vec!["generated".into()],
            ..Default::default()
        },
        languages.names().map(str::to_owned),
    )
    .unwrap();
    assert!(matches!(
        policy.classify("src/main/java/demo/Counter.java").unwrap(),
        Some(Classification::Assigned { language, .. }) if language == "java"
    ));
    assert_eq!(policy.classify("generated/Counter.java").unwrap(), None);
    assert_eq!(policy.classify("Counter.class").unwrap(), None);
    assert!(languages.call_languages().iter().any(|name| name == "java"));
}

#[test]
fn outlines_types_methods_and_constructors_with_exact_names() {
    let text = r#"package demo;
public class Counter {
    Counter() {}
    int next(int value) { return value + 1; }
    int next(String value) { return value.length(); }
    interface Source { int read(); }
    enum Mode { FAST }
    record Pair(int left, int right) { Pair { check(left); } }
    @interface Checked { String value() default ""; }
}
"#;
    let analysis = parser().analyze(text.as_bytes()).unwrap();
    assert_eq!(analysis.coverage, Coverage::Syntax);
    let outline = analysis
        .facts
        .iter()
        .map(|fact| {
            assert_eq!(&text[fact.name_span.clone().unwrap()], fact.name);
            (
                fact.kind.as_str(),
                fact.name.as_str(),
                fact.parent
                    .map(|parent| analysis.facts[parent].name.as_str()),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        outline,
        [
            ("class", "Counter", None),
            ("function", "Counter", Some("Counter")),
            ("function", "next", Some("Counter")),
            ("function", "next", Some("Counter")),
            ("interface", "Source", Some("Counter")),
            ("signature", "read", Some("Source")),
            ("enum", "Mode", Some("Counter")),
            ("record", "Pair", Some("Counter")),
            ("function", "Pair", Some("Pair")),
            ("annotation", "Checked", Some("Counter")),
            ("signature", "value", Some("Checked")),
        ]
    );
    assert!(!analysis.facts[2].unique && !analysis.facts[3].unique);
    assert_ne!(analysis.facts[2].key, analysis.facts[3].key);
    let coloring = analysis.coloring.unwrap();
    assert!(
        coloring
            .spans(0..text.len(), 256)
            .unwrap()
            .iter()
            .any(|span| span.kind == Style::Keyword)
    );
}

#[test]
fn written_calls_preserve_callee_spelling_and_identifier_in_source() {
    let text = r#"class Runner {
    void run() {
        helper();
        /* 😀 */ target.<String>send();
        new demo.Box<String>();
        Runnable reference = this::helper;
    }
}
"#;
    let analysis = parser().analyze(text.as_bytes()).unwrap();
    let facts = analysis.calls.unwrap();
    assert_eq!(facts.omitted, 0);
    let calls = facts
        .sites
        .iter()
        .map(|site| {
            assert_eq!(&text[site.span.clone()], site.spelling);
            assert_eq!(site.target, None);
            let owner = &analysis.facts[site.owner.unwrap()];
            (
                site.spelling.as_str(),
                &text[site.name_span.clone().unwrap()],
                owner.name.as_str(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        calls,
        [
            ("helper", "helper", "run"),
            ("target.<String>send", "send", "run"),
            ("demo.Box<String>", "Box", "run"),
        ]
    );
}

#[test]
fn implicit_record_constructor_has_a_written_name_candidate() {
    let workspace = WorkspaceId::derive(&[b"java-record"]);
    let languages = Languages::builtin().unwrap();
    let plan = CapturePlan::new(PolicyConfig::default(), &languages).unwrap();
    let path = "Caller.java";
    let text = "record Pair(int x, int y) {}\n\
        class Other { interface Pair {} }\n\
        class Caller { Pair make() { return new Pair(1, 2); } }\n";
    let doc = Document::capture(
        workspace.clone(),
        path.into(),
        text.as_bytes(),
        vec![parser().analyze(text.as_bytes()).unwrap()],
        plan.policy().classify(path).unwrap().unwrap(),
    )
    .unwrap();
    let snapshot = plan
        .snapshot(workspace, vec![std::sync::Arc::new(doc)])
        .unwrap();
    let index = Structure::new(&snapshot);
    let caller = &index.functions(path, "make", 0).unwrap().items[0].symbol;
    let outgoing = index.outgoing(&caller.id, path, 0).unwrap();
    let call = &outgoing.items[0];
    assert_eq!(call.name.as_deref(), Some("Pair"));
    assert_eq!(call.binding, None);
    let [record] = call.candidates.as_slice() else {
        panic!("expected one candidate for the implicit constructor");
    };
    assert_eq!(
        (
            record.kind.as_str(),
            record.name.as_str(),
            record.source.line
        ),
        ("record", "Pair", 1)
    );
}
