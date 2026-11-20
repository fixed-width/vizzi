//! Raw JavaScript, TypeScript, and JSX evidence from captured source.

use crate::{
    analysis::{Analyzer, LanguageAdapter},
    calls::CallExpression,
    model::{Context, ContextId, ModelError},
    syntax::{CallRules, Declaration, SyntaxAnalyzer},
};
use tree_sitter::Node;

const CALLEE_NAMES: &[&str] = &[
    "identifier",
    "property_identifier",
    "private_property_identifier",
];
const DOTTED_CALLEE: &[(&str, &str)] = &[("member_expression", "property")];
const CALLS: CallRules = CallRules {
    expressions: &[
        CallExpression {
            kind: "call_expression",
            callee_field: "function",
            unwrap_callee: Some(unwrap_expression),
            name_kinds: CALLEE_NAMES,
            name_fields: DOTTED_CALLEE,
        },
        CallExpression {
            kind: "new_expression",
            callee_field: "constructor",
            unwrap_callee: Some(unwrap_expression),
            name_kinds: CALLEE_NAMES,
            name_fields: DOTTED_CALLEE,
        },
    ],
    finish: None,
};

pub(crate) enum Dialect {
    JavaScript,
    TypeScript,
    Tsx,
}

impl LanguageAdapter for Dialect {
    fn context(&self) -> Context {
        let (language, grammar) = match self {
            Self::JavaScript => ("javascript", "JavaScript with JSX"),
            Self::TypeScript => ("typescript", "TypeScript"),
            Self::Tsx => ("tsx", "TSX"),
        };
        Context {
            language: language.into(),
            description: format!("Tree-sitter {grammar}, raw syntax with written-call candidates"),
            id: ContextId::derive(&[
                b"raw-ecmascript-syntax-v1",
                language.as_bytes(),
                include_bytes!("ecmascript.rs"),
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
        use tree_sitter_javascript::{HIGHLIGHT_QUERY, JSX_HIGHLIGHT_QUERY};
        use tree_sitter_typescript::HIGHLIGHTS_QUERY;

        let (grammar, highlights) = match self {
            Self::JavaScript => (
                tree_sitter_javascript::LANGUAGE,
                [HIGHLIGHT_QUERY, JSX_HIGHLIGHT_QUERY].join("\n"),
            ),
            Self::TypeScript => (
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
                [HIGHLIGHTS_QUERY, HIGHLIGHT_QUERY].join("\n"),
            ),
            Self::Tsx => (
                tree_sitter_typescript::LANGUAGE_TSX,
                [HIGHLIGHTS_QUERY, HIGHLIGHT_QUERY, JSX_HIGHLIGHT_QUERY].join("\n"),
            ),
        };
        Ok(Box::new(SyntaxAnalyzer::new(
            grammar.into(),
            &highlights,
            self.context(),
            collect,
            declaration,
            CALLS,
        )?))
    }

    fn supports_written_calls(&self) -> bool {
        CALLS.supported()
    }
}

fn is_class_declaration(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "class_declaration" | "abstract_class_declaration"
    )
}

fn unwrap_expression(mut node: Node<'_>) -> Option<Node<'_>> {
    for _ in 0..256 {
        let value_index = match node.kind() {
            "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
            | "instantiation_expression" => 0,
            "type_assertion" => 1,
            _ => return Some(node),
        };
        let mut cursor = node.walk();
        node = node
            .named_children(&mut cursor)
            .filter(|child| !child.is_extra())
            .nth(value_index)?;
    }
    None
}

fn declaration(node: Node<'_>, text: &str) -> Option<(&'static str, String)> {
    let kind = match node.kind() {
        "function_declaration" | "generator_function_declaration" => "function",
        "class_declaration" | "abstract_class_declaration" => "class",
        "interface_declaration" => "interface",
        "enum_declaration" => "enum",
        "type_alias_declaration" => "type",
        "method_definition" if node.parent()?.parent().is_some_and(is_class_declaration) => {
            "function"
        }
        "variable_declarator"
            if matches!(
                unwrap_expression(node.child_by_field_name("value")?)?.kind(),
                "arrow_function" | "function_expression" | "generator_function"
            ) =>
        {
            "function"
        }
        _ => return None,
    };
    let name = node.child_by_field_name("name")?;
    if !matches!(
        name.kind(),
        "identifier" | "property_identifier" | "private_property_identifier" | "type_identifier"
    ) {
        return None;
    }
    let spelling = text
        .get(name.byte_range())
        .filter(|name| !name.is_empty())?;
    Some((kind, spelling.into()))
}

fn collect(node: Node<'_>, text: &str, output: &mut Vec<Declaration>) {
    if let Some((kind, name)) = declaration(node, text) {
        output.push(Declaration::at(node, kind, name));
    }
}
