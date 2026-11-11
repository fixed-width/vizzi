//! Raw C/C++ declarations, source spans, and lexical identity contracts.

use vizzi_core::{
    analysis::Languages,
    model::{Analysis, Coverage, Document, WorkspaceId},
    policy::{PolicyConfig, SourcePolicy},
};

fn analyze(language: &str, text: &str) -> Analysis {
    let languages = Languages::builtin().unwrap();
    let adapter = languages.select(language).unwrap();
    let analysis = adapter.create().unwrap().analyze(text.as_bytes()).unwrap();
    assert_eq!(&analysis.context, adapter.context());
    let policy = SourcePolicy::new(
        PolicyConfig {
            c_family_roots: vec![".".into()],
            ..Default::default()
        },
        languages.names().map(String::from),
    )
    .unwrap();
    let path = if language == "c" { "a.c" } else { "a.cpp" };
    Document::capture(
        WorkspaceId::derive(&[b"test"]),
        path.into(),
        text.as_bytes(),
        vec![analysis.clone()],
        policy.classify(path).unwrap().unwrap(),
    )
    .unwrap();
    analysis
}

#[test]
fn c_declarations_and_every_typedef_keep_original_spans_without_inventing_variables() {
    let text = "struct Tag { int member; }; union U { int n; float f; }; enum E { A, B };\ntypedef struct Tag Alias, *Pointer; typedef int Array[3], (*Callback)(int);\nint global; struct Tag *reference; int prototype(int); int (*function_pointer)(int);\nint add(int a, int b) { return a + b; } int (*factory(void))(int) { return 0; }\n";
    let analysis = analyze("c", text);
    assert_eq!(analysis.coverage, Coverage::Syntax);
    assert_eq!(
        analysis
            .facts
            .iter()
            .map(|f| (f.kind.as_str(), f.name.as_str()))
            .collect::<Vec<_>>(),
        [
            ("struct", "Tag"),
            ("union", "U"),
            ("enum", "E"),
            ("typedef", "Alias"),
            ("typedef", "Pointer"),
            ("typedef", "Array"),
            ("typedef", "Callback"),
            ("function", "prototype"),
            ("function", "add"),
            ("function", "factory"),
        ]
    );
    assert_eq!(
        &text[analysis.facts[3].span.clone()],
        "typedef struct Tag Alias, *Pointer;"
    );
    assert_eq!(analysis.facts[3].span, analysis.facts[4].span);
    assert_eq!(
        &text[analysis
            .facts
            .iter()
            .find(|fact| fact.name == "factory")
            .unwrap()
            .span
            .clone()],
        "int (*factory(void))(int) { return 0; }"
    );
    assert!(analysis.facts.iter().all(|f| f.unique));
}

#[test]
fn cpp_namespaces_templates_operators_and_out_of_line_definitions_have_lexical_scopes() {
    let text = r#"namespace one {
template<class T> class Box {
public:
    Box() {} ~Box() {} T read() const { return {}; }
    T operator+(T other) const { return other; }
    operator bool() const { return true; }
    void declared();
};
using Count = unsigned;
template<class T> T identity(T value) { return value; }
void Box<int>::declared() {}
}
namespace two { int read() { return 2; } }
"#;
    let analysis = analyze("cpp", text);
    assert_eq!(analysis.coverage, Coverage::Syntax);
    assert_eq!(
        analysis
            .facts
            .iter()
            .map(|f| (f.kind.as_str(), f.name.as_str()))
            .collect::<Vec<_>>(),
        [
            ("namespace", "one"),
            ("class", "Box"),
            ("function", "Box"),
            ("function", "~Box"),
            ("function", "read"),
            ("function", "operator+"),
            ("function", "operator bool"),
            ("function", "declared"),
            ("alias", "Count"),
            ("function", "identity"),
            ("function", "Box<int>::declared"),
            ("namespace", "two"),
            ("function", "read"),
        ]
    );
    assert_ne!(analysis.facts[4].key, analysis.facts[12].key);
    assert!(analysis.facts[4].key.contains("class:Box"));
    assert!(analysis.facts[9].key.contains("namespace:one"));
    assert!(!analysis.facts[9].key.contains("class:Box"));
    assert_eq!(
        &text[analysis.facts[6].span.clone()],
        "operator bool() const { return true; }"
    );
}

#[test]
fn conditional_alternatives_and_overloads_stay_ambiguous_without_configuration_claims() {
    let text = "#if FEATURE\nint choose() { return 1; }\n#else\nint choose() { return 2; }\n#endif\nint overload(int x) { return x; } double overload(double x) { return x; }\n";
    let analysis = analyze("cpp", text);
    assert_eq!(analysis.coverage, Coverage::Syntax);
    assert_eq!(
        analysis
            .facts
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
        ["choose", "choose", "overload", "overload"]
    );
    assert!(analysis.facts.iter().all(|f| !f.unique));
    assert_ne!(analysis.facts[0].key, analysis.facts[1].key);
    assert_ne!(analysis.facts[2].key, analysis.facts[3].key);
    for language in ["c", "cpp"] {
        let languages = Languages::builtin().unwrap();
        let mut parser = languages.select(language).unwrap().create().unwrap();
        assert_eq!(
            parser.analyze(b"int broken( {").unwrap().coverage,
            Coverage::Partial
        );
        let bad = parser.analyze(&[255]).unwrap();
        assert_eq!(bad.coverage, Coverage::Unavailable);
        assert!(bad.facts.is_empty());
        assert!(!bad.diagnostics.is_empty());
        assert_eq!(parser.analyze(b"").unwrap().coverage, Coverage::Syntax);
    }
}

#[test]
fn conversions_references_aliases_and_anonymous_types_do_not_capture_parameters_as_names() {
    let text = r#"struct Item {
    Item() = default;
    ~Item() = delete;
    operator bool() const;
    operator int*() const { return nullptr; }
};
Item::operator bool() const { return true; }
int& reference(int& value) { return value; }
template<class T> using Pointer = T*;
typedef struct { int field; } Anonymous, *AnonymousPointer;
class Forward;
struct Item *variable;
namespace outer::inner { int café() { return 1; } }
"#;
    let analysis = analyze("cpp", text);
    assert_eq!(analysis.coverage, Coverage::Syntax);
    assert_eq!(
        analysis
            .facts
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
        [
            "Item",
            "operator int*",
            "Item::operator bool",
            "reference",
            "Pointer",
            "Anonymous",
            "AnonymousPointer",
            "outer::inner",
            "café"
        ]
    );
    assert_eq!(
        &text[analysis.facts[2].span.clone()],
        "Item::operator bool() const { return true; }"
    );
    assert!(!analysis.facts[5].key.contains("typedef:AnonymousPointer"));
    assert!(
        !analysis
            .facts
            .iter()
            .any(|fact| fact.name == "T" || fact.name == "value" || fact.name == "field")
    );
}
