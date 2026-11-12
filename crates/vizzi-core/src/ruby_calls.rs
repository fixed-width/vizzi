//! Ruby written calls with bounded, position-aware lexical local-variable tracking.
use crate::{
    calls::{self, CallFacts},
    syntax::Checkpoints,
};
use std::collections::BTreeMap;
use tree_sitter::Node;
const LOCALS: usize = 4096;

fn scope(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "program"
            | "method"
            | "singleton_method"
            | "class"
            | "module"
            | "singleton_class"
            | "block"
            | "do_block"
            | "lambda"
    )
}
fn field(node: Node<'_>, name: &str, child: Node<'_>) -> bool {
    node.child_by_field_name(name)
        .is_some_and(|n| n.id() == child.id())
}
fn walk<'tree>(
    root: Node<'tree>,
    cancelled: &dyn Fn() -> bool,
    mut visit: impl FnMut(Node<'tree>),
) -> bool {
    let mut cursor = root.walk();
    let mut checks = Checkpoints::new(cancelled);
    loop {
        if checks.poll() {
            return false;
        }
        visit(cursor.node());
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return !cancelled();
            }
        }
    }
}
fn binding(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    if matches!(
        parent.kind(),
        "method_parameters"
            | "block_parameters"
            | "lambda_parameters"
            | "block_parameter"
            | "destructured_parameter"
            | "splat_parameter"
            | "hash_splat_parameter"
            | "exception_variable"
            | "left_assignment_list"
            | "destructured_left_assignment"
            | "rest_assignment"
    ) {
        return true;
    }
    if matches!(
        parent.kind(),
        "optional_parameter" | "keyword_parameter" | "as_pattern"
    ) && field(parent, "name", node)
    {
        return true;
    }
    if matches!(parent.kind(), "assignment" | "operator_assignment") && field(parent, "left", node)
    {
        return true;
    }
    if parent.kind() == "for" && field(parent, "pattern", node) {
        return true;
    }
    let mut current = node;
    for _ in 0..256 {
        let Some(parent) = current.parent() else {
            break;
        };
        if matches!(
            parent.kind(),
            "variable_reference_pattern" | "expression_reference_pattern" | "call"
        ) || scope(parent)
        {
            break;
        }
        if matches!(
            parent.kind(),
            "in_clause" | "match_pattern" | "test_pattern"
        ) {
            return field(parent, "pattern", current);
        }
        current = parent;
    }
    false
}
fn declaration_name(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        (matches!(parent.kind(), "method" | "singleton_method") && field(parent, "name", node))
            || (parent.kind() == "singleton_method" && field(parent, "object", node))
            || (parent.kind() == "call" && field(parent, "method", node))
            || matches!(
                parent.kind(),
                "alias" | "setter" | "undef" | "variable_reference_pattern"
            )
    })
}
#[derive(Default)]
struct Bindings {
    scopes: BTreeMap<usize, BTreeMap<String, usize>>,
    count: usize,
    limited: bool,
}
impl Bindings {
    fn add(&mut self, node: Node<'_>, name: &str) {
        if self.limited {
            return;
        }
        let mut current = node.parent();
        for _ in 0..256 {
            let Some(parent) = current else { return };
            if scope(parent) {
                let names = self.scopes.entry(parent.id()).or_default();
                if let Some(position) = names.get_mut(name) {
                    *position = (*position).min(node.start_byte());
                    return;
                }
                if self.count == LOCALS {
                    self.limited = true;
                    return;
                }
                names.insert(name.into(), node.start_byte());
                self.count += 1;
                return;
            }
            current = parent.parent();
        }
        self.limited = true;
    }
    fn local(&self, node: Node<'_>, name: &str) -> bool {
        let mut current = node.parent();
        for _ in 0..256 {
            let Some(parent) = current else { return false };
            if scope(parent) {
                if self
                    .scopes
                    .get(&parent.id())
                    .and_then(|names| names.get(name))
                    .is_some_and(|at| *at <= node.start_byte())
                {
                    return true;
                }
                if matches!(parent.kind(), "block" | "do_block" | "lambda") {
                    if parent.child_by_field_name("parameters").is_none()
                        && (name == "it"
                            || matches!(
                                name,
                                "_1" | "_2" | "_3" | "_4" | "_5" | "_6" | "_7" | "_8" | "_9"
                            ))
                    {
                        return true;
                    }
                } else {
                    return false;
                }
            }
            current = parent.parent();
        }
        true
    }
}
pub(crate) fn collect(
    root: Node<'_>,
    text: &str,
    owners: &BTreeMap<usize, usize>,
    cancelled: &dyn Fn() -> bool,
) -> Option<CallFacts> {
    let mut bindings = Bindings::default();
    if !walk(root, cancelled, |node| {
        if node.kind() == "identifier" && binding(node) {
            if let Some(name) = text.get(node.byte_range()).filter(|s| s.len() <= 128) {
                bindings.add(node, name);
            }
        } else if node.kind() == "hash_key_symbol"
            && node.parent().is_some_and(|parent| {
                parent.kind() == "keyword_pattern" && parent.child_by_field_name("value").is_none()
            })
            && let Some(name) = text.get(node.byte_range())
        {
            bindings.add(node, name.trim_end_matches(':'));
        }
    }) {
        return None;
    }
    let mut output = CallFacts::default();
    if !walk(root, cancelled, |node| {
        if node.kind() == "call" {
            let terminal = node.child_by_field_name("method");
            let Some(end) = terminal.or_else(|| node.child_by_field_name("operator")) else {
                output.omitted += 1;
                return;
            };
            let name = terminal.and_then(|name| {
                text.get(name.byte_range())
                    .filter(|s| !s.is_empty() && s.len() <= 128)
                    .map(|text| (text.into(), name.byte_range()))
            });
            calls::record(
                node,
                node.start_byte()..end.end_byte(),
                name,
                text,
                owners,
                &mut output,
            );
        } else if node.kind() == "identifier" && !binding(node) && !declaration_name(node) {
            let Some(name) = text.get(node.byte_range()).filter(|s| s.len() <= 128) else {
                output.omitted += 1;
                return;
            };
            if bindings.local(node, name) {
                return;
            }
            if bindings.limited {
                output.omitted += 1;
                return;
            }
            calls::record(
                node,
                node.byte_range(),
                Some((name.into(), node.byte_range())),
                text,
                owners,
                &mut output,
            );
        }
    }) {
        return None;
    }
    Some(output)
}
