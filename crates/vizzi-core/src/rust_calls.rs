//! Conservative source-local Rust `self` bindings; no import or general type inference.
use crate::{
    calls::{CallTarget, TargetBasis},
    model::{Analysis, Coverage},
    syntax::Checkpoints,
};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Receiver {
    Owned,
    Shared,
    Mutable,
}
struct Method<'a> {
    scope: usize,
    ty: &'a str,
    name: &'a str,
    receiver: Option<Receiver>,
    inherent: bool,
    eligible: bool,
}
fn attributed(mut node: Node<'_>) -> bool {
    loop {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "inner_attribute_item" => return true,
                "line_comment" | "block_comment" => continue,
                _ => break,
            }
        }
        let mut previous = node.prev_named_sibling();
        while let Some(sibling) = previous {
            match sibling.kind() {
                "attribute_item" | "inner_attribute_item" => return true,
                "line_comment" | "block_comment" => previous = sibling.prev_named_sibling(),
                _ => break,
            }
        }
        let Some(parent) = node.parent() else {
            return false;
        };
        node = parent;
    }
}
fn receiver(function: Node<'_>) -> Option<Receiver> {
    let parameters = function.child_by_field_name("parameters")?;
    let mut cursor = parameters.walk();
    let parameter = parameters
        .named_children(&mut cursor)
        .find(|n| n.kind() == "self_parameter")?;
    let mut cursor = parameter.walk();
    let mut reference = false;
    let mut mutable = false;
    for child in parameter.children(&mut cursor) {
        reference |= child.kind() == "&";
        mutable |= child.kind() == "mutable_specifier";
    }
    Some(if !reference {
        Receiver::Owned
    } else if mutable {
        Receiver::Mutable
    } else {
        Receiver::Shared
    })
}
fn method<'a>(function: Node<'_>, text: &'a str) -> Option<Method<'a>> {
    let implementation = function.parent()?.parent()?;
    if implementation.kind() != "impl_item" {
        return None;
    }
    let ty = implementation.child_by_field_name("type")?;
    if ty.kind() != "type_identifier"
        || implementation
            .child_by_field_name("type_parameters")
            .is_some()
    {
        return None;
    }
    Some(Method {
        scope: implementation.parent()?.id(),
        ty: text.get(ty.byte_range())?,
        name: text.get(function.child_by_field_name("name")?.byte_range())?,
        receiver: receiver(function),
        inherent: implementation.child_by_field_name("trait").is_none(),
        eligible: !attributed(function),
    })
}
fn direct_self(mut callee: Node<'_>) -> bool {
    if callee.kind() == "generic_function" {
        let Some(function) = callee.child_by_field_name("function") else {
            return false;
        };
        callee = function;
    }
    callee.kind() == "field_expression"
        && callee
            .child_by_field_name("value")
            .is_some_and(|value| value.kind() == "self")
}

pub(crate) fn bind(
    root: Node<'_>,
    text: &str,
    analysis: &mut Analysis,
    declarations: &BTreeMap<usize, usize>,
    cancelled: &dyn Fn() -> bool,
) -> bool {
    if analysis.coverage != Coverage::Syntax {
        return true;
    }
    let Some(calls) = &mut analysis.calls else {
        return true;
    };
    let captured = calls
        .sites
        .iter()
        .map(|c| (c.span.start, c.span.end))
        .collect::<BTreeSet<_>>();
    let mut direct = BTreeSet::new();
    let mut methods = BTreeMap::new();
    let mut targets = BTreeMap::<_, Vec<usize>>::new();
    let mut cursor = root.walk();
    let mut checkpoints = Checkpoints::new(cancelled);
    loop {
        if checkpoints.poll() {
            return false;
        }
        let node = cursor.node();
        if node.kind() == "function_item"
            && let Some(ordinal) = declarations.get(&node.id())
            && let Some(method) = method(node, text)
        {
            if method.inherent {
                targets
                    .entry((method.scope, method.ty, method.name))
                    .or_default()
                    .push(*ordinal);
            }
            methods.insert(*ordinal, method);
        }
        if node.kind() == "call_expression"
            && let Some(callee) = node.child_by_field_name("function")
        {
            let span = (callee.start_byte(), callee.end_byte());
            if captured.contains(&span) && direct_self(callee) {
                direct.insert(span);
            }
        }
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                break;
            }
        }
        if cursor.node() == root {
            break;
        }
    }
    for call in &mut calls.sites {
        if checkpoints.poll() {
            return false;
        }
        let Some(owner) = call.owner.and_then(|ordinal| methods.get(&ordinal)) else {
            continue;
        };
        if !owner.eligible
            || owner.receiver.is_none()
            || !direct.contains(&(call.span.start, call.span.end))
        {
            continue;
        }
        let Some(name) = call.name.as_deref() else {
            continue;
        };
        let Some(destinations) = targets.get(&(owner.scope, owner.ty, name)) else {
            continue;
        };
        let [ordinal] = destinations.as_slice() else {
            continue;
        };
        let target = &methods[ordinal];
        if target.eligible && owner.receiver == target.receiver {
            call.target = Some(CallTarget {
                declaration: *ordinal,
                basis: TargetBasis::RustSelfReceiver,
            });
        }
    }
    true
}
