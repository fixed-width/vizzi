//! Language adapters exercised through registration and captured analysis.

use vizzi_core::{
    analysis::Languages,
    model::{Coverage, Document, WorkspaceId},
};

#[test]
fn registration_rejects_ambiguous_selection_and_invalid_extensions() {
    use std::sync::Arc;
    use vizzi_core::{
        analysis::{Analyzer, LanguageAdapter},
        model::{Context, ContextId, ModelError},
        rust::RustAdapter,
    };
    struct Adapter {
        extensions: Vec<&'static str>,
        id: u8,
    }
    impl LanguageAdapter for Adapter {
        fn extensions(&self) -> &[&str] {
            &self.extensions
        }
        fn context(&self) -> Context {
            Context {
                language: "test".into(),
                description: "fixture".into(),
                id: ContextId::derive(&[&[self.id]]),
            }
        }
        fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
            Err(ModelError("factory failure".into()))
        }
    }
    for extensions in [
        vec![],
        vec![""],
        vec![".rs"],
        vec!["r/s"],
        vec!["é"],
        vec!["rs", "rs"],
    ] {
        assert!(Languages::new(vec![Arc::new(Adapter { extensions, id: 1 })]).is_err());
    }
    assert!(
        Languages::new(vec![
            Arc::new(Adapter {
                extensions: vec!["x"],
                id: 1
            }),
            Arc::new(Adapter {
                extensions: vec!["y"],
                id: 1
            })
        ])
        .is_err()
    );
    assert!(
        Languages::new(vec![
            Arc::new(Adapter {
                extensions: vec!["rs"],
                id: 1
            }),
            Arc::new(RustAdapter)
        ])
        .is_err()
    );
    let languages = Languages::new(vec![Arc::new(Adapter {
        extensions: vec!["x", "y1"],
        id: 1,
    })])
    .unwrap();
    assert_eq!(
        languages.select("x").unwrap().context(),
        languages.select("y1").unwrap().context()
    );
    assert!(languages.select("x").unwrap().create().is_err());
    assert!(Languages::new(vec![]).unwrap().select("rs").is_none());
}

#[test]
fn python_adapter_preserves_scopes_ranges_and_duplicate_ambiguity() {
    let languages = Languages::builtin().unwrap();
    let language = languages.select("py").unwrap();
    assert_eq!(language.context().language, "python");
    assert_ne!(
        language.context().id,
        languages.select("rs").unwrap().context().id
    );
    for extension in ["h", "hpp", "c", "cpp", "cu", "cuh", "lpc", "PY", ""] {
        assert!(languages.select(extension).is_none(), "{extension}");
    }
    let text = "@decorator\nclass Café:\n    async def read(self):\n        def nested():\n            pass\n        return nested()\n\ndef repeated(): pass\ndef repeated(): pass\nclass Other:\n    def read(self): pass\n";
    let mut parser = language.create().unwrap();
    let analysis = parser.analyze(text.as_bytes()).unwrap();
    assert_eq!(analysis.context, *language.context());
    assert_eq!(analysis.coverage, Coverage::Syntax);
    assert!(analysis.diagnostics.is_empty());
    assert_eq!(
        analysis
            .facts
            .iter()
            .map(|fact| (fact.kind.as_str(), fact.name.as_str()))
            .collect::<Vec<_>>(),
        [
            ("class", "Café"),
            ("function", "read"),
            ("function", "nested"),
            ("function", "repeated"),
            ("function", "repeated"),
            ("class", "Other"),
            ("function", "read"),
        ]
    );
    assert_ne!(analysis.facts[1].key, analysis.facts[6].key);
    assert!(analysis.facts[1].unique);
    assert!(!analysis.facts[3].unique);
    assert!(!analysis.facts[4].unique);
    assert_ne!(analysis.facts[3].key, analysis.facts[4].key);
    assert_eq!(
        &text[analysis.facts[2].span.clone()],
        "def nested():\n            pass"
    );
    let document = Document::capture(
        WorkspaceId::derive(&[b"test"]),
        "a.py".into(),
        text.as_bytes(),
        vec![analysis],
    )
    .unwrap();
    assert_eq!(document.lines(&document.analyses()[0].facts[1].span), 3..6);
    assert_eq!(
        parser.analyze(b"def broken( :").unwrap().coverage,
        Coverage::Partial
    );
    let unavailable = parser.analyze(&[255]).unwrap();
    assert_eq!(unavailable.coverage, Coverage::Unavailable);
    assert!(unavailable.facts.is_empty());
    assert!(!unavailable.diagnostics.is_empty());
    assert_eq!(
        parser.analyze(b"# empty\n").unwrap().coverage,
        Coverage::Syntax
    );
}
