//! Python class and function declarations from captured, unexecuted source.

use crate::{
    analysis::{Analyzer, LanguageAdapter},
    calls::CallExpression,
    model::{Context, ContextId, ModelError},
    syntax::{CallRules, Declaration, SyntaxAnalyzer, unparenthesized},
};

use tree_sitter::Node;

const CALLS: CallRules = CallRules {
    expressions: &[CallExpression {
        kind: "call",
        callee_field: "function",
        receiver_field: None,
        unwrap_callee: None,
        name_kinds: &["identifier"],
        name_fields: &[("attribute", "attribute")],
    }],
    finish: None,
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
                include_bytes!("calls.rs"),
                include_bytes!("coloring.rs"),
                include_bytes!("syntax_state.rs"),
                include_bytes!("analysis.rs"),
                include_bytes!("model.rs"),
                include_bytes!("captured_source.rs"),
                include_bytes!("../../../Cargo.lock"),
                b"raw-python-syntax-v1",
            ]),
        }
    }
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        Ok(Box::new(
            SyntaxAnalyzer::new(
                tree_sitter_python::LANGUAGE.into(),
                tree_sitter_python::HIGHLIGHTS_QUERY,
                self.context(),
                collect,
                declaration,
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
    if node.kind() != "if_statement" || node.parent().is_none_or(|parent| parent.kind() != "module")
    {
        return false;
    }
    let mut condition = node.child_by_field_name("condition");
    let mut remaining = Vec::new();
    while let Some(expression) = condition.map(unparenthesized) {
        if expression.kind() == "boolean_operator"
            && expression
                .child_by_field_name("operator")
                .is_some_and(|operator| operator.kind() == "and")
        {
            remaining.extend(expression.child_by_field_name("right"));
            condition = expression.child_by_field_name("left");
        } else if main_comparison(expression, text) {
            return true;
        } else {
            condition = None;
        }
        condition = condition.or_else(|| remaining.pop());
    }
    false
}

fn main_comparison(condition: Node<'_>, text: &str) -> bool {
    if condition.kind() != "comparison_operator" {
        return false;
    }
    let mut operator_cursor = condition.walk();
    let mut operators = condition.children_by_field_name("operators", &mut operator_cursor);
    if operators
        .next()
        .is_none_or(|operator| operator.kind() != "==")
        || operators.next().is_some()
    {
        return false;
    }
    let mut cursor = condition.walk();
    let mut operands = condition
        .named_children(&mut cursor)
        .filter(|child| !child.is_extra());
    operands
        .next()
        .zip(operands.next())
        .is_some_and(|(left, right)| {
            let (left, right) = (unparenthesized(left), unparenthesized(right));
            main_guard(left, right, text) || main_guard(right, left, text)
        })
}

fn main_guard(name: Node<'_>, value: Node<'_>, text: &str) -> bool {
    name.kind() == "identifier"
        && text.get(name.byte_range()) == Some("__name__")
        && value.kind() == "string"
        && matches!(
            text.get(value.byte_range()),
            Some("'__main__'" | "\"__main__\"" | "'''__main__'''" | "\"\"\"__main__\"\"\"")
        )
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
