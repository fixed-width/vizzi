//! Raw C/C++ definitions and type aliases, without preprocessing or name resolution.

use crate::{
    analysis::{Analyzer, LanguageAdapter},
    calls::CallExpression,
    model::{Context, ContextId, ModelError},
    syntax::{CallRules, Declaration, SyntaxAnalyzer, unparenthesized},
};
use tree_sitter::Node;

const CALLS: CallRules = CallRules {
    expressions: &[CallExpression {
        kind: "call_expression",
        callee_field: "function",
        receiver_field: None,
        unwrap_callee: None,
        name_kinds: &["identifier", "field_identifier", "type_identifier"],
        name_fields: &[
            ("field_expression", "field"),
            ("qualified_identifier", "name"),
            ("template_function", "name"),
            ("template_method", "name"),
        ],
    }],
    finish: None,
};

/// Registers raw C syntax for explicitly selected C inputs.
pub struct CAdapter;
impl LanguageAdapter for CAdapter {
    fn context(&self) -> Context {
        context(
            "c",
            "Tree-sitter C; raw syntax, no preprocessing or build configuration",
        )
    }
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        Ok(Box::new(
            SyntaxAnalyzer::new(
                tree_sitter_c::LANGUAGE.into(),
                tree_sitter_c::HIGHLIGHT_QUERY,
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
/// Registers raw C++ syntax for explicitly selected C++ inputs.
pub struct CppAdapter;
impl LanguageAdapter for CppAdapter {
    fn context(&self) -> Context {
        context(
            "cpp",
            "Tree-sitter C++; raw syntax, no preprocessing or build configuration",
        )
    }
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        Ok(Box::new(
            SyntaxAnalyzer::new(
                tree_sitter_cpp::LANGUAGE.into(),
                &format!(
                    "{}\n{}",
                    tree_sitter_c::HIGHLIGHT_QUERY,
                    tree_sitter_cpp::HIGHLIGHT_QUERY
                ),
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
fn context(language: &str, description: &str) -> Context {
    Context {
        language: language.into(),
        description: description.into(),
        id: ContextId::derive(&[
            b"raw-c-family-syntax-v1",
            language.as_bytes(),
            include_bytes!("c_family.rs"),
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

fn entry_point(node: Node<'_>, text: &str) -> bool {
    if node.kind() != "function_definition" || node.child_by_field_name("body").is_none() {
        return false;
    }
    let Some(declarator) = node.child_by_field_name("declarator").map(unparenthesized) else {
        return false;
    };
    if declarator.kind() != "function_declarator"
        || declarator
            .child_by_field_name("declarator")
            .map(unparenthesized)
            .is_none_or(|name| {
                name.kind() != "identifier" || text.get(name.byte_range()) != Some("main")
            })
        || node.named_children(&mut node.walk()).any(|child| {
            child.kind() == "storage_class_specifier"
                && text.get(child.byte_range()) == Some("static")
        })
    {
        return false;
    }
    let return_type = node.child_by_field_name("type");
    let trailing_return = declarator
        .named_children(&mut declarator.walk())
        .find(|child| child.kind() == "trailing_return_type");
    let returns_int = match trailing_return {
        Some(trailing) => trailing.named_child(0).is_some_and(|descriptor| {
            descriptor.child_by_field_name("declarator").is_none()
                && int_type(descriptor.child_by_field_name("type"), text)
        }),
        None => int_type(return_type, text),
    };
    if !returns_int {
        return false;
    }
    let mut parent = node.parent();
    while let Some(scope) = parent {
        match scope.kind() {
            "translation_unit" => break,
            "preproc_if" | "preproc_ifdef" | "preproc_elif" | "preproc_elifdef"
            | "preproc_else" => {
                parent = scope.parent();
            }
            _ => return false,
        }
    }
    declarator
        .child_by_field_name("parameters")
        .is_some_and(|parameters| {
            let mut cursor = parameters.walk();
            let mut params = parameters
                .named_children(&mut cursor)
                .filter(|child| !child.is_extra());
            match (params.next(), params.next(), params.next()) {
                (None, None, None) => true,
                (Some(only), None, None) => {
                    only.kind() == "parameter_declaration"
                        && only
                            .child_by_field_name("type")
                            .and_then(|ty| text.get(ty.byte_range()))
                            == Some("void")
                        && only.child_by_field_name("declarator").is_none()
                }
                (Some(argc), Some(argv), None) => {
                    argc.kind() == "parameter_declaration"
                        && int_type(argc.child_by_field_name("type"), text)
                        && argc
                            .child_by_field_name("declarator")
                            .is_none_or(|name| name.kind() == "identifier")
                        && argv_parameter(argv, text)
                }
                _ => false,
            }
        })
}

fn int_type(node: Option<Node<'_>>, text: &str) -> bool {
    node.and_then(|node| text.get(node.byte_range()))
        .is_some_and(|ty| matches!(ty, "int" | "signed" | "signed int" | "int signed"))
}

fn argv_parameter(node: Node<'_>, text: &str) -> bool {
    if node.kind() != "parameter_declaration"
        || node
            .child_by_field_name("type")
            .and_then(|ty| text.get(ty.byte_range()))
            != Some("char")
    {
        return false;
    }
    let declarator = node.child_by_field_name("declarator");
    let inner = declarator.and_then(|node| node.child_by_field_name("declarator"));
    let pointer = |node: Option<Node<'_>>| {
        matches!(
            node.map(|node| node.kind()),
            Some("pointer_declarator" | "abstract_pointer_declarator")
        )
    };
    let array = |node: Option<Node<'_>>| {
        matches!(
            node.map(|node| node.kind()),
            Some("array_declarator" | "abstract_array_declarator")
        )
    };
    let leaf = inner.and_then(|node| node.child_by_field_name("declarator"));
    leaf.is_none_or(|node| node.kind() == "identifier")
        && ((pointer(declarator) && (pointer(inner) || array(inner)))
            || (array(declarator) && pointer(inner)))
}

fn collect(node: Node<'_>, text: &str, output: &mut Vec<Declaration>) {
    if let Some((kind, name)) = scope(node, text) {
        let mut declaration = Declaration::at(node, kind, name);
        if kind == "function" {
            declaration.name_span = declarator_span(node, text, &declaration.name);
            if node.kind() == "function_declarator" {
                declaration.declaration_span = prototype_container(node).and_then(|parent| {
                    let mut cursor = parent.walk();
                    (parent
                        .children_by_field_name("declarator", &mut cursor)
                        .count()
                        == 1)
                        .then(|| parent.byte_range())
                });
            }
        }
        output.push(declaration);
    } else if node.kind() == "type_definition" {
        let mut cursor = node.walk();
        for declarator in node.children_by_field_name("declarator", &mut cursor) {
            if let Some(name) = declarator_name(declarator, text) {
                output.push(Declaration::at(node, "typedef", name));
            }
        }
    } else if node.kind() == "alias_declaration"
        && let Some(name) = node
            .child_by_field_name("name")
            .and_then(|name| spelling(name, text))
    {
        output.push(Declaration::at(node, "alias", name));
    }
}

fn declarator_span(mut node: Node<'_>, text: &str, name: &str) -> Option<std::ops::Range<usize>> {
    loop {
        let end = node.start_byte().checked_add(name.len())?;
        if end <= node.end_byte() && text.get(node.start_byte()..end) == Some(name) {
            return Some(node.start_byte()..end);
        }
        if let Some(child) = node.child_by_field_name("declarator") {
            node = child;
        } else if matches!(
            node.kind(),
            "parenthesized_declarator" | "reference_declarator" | "attributed_declarator"
        ) {
            let mut cursor = node.walk();
            node = node
                .named_children(&mut cursor)
                .find(|child| child.kind() != "ms_call_modifier")?;
        } else {
            return None;
        }
    }
}

fn scope(node: Node<'_>, text: &str) -> Option<(&'static str, String)> {
    if node.kind() == "function_declarator" {
        let declarator = node.child_by_field_name("declarator")?;
        if !matches!(
            declarator.kind(),
            "identifier"
                | "field_identifier"
                | "type_identifier"
                | "qualified_identifier"
                | "operator_name"
                | "destructor_name"
                | "template_function"
                | "template_method"
        ) {
            return None;
        }
        prototype_container(node)?;
        return Some(("function", declarator_name(node, text)?));
    }
    let kind = match node.kind() {
        "function_definition" => "function",
        "struct_specifier" => "struct",
        "class_specifier" => "class",
        "union_specifier" => "union",
        "enum_specifier" => "enum",
        "namespace_definition" => "namespace",
        _ => return None,
    };
    node.child_by_field_name("body")?;
    let name = if kind == "function" {
        declarator_name(node.child_by_field_name("declarator")?, text)?
    } else {
        spelling(node.child_by_field_name("name")?, text)?
    };
    Some((kind, name))
}

fn prototype_container(mut node: Node<'_>) -> Option<Node<'_>> {
    for _ in 0..32 {
        node = node.parent()?;
        match node.kind() {
            "declaration" | "field_declaration" => return Some(node),
            "function_definition"
            | "parameter_declaration"
            | "type_definition"
            | "type_descriptor" => return None,
            _ => {}
        }
    }
    None
}

fn spelling(node: Node<'_>, text: &str) -> Option<String> {
    let name = text.get(node.byte_range())?;
    (!name.is_empty()).then(|| name.into())
}

fn declarator_name(mut node: Node<'_>, text: &str) -> Option<String> {
    loop {
        match node.kind() {
            "identifier" | "field_identifier" | "type_identifier" | "destructor_name"
            | "operator_name" | "template_function" | "template_method" => {
                return spelling(node, text);
            }
            "qualified_identifier" => {
                let mut name = node;
                while name.kind() == "qualified_identifier" {
                    name = name.child_by_field_name("name")?;
                }
                return if name.kind() == "operator_cast" {
                    conversion_name(name, node.start_byte(), text)
                } else {
                    spelling(node, text)
                };
            }
            "operator_cast" => return conversion_name(node, node.start_byte(), text),
            _ => {
                if let Some(child) = node.child_by_field_name("declarator") {
                    node = child;
                } else if matches!(
                    node.kind(),
                    "parenthesized_declarator" | "reference_declarator" | "attributed_declarator"
                ) {
                    let mut cursor = node.walk();
                    node = node
                        .named_children(&mut cursor)
                        .find(|child| child.kind() != "ms_call_modifier")?;
                } else {
                    return None;
                }
            }
        }
    }
}

fn conversion_name(node: Node<'_>, start: usize, text: &str) -> Option<String> {
    let mut declarator = node.child_by_field_name("declarator")?;
    loop {
        if let Some(parameters) = declarator.child_by_field_name("parameters") {
            return Some(text.get(start..parameters.start_byte())?.trim_end().into());
        }
        declarator = declarator.child_by_field_name("declarator")?;
    }
}
