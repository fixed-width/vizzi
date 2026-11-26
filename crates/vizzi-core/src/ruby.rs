//! Ruby declarations from captured source, without executing project code.
use crate::{
    analysis::{Analyzer, LanguageAdapter},
    model::{Context, ContextId, ModelError},
    syntax::{CallRules, Declaration, SyntaxAnalyzer, unparenthesized},
};
use tree_sitter::Node;

const CALLS: CallRules = CallRules {
    expressions: &[],
    finish: Some(crate::ruby_calls::collect),
};

/// Registers raw Ruby syntax independently of a runtime or language server.
pub struct RubyAdapter;
impl LanguageAdapter for RubyAdapter {
    fn context(&self) -> Context {
        Context {
            language: "ruby".into(),
            description: "Tree-sitter Ruby; raw syntax, no runtime dispatch resolution".into(),
            id: ContextId::derive(&[
                include_bytes!("ruby.rs"),
                include_bytes!("ruby-highlights.scm"),
                include_bytes!("ruby_calls.rs"),
                include_bytes!("syntax.rs"),
                include_bytes!("calls.rs"),
                include_bytes!("coloring.rs"),
                include_bytes!("syntax_state.rs"),
                include_bytes!("analysis.rs"),
                include_bytes!("model.rs"),
                include_bytes!("captured_source.rs"),
                include_bytes!("../../../Cargo.lock"),
                b"raw-ruby-syntax-v1",
            ]),
        }
    }
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        Ok(Box::new(
            SyntaxAnalyzer::new(
                tree_sitter_ruby::LANGUAGE.into(),
                include_str!("ruby-highlights.scm"),
                self.context(),
                collect,
                scope,
                CALLS,
            )?
            .with_entry_point(entry_point),
        ))
    }
    fn supports_written_calls(&self) -> bool {
        CALLS.supported()
    }
}
fn entry_point(node: Node<'_>, text: &str) -> bool {
    if !matches!(node.kind(), "if" | "if_modifier")
        || node
            .parent()
            .is_none_or(|parent| parent.kind() != "program")
    {
        return false;
    }
    let Some(condition) = node
        .child_by_field_name("condition")
        .map(unparenthesized)
        .filter(|node| node.kind() == "binary")
    else {
        return false;
    };
    condition
        .child_by_field_name("operator")
        .is_some_and(|operator| operator.kind() == "==")
        && condition
            .child_by_field_name("left")
            .zip(condition.child_by_field_name("right"))
            .is_some_and(|(left, right)| {
                let left = text.get(left.byte_range());
                let right = text.get(right.byte_range());
                matches!(
                    (left, right),
                    (Some("__FILE__"), Some("$0" | "$PROGRAM_NAME"))
                        | (Some("$0" | "$PROGRAM_NAME"), Some("__FILE__"))
                )
            })
}

fn declaration(node: Node<'_>, text: &str) -> Option<(&'static str, String)> {
    if node.kind() == "assignment" {
        let left = node.child_by_field_name("left")?;
        if matches!(left.kind(), "constant" | "scope_resolution") {
            return Some(("constant", text.get(left.byte_range())?.into()));
        }
        return None;
    }
    let kind = match node.kind() {
        "module" => "module",
        "class" => "class",
        "method" | "singleton_method" => "function",
        _ => return None,
    };
    Some((
        kind,
        text.get(node.child_by_field_name("name")?.byte_range())?
            .into(),
    ))
}
fn scope(node: Node<'_>, text: &str) -> Option<(&'static str, String)> {
    if node.kind() == "singleton_class" {
        return Some((
            "singleton",
            text.get(node.child_by_field_name("value")?.byte_range())?
                .into(),
        ));
    }
    let (kind, name) = declaration(node, text)?;
    if node.kind() == "singleton_method" {
        let object = text.get(node.child_by_field_name("object")?.byte_range())?;
        Some((
            kind,
            format!(
                "singleton {}.{name}",
                blake3::hash(object.as_bytes()).to_hex()
            ),
        ))
    } else {
        Some((kind, name))
    }
}
fn collect(node: Node<'_>, text: &str, output: &mut Vec<Declaration>) {
    if let Some((kind, name)) = declaration(node, text) {
        let mut declaration = Declaration::at(node, kind, name);
        if kind == "constant" {
            declaration.name_span = node
                .child_by_field_name("left")
                .map(|left| left.byte_range());
        }
        if node.kind() == "singleton_method" {
            declaration.identity = scope(node, text).map(|(_, key)| key);
        }
        output.push(declaration);
    }
}
