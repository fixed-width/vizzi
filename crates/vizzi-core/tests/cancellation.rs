//! Complete results, interruption, and parser recovery through the analyzer interface.

use std::{cell::Cell, rc::Rc};
use vizzi_core::{
    analysis::{Analyzer, Languages},
    model::{Analysis, ModelError},
};

#[test]
fn cancellable_results_match_complete_facts_for_every_builtin_language() {
    let languages = Languages::builtin().unwrap();
    for (name, text) in [
        (
            "rust",
            "mod café { fn same() {} } fn repeated() {} fn repeated() {}",
        ),
        (
            "python",
            "class Café:\n    def same(self): pass\ndef repeated(): pass\ndef repeated(): pass\n",
        ),
        (
            "c",
            "typedef int One, Two; struct Café { int x; }; int same(void) { return 0; }",
        ),
        (
            "cpp",
            "namespace café { class C { int same() { return 0; } }; } using Alias = int;",
        ),
    ] {
        let language = languages.select(name).unwrap();
        let mut parser = language.create().unwrap();
        for bytes in [text.as_bytes(), b"", b"broken ( {", &[255]] {
            let complete = parser.analyze(bytes).unwrap();
            let cancellable = parser
                .analyze_cancellable(bytes, &|| false)
                .unwrap()
                .unwrap();
            assert_eq!(complete, cancellable, "{name}");
            assert!(
                parser
                    .analyze_cancellable(bytes, &|| true)
                    .unwrap()
                    .is_none()
            );
        }
    }
}

#[test]
fn cancellation_inside_parsing_drops_facts_and_resets_before_a_different_input() {
    let languages = Languages::builtin().unwrap();
    for (name, text, next) in [
        (
            "rust",
            "fn repeated() { let x = 12; }\n",
            "struct Different;",
        ),
        (
            "python",
            "def repeated():\n    return 12\n",
            "class Different: pass\n",
        ),
        (
            "c",
            "int repeated(void) { return 12; }\n",
            "struct Different { int x; };",
        ),
        (
            "cpp",
            "int repeated() { return 12; }\n",
            "class Different {};",
        ),
    ] {
        let source = text.repeat(10000);
        let mut parser = languages.select(name).unwrap().create().unwrap();
        let calls = Cell::new(0);
        assert!(
            parser
                .analyze_cancellable(source.as_bytes(), &|| {
                    calls.set(calls.get() + 1);
                    calls.get() >= 20
                })
                .unwrap()
                .is_none(),
            "{name}"
        );
        let actual = parser
            .analyze_cancellable(next.as_bytes(), &|| false)
            .unwrap()
            .unwrap();
        let expected = languages
            .select(name)
            .unwrap()
            .create()
            .unwrap()
            .analyze(next.as_bytes())
            .unwrap();
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn legacy_adapter_checks_before_and_after_its_complete_operation_and_preserves_errors() {
    struct Legacy {
        calls: Rc<Cell<usize>>,
        stop: Rc<Cell<bool>>,
        fail: bool,
    }
    impl Analyzer for Legacy {
        fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError> {
            self.calls.set(self.calls.get() + 1);
            self.stop.set(true);
            if self.fail {
                return Err(ModelError("adapter failed".into()));
            }
            Languages::builtin()
                .unwrap()
                .select("rust")
                .unwrap()
                .create()
                .unwrap()
                .analyze(bytes)
        }
    }
    let calls = Rc::new(Cell::new(0));
    let stop = Rc::new(Cell::new(true));
    let mut adapter = Legacy {
        calls: calls.clone(),
        stop: stop.clone(),
        fail: false,
    };
    assert!(
        adapter
            .analyze_cancellable(b"fn example() {}", &|| stop.get())
            .unwrap()
            .is_none()
    );
    assert_eq!(calls.get(), 0);
    stop.set(false);
    assert!(
        adapter
            .analyze_cancellable(b"fn example() {}", &|| stop.get())
            .unwrap()
            .is_none()
    );
    assert_eq!(calls.get(), 1);
    stop.set(false);
    adapter.fail = true;
    assert!(adapter.analyze_cancellable(b"", &|| stop.get()).is_err());
}
