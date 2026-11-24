//! Raw Java declarations and written calls from captured source.
use crate::{
    analysis::{Analyzer, LanguageAdapter},
    calls::CallExpression,
    model::{Context, ContextId, ModelError},
    syntax::{CallRules, Declaration, SyntaxAnalyzer},
};
use tree_sitter::Node;

const CALLS: CallRules = CallRules {
    expressions: &[
        CallExpression {
            kind: "method_invocation",
            callee_field: "name",
            receiver_field: Some("object"),
            unwrap_callee: None,
            name_kinds: &["identifier"],
            name_fields: &[],
        },
        CallExpression {
            kind: "object_creation_expression",
            callee_field: "type",
            receiver_field: None,
            unwrap_callee: Some(constructor_name),
            name_kinds: &["type_identifier"],
            name_fields: &[],
        },
    ],
    finish: None,
};

pub(crate) struct JavaAdapter;

impl LanguageAdapter for JavaAdapter {
    fn context(&self) -> Context {
        Context {
            language: "java".into(),
            description: "Tree-sitter Java, raw syntax with written-call candidates".into(),
            id: ContextId::derive(&[
                b"raw-java-syntax-v1",
                include_bytes!("java.rs"),
                include_bytes!("syntax.rs"),
                include_bytes!("calls.rs"),
                include_bytes!("coloring.rs"),
                include_bytes!("syntax_state.rs"),
                include_bytes!("analysis.rs"),
                include_bytes!("model.rs"),
                include_bytes!("captured_source.rs"),
                include_bytes!("../../../Cargo.lock"),
            ]),
        }
    }

    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        Ok(Box::new(SyntaxAnalyzer::new(
            tree_sitter_java::LANGUAGE.into(),
            tree_sitter_java::HIGHLIGHTS_QUERY,
            self.context(),
            collect,
            scope,
            CALLS,
        )?))
    }

    fn supports_written_calls(&self) -> bool {
        CALLS.supported()
    }
}

fn scope(node: Node<'_>, text: &str) -> Option<(&'static str, String)> {
    let kind = match node.kind() {
        "class_declaration" => "class",
        "interface_declaration" => "interface",
        "enum_declaration" => "enum",
        "record_declaration" => "record",
        "annotation_type_declaration" => "annotation",
        "constructor_declaration" | "compact_constructor_declaration" => "function",
        "method_declaration" if node.child_by_field_name("body").is_some() => "function",
        "method_declaration" | "annotation_type_element_declaration" => "signature",
        _ => return None,
    };
    let name = text.get(node.child_by_field_name("name")?.byte_range())?;
    (!name.is_empty()).then(|| (kind, name.into()))
}

fn collect(node: Node<'_>, text: &str, output: &mut Vec<Declaration>) {
    if let Some((kind, name)) = scope(node, text) {
        output.push(Declaration::at(node, kind, name));
    }
}

fn constructor_name(mut node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "generic_type" {
        node = node
            .named_children(&mut node.walk())
            .find(|child| !child.is_extra())?;
    }
    if node.kind() == "scoped_type_identifier" {
        node = node
            .named_children(&mut node.walk())
            .filter(|child| !child.is_extra())
            .last()?;
    }
    Some(node)
}
