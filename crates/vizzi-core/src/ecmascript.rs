//! Raw JavaScript, TypeScript, and JSX evidence from captured source.

use crate::{
    analysis::{Analyzer, LanguageAdapter},
    calls::CallExpression,
    model::{Context, ContextId, ModelError},
    syntax::{CallRules, Declaration, SyntaxAnalyzer, unparenthesized},
};
use tree_sitter::Node;

const COMPUTED_LABEL_BYTES: usize = 128;
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
            receiver_field: None,
            unwrap_callee: Some(unwrap_expression),
            name_kinds: CALLEE_NAMES,
            name_fields: DOTTED_CALLEE,
        },
        CallExpression {
            kind: "new_expression",
            callee_field: "constructor",
            receiver_field: None,
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
        Ok(Box::new(
            SyntaxAnalyzer::new(
                grammar.into(),
                &highlights,
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
    if node.kind() != "if_statement"
        || node
            .parent()
            .is_none_or(|parent| parent.kind() != "program")
    {
        return false;
    }
    let Some(condition) = node.child_by_field_name("condition").map(unparenthesized) else {
        return false;
    };
    if let Some(object) = main_member_object(condition, text) {
        return object.kind() == "meta_property"
            && object
                .children(&mut object.walk())
                .any(|token| token.kind() == "import");
    }
    condition.kind() == "binary_expression"
        && condition
            .child_by_field_name("operator")
            .is_some_and(|operator| operator.kind() == "===")
        && condition
            .child_by_field_name("left")
            .zip(condition.child_by_field_name("right"))
            .is_some_and(|(left, right)| {
                let (left, right) = (unparenthesized(left), unparenthesized(right));
                commonjs_guard(left, right, text) || commonjs_guard(right, left, text)
            })
}

fn main_member_object<'tree>(node: Node<'tree>, text: &str) -> Option<Node<'tree>> {
    if node.kind() != "member_expression" || node.child_by_field_name("optional_chain").is_some() {
        return None;
    }
    let property = node.child_by_field_name("property")?;
    (text.get(property.byte_range()) == Some("main")).then_some(node.child_by_field_name("object")?)
}

fn commonjs_guard(member: Node<'_>, module: Node<'_>, text: &str) -> bool {
    module.kind() == "identifier"
        && text.get(module.byte_range()) == Some("module")
        && main_member_object(member, text).is_some_and(|object| {
            object.kind() == "identifier" && text.get(object.byte_range()) == Some("require")
        })
}

fn is_class_declaration(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "class_declaration" | "abstract_class_declaration"
    )
}

fn expression_child(kind: &str) -> Option<usize> {
    match kind {
        "parenthesized_expression"
        | "non_null_expression"
        | "as_expression"
        | "satisfies_expression"
        | "instantiation_expression" => Some(0),
        "type_assertion" => Some(1),
        _ => None,
    }
}

fn unwrap_expression(mut node: Node<'_>) -> Option<Node<'_>> {
    for _ in 0..256 {
        let Some(value_index) = expression_child(node.kind()) else {
            return Some(node);
        };
        let mut cursor = node.walk();
        node = node
            .named_children(&mut cursor)
            .filter(|child| !child.is_extra())
            .nth(value_index)?;
    }
    None
}

fn enclosing_expression(mut node: Node<'_>) -> Option<Node<'_>> {
    for _ in 0..256 {
        node = node.parent()?;
        if expression_child(node.kind()).is_none() {
            return Some(node);
        }
    }
    None
}

fn named_method(node: Node<'_>) -> bool {
    node.parent()
        .is_some_and(|container| match container.kind() {
            "class_body" => container.parent().is_some_and(is_class_declaration),
            "object" => enclosing_expression(container).is_some_and(|binding| {
                binding.kind() == "variable_declarator"
                    && binding
                        .child_by_field_name("name")
                        .is_some_and(|name| name.kind() == "identifier")
            }),
            _ => false,
        })
}

fn declaration(node: Node<'_>, text: &str) -> Option<Declaration> {
    let kind = match node.kind() {
        "function_declaration" | "generator_function_declaration" => "function",
        "class_declaration" | "abstract_class_declaration" => "class",
        "interface_declaration" => "interface",
        "enum_declaration" => "enum",
        "type_alias_declaration" => "type",
        "function_signature" | "method_signature" | "abstract_method_signature" => "signature",
        "method_definition" if named_method(node) => "function",
        "variable_declarator" => {
            match unwrap_expression(node.child_by_field_name("value")?)?.kind() {
                "arrow_function" | "function_expression" | "generator_function" => "function",
                "object" => "object",
                _ => return None,
            }
        }
        "arrow_function" | "function_expression" | "generator_function"
            if node.child_by_field_name("name").is_none()
                && enclosing_expression(node)
                    .is_some_and(|parent| parent.kind() == "arguments") =>
        {
            let mut declaration = Declaration::at(node, "callback", "callback".into());
            declaration.owns_calls = false;
            return Some(declaration);
        }
        _ => return None,
    };
    let name = node.child_by_field_name("name")?;
    let mut range = name.byte_range();
    let kind = match name.kind() {
        "identifier"
        | "property_identifier"
        | "private_property_identifier"
        | "type_identifier" => kind,
        "computed_property_name" => {
            if kind == "signature" {
                kind
            } else {
                "computed_method"
            }
        }
        "string" if !name.has_error() && range.len() > 2 => {
            range = range.start + 1..range.end - 1;
            if text.get(range.clone())?.contains(['\\', '\r', '\n']) {
                return None;
            }
            kind
        }
        _ => return None,
    };
    let spelling = text.get(range.clone()).filter(|name| !name.is_empty())?;
    let (label, identity) =
        if name.kind() == "computed_property_name" && spelling.len() > COMPUTED_LABEL_BYTES {
            let suffix = "…]";
            let end = spelling.floor_char_boundary(COMPUTED_LABEL_BYTES - suffix.len());
            (
                format!("{}{suffix}", &spelling[..end]),
                Some(format!(
                    "computed:{}",
                    blake3::hash(spelling.as_bytes()).to_hex()
                )),
            )
        } else {
            (spelling.into(), None)
        };
    let mut declaration = Declaration::at(node, kind, label);
    declaration.identity = identity;
    declaration.definition_anchor = (name.kind() == "string").then(|| name.start_byte());
    declaration.name_span = (name.kind() != "computed_property_name").then_some(range);
    declaration.owns_calls = !matches!(kind, "object" | "signature" | "computed_method");
    Some(declaration)
}

fn scope(node: Node<'_>, text: &str) -> Option<(&'static str, String)> {
    let declaration = declaration(node, text)?;
    Some((
        declaration.kind,
        declaration.identity.unwrap_or(declaration.name),
    ))
}

fn collect(node: Node<'_>, text: &str, output: &mut Vec<Declaration>) {
    if let Some(declaration) = declaration(node, text) {
        output.push(declaration);
    }
}
