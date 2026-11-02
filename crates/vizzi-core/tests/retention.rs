//! Incremental syntax state must preserve complete fresh-analysis results.

use vizzi_core::analysis::Languages;

#[test]
fn owned_state_reuses_trees_and_matches_fresh_facts_across_all_four_languages() {
    let languages = Languages::builtin().unwrap();
    for (name, initial, changes) in [
        (
            "rust",
            "mod café { fn alpha() {} fn repeated() {} fn repeated() {} }",
            vec![
                "// π\r\nmod café { fn alpha() {} fn repeated() {} fn repeated() {} }",
                "mod cafê { fn beta() {} }",
                "fn broken( {",
                "",
                "struct Last;",
            ],
        ),
        (
            "python",
            "class Café:\n    def alpha(self): pass\n",
            vec![
                "# π\r\nclass Café:\n    def alpha(self): pass\n",
                "class Cafê:\n    def beta(self):\n        return 'λ'\n",
                "def broken( :",
                "",
                "class Last: pass\n",
            ],
        ),
        (
            "c",
            "struct Café { int x; }; int alpha(void) { return 1; }",
            vec![
                "// π\r\nstruct Café { int x; }; int alpha(void) { return 1; }",
                "typedef int One, Two; int beta(void) { return 2; }",
                "int broken( {",
                "",
                "struct Last { int value; };",
            ],
        ),
        (
            "cpp",
            "namespace café { int alpha() { return 1; } }",
            vec![
                "// π\r\nnamespace café { int alpha() { return 1; } }",
                "namespace cafê { class Last { int beta() { return 2; } }; }",
                "class Broken {",
                "",
                "using Last = int;",
            ],
        ),
    ] {
        let language = languages.select(name).unwrap();
        let mut parser = language.create().unwrap();
        let first = parser
            .analyze_with_state(initial.as_bytes(), None, usize::MAX, &|| false)
            .unwrap()
            .unwrap();
        assert!(!first.tree_reused);
        assert_eq!(first.state.as_ref().unwrap().source_bytes(), initial.len());
        assert!(first.state.as_ref().unwrap().nodes() > 1);
        let mut state = first.state;
        assert!(state.is_some());
        for text in changes {
            let result = parser
                .analyze_with_state(text.as_bytes(), state, usize::MAX, &|| false)
                .unwrap()
                .unwrap();
            let fresh = language.create().unwrap().analyze(text.as_bytes()).unwrap();
            assert_eq!(result.analysis, fresh, "{name}: {text}");
            assert!(result.tree_reused);
            state = result.state;
        }
    }
}

#[test]
fn context_mismatch_invalid_encoding_and_retention_limits_discard_state_safely() {
    let languages = Languages::builtin().unwrap();
    let mut rust = languages.select("rust").unwrap().create().unwrap();
    let first = rust
        .analyze_with_state(b"fn first() {}", None, usize::MAX, &|| false)
        .unwrap()
        .unwrap();
    let weight = first.state.as_ref().unwrap().weight();
    assert!(weight > b"fn first() {}".len());
    assert!(
        rust.analyze_with_state(b"fn first() {}", None, weight, &|| false)
            .unwrap()
            .unwrap()
            .state
            .is_some()
    );
    assert!(
        rust.analyze_with_state(b"fn first() {}", None, weight - 1, &|| false)
            .unwrap()
            .unwrap()
            .state
            .is_none()
    );
    let mut python = languages.select("python").unwrap().create().unwrap();
    let changed = python
        .analyze_with_state(b"class Different: pass\n", first.state, usize::MAX, &|| {
            false
        })
        .unwrap()
        .unwrap();
    assert!(!changed.tree_reused);
    assert_eq!(changed.analysis.facts[0].name, "Different");
    let invalid = python
        .analyze_with_state(&[255], changed.state, usize::MAX, &|| false)
        .unwrap()
        .unwrap();
    assert!(invalid.state.is_none());
    assert!(!invalid.tree_reused);
    assert!(invalid.analysis.facts.is_empty());
}

#[test]
fn cancelled_state_is_discarded_and_the_parser_recovers_for_another_input() {
    let languages = Languages::builtin().unwrap();
    let mut parser = languages.select("rust").unwrap().create().unwrap();
    let first = parser
        .analyze_with_state(b"fn first() {}", None, usize::MAX, &|| false)
        .unwrap()
        .unwrap();
    assert!(
        parser
            .analyze_with_state(b"fn changed() {}", first.state, usize::MAX, &|| true)
            .unwrap()
            .is_none()
    );
    let next = parser
        .analyze_with_state(b"struct Different;", None, 0, &|| false)
        .unwrap()
        .unwrap();
    assert!(next.state.is_none());
    assert!(!next.tree_reused);
    assert_eq!(next.analysis.facts[0].name, "Different");
}

#[test]
fn legacy_adapters_can_decline_retention_without_changing_their_results() {
    use vizzi_core::{
        analysis::Analyzer,
        model::{Analysis, ModelError},
        rust::RustAnalyzer,
    };
    struct Legacy(RustAnalyzer);
    impl Analyzer for Legacy {
        fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError> {
            self.0.analyze(bytes)
        }
    }
    let mut legacy = Legacy(RustAnalyzer::new().unwrap());
    let result = legacy
        .analyze_with_state(b"fn legacy() {}", None, usize::MAX, &|| false)
        .unwrap()
        .unwrap();
    assert!(result.state.is_none());
    assert!(!result.tree_reused);
    assert_eq!(result.analysis.facts[0].name, "legacy");
}

#[test]
fn empty_tree_has_a_defined_admission_weight_and_retained_parse_can_be_interrupted() {
    use std::cell::Cell;
    let language = Languages::builtin().unwrap();
    let mut parser = language.select("rust").unwrap().create().unwrap();
    let empty = parser
        .analyze_with_state(b"", None, usize::MAX, &|| false)
        .unwrap()
        .unwrap()
        .state
        .unwrap();
    assert_eq!(empty.nodes(), 1);
    assert_eq!(empty.source_bytes(), 0);
    assert_eq!(empty.weight(), 4224);
    let polls = Cell::new(0);
    let text = "fn repeated() { let n = 2; }\n".repeat(10000);
    assert!(
        parser
            .analyze_with_state(text.as_bytes(), Some(empty), usize::MAX, &|| {
                polls.set(polls.get() + 1);
                polls.get() >= 100
            })
            .unwrap()
            .is_none()
    );
    let recovered = parser
        .analyze_with_state(b"struct Recovered;", None, usize::MAX, &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(recovered.analysis.facts[0].name, "Recovered");
}
