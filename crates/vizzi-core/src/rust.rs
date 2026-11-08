//! Rust lexical declarations from raw, unexpanded source.

use crate::{
    analysis::{AnalysisResult, Analyzer, LanguageAdapter, SyntaxState},
    model::{Analysis, Context, ContextId, ModelError},
    syntax::{Declaration, SyntaxAnalyzer},
};

/// Registers the Rust raw-syntax adapter.
pub struct RustAdapter;
impl LanguageAdapter for RustAdapter {
    fn context(&self) -> Context {
        RustAnalyzer::context()
    }
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        Ok(Box::new(RustAnalyzer::new()?))
    }
}

/// A reusable parser; each worker owns its instance.
pub struct RustAnalyzer {
    syntax: SyntaxAnalyzer,
}
impl RustAnalyzer {
    /// Configures the current grammar and implementation fingerprint.
    pub fn new() -> Result<Self, ModelError> {
        Ok(Self {
            syntax: SyntaxAnalyzer::new(
                tree_sitter_rust::LANGUAGE.into(),
                tree_sitter_rust::HIGHLIGHTS_QUERY,
                Self::context(),
                collect,
                declaration,
            )?,
        })
    }
    /// Identifies the current grammar, configuration, and source implementation.
    pub fn context() -> Context {
        Context {
            language: "rust".into(),
            description: "Tree-sitter Rust; raw syntax, no macro expansion".into(),
            id: ContextId::derive(&[
                include_bytes!("rust.rs"),
                include_bytes!("syntax.rs"),
                include_bytes!("calls.rs"),
                include_bytes!("coloring.rs"),
                include_bytes!("syntax_state.rs"),
                include_bytes!("analysis.rs"),
                include_bytes!("model.rs"),
                include_bytes!("captured_source.rs"),
                include_bytes!("../../../Cargo.lock"),
                b"raw-rust-syntax-v1",
            ]),
        }
    }
    /// Extracts source-backed declarations and reports syntax recovery.
    pub fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError> {
        self.syntax.analyze(bytes)
    }
}
impl Analyzer for RustAnalyzer {
    fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError> {
        self.syntax.analyze(bytes)
    }
    fn analyze_cancellable(
        &mut self,
        bytes: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Analysis>, ModelError> {
        self.syntax.analyze_cancellable(bytes, cancelled)
    }
    fn analyze_with_state(
        &mut self,
        bytes: &[u8],
        previous: Option<SyntaxState>,
        retain_weight: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<AnalysisResult>, ModelError> {
        self.syntax
            .analyze_with_state(bytes, previous, retain_weight, cancelled)
    }
}

fn collect(node: tree_sitter::Node<'_>, text: &str, output: &mut Vec<Declaration>) {
    if let Some((kind, name)) = declaration(node, text) {
        output.push(Declaration::at(node, kind, name));
    }
}
fn declaration(node: tree_sitter::Node<'_>, text: &str) -> Option<(&'static str, String)> {
    let kind = match node.kind() {
        "function_item" | "function_signature_item" => "function",
        "struct_item" => "struct",
        "enum_item" => "enum",
        "trait_item" => "trait",
        "mod_item" => "module",
        "type_item" => "type",
        "const_item" => "constant",
        "static_item" => "static",
        "union_item" => "union",
        "impl_item" => "implementation",
        "macro_definition" => "macro",
        _ => return None,
    };
    let name = if node.kind() == "impl_item" {
        let target = text.get(node.child_by_field_name("type")?.byte_range())?;
        match node.child_by_field_name("trait") {
            Some(trait_node) => format!("{} for {target}", text.get(trait_node.byte_range())?),
            None => target.into(),
        }
    } else {
        text.get(node.child_by_field_name("name")?.byte_range())?
            .into()
    };
    Some((kind, name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Coverage, WorkspaceId};
    #[test]
    fn macro_definitions_retain_raw_scope_without_expanding_tokens_or_invocations() {
        let definition = "macro_rules! codecs { () => { fn generated() {} }; }";
        let text = format!("mod registry {{ {definition} codecs!(); fn install() {{}} }}");
        let result = RustAnalyzer::new()
            .unwrap()
            .analyze(text.as_bytes())
            .unwrap();
        assert_eq!(result.coverage, Coverage::Syntax);
        assert_eq!(
            result
                .facts
                .iter()
                .map(|f| (f.kind.as_str(), f.name.as_str()))
                .collect::<Vec<_>>(),
            [
                ("module", "registry"),
                ("macro", "codecs"),
                ("function", "install")
            ]
        );
        let found = &result.facts[1];
        assert_eq!(&text[found.span.clone()], definition);
        assert_eq!(found.parent, Some(0));
        assert!(found.key.contains("registry"));
    }
    #[test]
    fn extracts_qualified_declarations_and_marks_duplicates_ambiguous() {
        let text = b"mod one { fn same() {} } mod two { fn same() {} } fn repeated() {} fn repeated() {} struct Example; impl Example { fn method(&self) {} }";
        let analysis = RustAnalyzer::new().unwrap().analyze(text).unwrap();
        assert_eq!(analysis.coverage, Coverage::Syntax);
        assert_eq!(
            analysis
                .facts
                .iter()
                .map(|fact| fact.name.as_str())
                .collect::<Vec<_>>(),
            [
                "one", "same", "two", "same", "repeated", "repeated", "Example", "Example",
                "method"
            ]
        );
        let same: Vec<_> = analysis
            .facts
            .iter()
            .filter(|fact| fact.name == "same")
            .collect();
        assert_ne!(same[0].key, same[1].key);
        assert!(same.iter().all(|fact| fact.unique));
        assert!(
            analysis
                .facts
                .iter()
                .filter(|fact| fact.name == "repeated")
                .all(|fact| !fact.unique)
        );
        let doc = crate::model::fixture_document(
            WorkspaceId::derive(&[b"t"]),
            "a.rs".into(),
            text,
            vec![analysis],
        )
        .unwrap();
        assert_eq!(
            &doc.resident_source().unwrap()[doc.analyses()[0].facts[1].span.clone()],
            "fn same() {}"
        );
    }
    #[test]
    fn malformed_and_non_utf8_sources_report_their_limits() {
        let mut parser = RustAnalyzer::new().unwrap();
        let broken = parser.analyze(b"fn broken( {").unwrap();
        assert_eq!(broken.coverage, Coverage::Partial);
        assert!(!broken.diagnostics.is_empty());
        let invalid = parser.analyze(&[0xff]).unwrap();
        assert_eq!(invalid.coverage, Coverage::Unavailable);
        assert!(invalid.facts.is_empty());
        assert!(!invalid.diagnostics.is_empty());
        assert_eq!(parser.analyze(b"").unwrap().coverage, Coverage::Syntax);
    }

    #[test]
    fn outlines_declaration_kinds_trait_methods_and_nested_scopes() {
        let text = b"enum E { A } trait T { fn required(&self); } type Alias = E; const C: u8 = 0; static S: u8 = 0; union U { a: u8 } impl T for E { fn required(&self) {} } mod outer { mod inner { struct Nested; } }";
        let result = RustAnalyzer::new().unwrap().analyze(text).unwrap();
        assert_eq!(result.coverage, Coverage::Syntax);
        assert_eq!(
            result
                .facts
                .iter()
                .map(|fact| (fact.kind.as_str(), fact.name.as_str()))
                .collect::<Vec<_>>(),
            [
                ("enum", "E"),
                ("trait", "T"),
                ("function", "required"),
                ("type", "Alias"),
                ("constant", "C"),
                ("static", "S"),
                ("union", "U"),
                ("implementation", "T for E"),
                ("function", "required"),
                ("module", "outer"),
                ("module", "inner"),
                ("struct", "Nested"),
            ]
        );
        assert_ne!(result.facts[2].key, result.facts[8].key);
        assert!(
            result.facts.last().unwrap().key.find("outer").unwrap()
                < result.facts.last().unwrap().key.find("inner").unwrap()
        );
    }
}
