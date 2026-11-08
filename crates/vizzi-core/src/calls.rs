//! Bounded literal call evidence, without import, type or runtime resolution.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Range};
use tree_sitter::Node;

/// Maximum retained call sites per analyzed document.
pub const CALL_LIMIT: usize = 4096;
/// Maximum literal callee spelling retained for one site.
pub const SPELLING_BYTES: usize = 256;
/// A raw invocation expression, owned by its nearest emitted enclosing declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSite {
    /// Enclosing declaration ordinal; syntactic containment, not runtime ownership.
    pub owner: Option<usize>,
    /// Terminal callee name when recognizable; dynamic expressions remain unnamed.
    pub name: Option<String>,
    /// Exact callee spelling from captured source, excluding its argument list.
    pub spelling: String,
    /// Nonempty UTF-8 source range of that spelling.
    pub span: Range<usize>,
}
/// Supported call extraction, with explicit allowance omissions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallFacts {
    /// Sites in syntax traversal order.
    pub sites: Vec<CallSite>,
    /// Calls outside count/spelling allowances; not evidence that no calls exist.
    pub omitted: usize,
}
pub(crate) fn supported(language: &str) -> bool {
    matches!(language, "rust" | "python")
}
fn name(mut node: Node<'_>, text: &str) -> Option<String> {
    for _ in 0..8 {
        if matches!(
            node.kind(),
            "identifier" | "field_identifier" | "type_identifier"
        ) {
            return text
                .get(node.byte_range())
                .filter(|s| !s.is_empty() && s.len() <= 128)
                .map(str::to_owned);
        }
        node = match node.kind() {
            "scoped_identifier" | "scoped_type_identifier" => node.child_by_field_name("name"),
            "generic_function" => node.child_by_field_name("function"),
            "field_expression" => node.child_by_field_name("field"),
            "attribute" => node.child_by_field_name("attribute"),
            _ => None,
        }?;
    }
    None
}
pub(crate) fn collect(
    language: &str,
    node: Node<'_>,
    text: &str,
    owners: &BTreeMap<usize, usize>,
    output: &mut CallFacts,
) {
    let is_call = match language {
        "rust" => node.kind() == "call_expression",
        "python" => node.kind() == "call",
        _ => false,
    };
    if !is_call {
        return;
    }
    let Some(callee) = node.child_by_field_name("function") else {
        output.omitted += 1;
        return;
    };
    let Some(spelling) = text
        .get(callee.byte_range())
        .filter(|s| !s.is_empty() && s.len() <= SPELLING_BYTES)
    else {
        output.omitted += 1;
        return;
    };
    if output.sites.len() == CALL_LIMIT {
        output.omitted += 1;
        return;
    }
    let mut parent = node.parent();
    let mut owner = None;
    for _ in 0..256 {
        let Some(current) = parent else {
            break;
        };
        if let Some(index) = owners.get(&current.id()) {
            owner = Some(*index);
            break;
        }
        parent = current.parent();
    }
    output.sites.push(CallSite {
        owner,
        name: name(callee, text),
        spelling: spelling.into(),
        span: callee.byte_range(),
    });
}

impl CallFacts {
    pub(crate) fn validate(
        &self,
        text: Option<&str>,
        facts: &[crate::model::Fact],
        coverage: &crate::model::Coverage,
    ) -> Result<(), crate::model::ModelError> {
        use crate::model::{Coverage, ModelError};
        if !matches!(coverage, Coverage::Syntax | Coverage::Partial)
            || self.sites.len() > CALL_LIMIT
        {
            return Err(ModelError(
                "call evidence requires bounded syntax coverage".into(),
            ));
        }
        for call in &self.sites {
            if call.span.start >= call.span.end
                || call.spelling.len() > SPELLING_BYTES
                || !text.is_some_and(|s| s.get(call.span.clone()) == Some(call.spelling.as_str()))
                || call
                    .name
                    .as_ref()
                    .is_some_and(|n| n.is_empty() || n.len() > 128 || !call.spelling.contains(n))
                || call.owner.is_some_and(|index| {
                    facts.get(index).is_none_or(|f| {
                        f.span.start > call.span.start || f.span.end < call.span.end
                    })
                })
            {
                return Err(ModelError(
                    "invalid captured call range, spelling or owner".into(),
                ));
            }
        }
        Ok(())
    }
}
