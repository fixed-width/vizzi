//! Rust lexical declarations from raw, unexpanded source.

use crate::model::{Analysis, ContentId, Context, ContextId, Coverage, Fact, ModelError};
use std::collections::BTreeMap;

/// A reusable parser; each worker owns its instance.
pub struct RustAnalyzer {
    parser: tree_sitter::Parser,
    context: Context,
}
impl RustAnalyzer {
    /// Configures the current grammar and implementation fingerprint.
    pub fn new() -> Result<Self, ModelError> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|error| ModelError(error.to_string()))?;
        Ok(Self {
            parser,
            context: Self::context(),
        })
    }
    /// Identifies the current grammar, configuration, and source implementation.
    pub fn context() -> Context {
        Context {
            language: "rust".into(),
            description: "Tree-sitter Rust; raw syntax, no macro expansion".into(),
            id: ContextId::derive(&[
                include_bytes!("rust.rs"),
                include_bytes!("model.rs"),
                include_bytes!("../../../Cargo.lock"),
                b"raw-rust-syntax-v1",
            ]),
        }
    }
    /// Extracts source-backed declarations and reports syntax recovery.
    pub fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError> {
        let mut analysis = Analysis {
            content: ContentId::derive(&[bytes]),
            context: self.context.clone(),
            coverage: Coverage::Syntax,
            facts: Vec::new(),
            diagnostics: Vec::new(),
        };
        let Ok(text) = std::str::from_utf8(bytes) else {
            analysis.coverage = Coverage::Unavailable;
            analysis
                .diagnostics
                .push("Source is not UTF-8; syntax analysis is unavailable".into());
            return Ok(analysis);
        };
        let tree = self
            .parser
            .parse(text, None)
            .ok_or_else(|| ModelError("parser did not return a tree".into()))?;
        if tree.root_node().has_error() {
            analysis.coverage = Coverage::Partial;
            analysis.diagnostics.push(
                "Raw source required grammar recovery; declarations may be incomplete".into(),
            );
        }
        let mut cursor = tree.walk();
        loop {
            let node = cursor.node();
            if let Some((kind, name)) = declaration(node, text) {
                let mut scope = Vec::new();
                let mut parent = node.parent();
                while let Some(ancestor) = parent {
                    if let Some((kind, name)) = declaration(ancestor, text) {
                        scope.push(format!("{kind}:{name}"));
                    }
                    parent = ancestor.parent();
                }
                scope.reverse();
                scope.push(format!("{kind}:{name}"));
                let key =
                    serde_json::to_string(&scope).map_err(|error| ModelError(error.to_string()))?;
                analysis.facts.push(Fact {
                    key,
                    kind: kind.into(),
                    name,
                    span: node.byte_range(),
                    unique: true,
                });
            }
            if cursor.goto_first_child() {
                continue;
            }
            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    let mut counts = BTreeMap::<String, usize>::new();
                    for fact in &analysis.facts {
                        *counts.entry(fact.key.clone()).or_default() += 1;
                    }
                    let mut ordinals = BTreeMap::<String, usize>::new();
                    for fact in &mut analysis.facts {
                        fact.unique = counts[&fact.key] == 1;
                        let ordinal = ordinals.entry(fact.key.clone()).or_default();
                        let key = serde_json::to_string(&(&fact.key, *ordinal))
                            .map_err(|error| ModelError(error.to_string()))?;
                        *ordinal += 1;
                        fact.key = key;
                    }
                    return Ok(analysis);
                }
            }
        }
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
    use crate::model::{Document, WorkspaceId};
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
        let doc = Document::capture(
            WorkspaceId::derive(&[b"t"]),
            "a.rs".into(),
            text,
            vec![analysis],
        )
        .unwrap();
        assert_eq!(
            &doc.source().unwrap()[doc.analyses()[0].facts[1].span.clone()],
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
