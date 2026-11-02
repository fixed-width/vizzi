//! Raw C/C++ definitions and type aliases, without preprocessing or name resolution.

use crate::{
    analysis::{Analyzer, LanguageAdapter},
    model::{Context, ContextId, ModelError},
    syntax::{Declaration, SyntaxAnalyzer},
};
use tree_sitter::Node;

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
        Ok(Box::new(SyntaxAnalyzer::new(
            tree_sitter_c::LANGUAGE.into(),
            self.context(),
            collect,
            scope,
        )?))
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
        Ok(Box::new(SyntaxAnalyzer::new(
            tree_sitter_cpp::LANGUAGE.into(),
            self.context(),
            collect,
            scope,
        )?))
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
            include_bytes!("analysis.rs"),
            include_bytes!("model.rs"),
            include_bytes!("../../../Cargo.lock"),
        ]),
    }
}

fn collect(node: Node<'_>, text: &str, output: &mut Vec<Declaration>) {
    if let Some((kind, name)) = scope(node, text) {
        output.push(Declaration::at(node, kind, name));
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

fn scope(node: Node<'_>, text: &str) -> Option<(&'static str, String)> {
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
