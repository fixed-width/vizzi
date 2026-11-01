//! Shared raw-syntax traversal and declaration identity construction.

use crate::{
    analysis::Analyzer,
    model::{Analysis, ContentId, Context, Coverage, Fact, ModelError},
};
use std::collections::BTreeMap;

pub(crate) struct SyntaxAnalyzer {
    parser: tree_sitter::Parser,
    context: Context,
    declaration: fn(tree_sitter::Node<'_>, &str) -> Option<(&'static str, String)>,
}
impl SyntaxAnalyzer {
    pub(crate) fn new(
        language: tree_sitter::Language,
        context: Context,
        declaration: fn(tree_sitter::Node<'_>, &str) -> Option<(&'static str, String)>,
    ) -> Result<Self, ModelError> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language)
            .map_err(|error| ModelError(error.to_string()))?;
        Ok(Self {
            parser,
            context,
            declaration,
        })
    }
}
impl Analyzer for SyntaxAnalyzer {
    /// Extracts source-backed declarations and reports syntax recovery.
    fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError> {
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
            if let Some((kind, name)) = (self.declaration)(node, text) {
                let mut scope = Vec::new();
                let mut parent = node.parent();
                while let Some(ancestor) = parent {
                    if let Some((kind, name)) = (self.declaration)(ancestor, text) {
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
