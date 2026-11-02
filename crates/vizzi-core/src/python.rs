//! Python class and function declarations from captured, unexecuted source.

use crate::{
    analysis::{Analyzer, LanguageAdapter},
    model::{Context, ContextId, ModelError},
    syntax::{Declaration, SyntaxAnalyzer},
};

/// Registers raw Python syntax; imports and annotations are not resolved.
pub struct PythonAdapter;
impl LanguageAdapter for PythonAdapter {
    fn context(&self) -> Context {
        Context {
            language: "python".into(),
            description: "Tree-sitter Python; raw syntax, no import or type resolution".into(),
            id: ContextId::derive(&[
                include_bytes!("python.rs"),
                include_bytes!("syntax.rs"),
                include_bytes!("analysis.rs"),
                include_bytes!("model.rs"),
                include_bytes!("../../../Cargo.lock"),
                b"raw-python-syntax-v1",
            ]),
        }
    }
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        Ok(Box::new(SyntaxAnalyzer::new(
            tree_sitter_python::LANGUAGE.into(),
            self.context(),
            collect,
            declaration,
        )?))
    }
}
fn collect(node: tree_sitter::Node<'_>, text: &str, output: &mut Vec<Declaration>) {
    if let Some((kind, name)) = declaration(node, text) {
        output.push(Declaration::at(node, kind, name));
    }
}
fn declaration(node: tree_sitter::Node<'_>, text: &str) -> Option<(&'static str, String)> {
    let kind = match node.kind() {
        "class_definition" => "class",
        "function_definition" => "function",
        _ => return None,
    };
    let name = text.get(node.child_by_field_name("name")?.byte_range())?;
    Some((kind, name.into()))
}
