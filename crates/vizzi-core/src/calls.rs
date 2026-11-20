//! Bounded call evidence and source-local bindings; no general import/type/runtime resolution.
use crate::syntax::ScopeFacts;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use tree_sitter::Node;

/// Maximum retained call sites per analyzed document.
pub const CALL_LIMIT: usize = 4096;
/// Maximum literal callee spelling retained for one site.
pub const SPELLING_BYTES: usize = 256;
/// Language-specific evidence for choosing a source destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetBasis {
    /// Same shorthand Rust `self` receiver and one inherent method in its lexical scope.
    RustSelfReceiver,
    /// Definition locations supplied by a host language server, never raw syntax.
    LanguageServerDefinition,
}
/// A language-supported destination in the same captured analysis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallTarget {
    /// Destination declaration ordinal, independent of display names.
    pub declaration: usize,
    /// Evidence supporting this destination.
    pub basis: TargetBasis,
}
/// A raw invocation expression, attributed to the closest enclosing declaration that owns calls.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSite {
    /// Optional language-supported destination; absent for legacy or ambiguous evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<CallTarget>,
    /// Enclosing declaration ordinal; syntactic containment, not runtime ownership.
    pub owner: Option<usize>,
    /// Terminal callee name when recognizable; dynamic expressions remain unnamed.
    pub name: Option<String>,
    /// Exact terminal identifier; absent in legacy captures and dynamic callees.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_span: Option<Range<usize>>,
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
pub(crate) struct CallExpression {
    pub kind: &'static str,
    pub callee_field: &'static str,
    pub unwrap_callee: Option<fn(Node<'_>) -> Option<Node<'_>>>,
    pub name_kinds: &'static [&'static str],
    pub name_fields: &'static [(&'static str, &'static str)],
}

impl CallExpression {
    fn name(&self, mut node: Node<'_>, text: &str) -> Option<(String, Range<usize>)> {
        for _ in 0..8 {
            let kind = node.kind();
            if self.name_kinds.contains(&kind) {
                return text
                    .get(node.byte_range())
                    .filter(|s| !s.is_empty() && s.len() <= 128)
                    .map(|name| (name.to_owned(), node.byte_range()));
            }
            let (_, field) = self
                .name_fields
                .iter()
                .find(|(parent, _)| *parent == kind)?;
            node = node.child_by_field_name(field)?;
        }
        None
    }

    pub(crate) fn collect(
        &self,
        node: Node<'_>,
        text: &str,
        owners: &ScopeFacts,
        output: &mut CallFacts,
    ) {
        if node.kind() != self.kind {
            return;
        }
        let Some(callee) = node.child_by_field_name(self.callee_field) else {
            output.omitted += 1;
            return;
        };
        let name = self
            .unwrap_callee
            .map_or(Some(callee), |unwrap| unwrap(callee))
            .and_then(|value| self.name(value, text));
        record(node, callee.byte_range(), name, text, owners, output);
    }
}
pub(crate) fn record(
    node: Node<'_>,
    span: Range<usize>,
    name: Option<(String, Range<usize>)>,
    text: &str,
    owners: &ScopeFacts,
    output: &mut CallFacts,
) {
    let Some(spelling) = text
        .get(span.clone())
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
        if let Some(fact) = owners.get(&current.id()).filter(|fact| fact.owns_calls) {
            owner = Some(fact.ordinal);
            break;
        }
        parent = current.parent();
    }
    let (name, name_span) = match name {
        Some((name, span)) => (Some(name), Some(span)),
        None => (None, None),
    };
    output.sites.push(CallSite {
        target: None,
        owner,
        name,
        name_span,
        spelling: spelling.into(),
        span,
    });
}

impl CallFacts {
    pub(crate) fn validate(
        &self,
        text: Option<&str>,
        facts: &[crate::model::Fact],
        coverage: &crate::model::Coverage,
        language: &str,
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
            if call.name_span.as_ref().is_some_and(|span| {
                span.start >= span.end
                    || span.start < call.span.start
                    || span.end > call.span.end
                    || call.name.is_none()
                    || text.and_then(|s| s.get(span.clone())) != call.name.as_deref()
            }) {
                return Err(ModelError("invalid captured callee identifier span".into()));
            }
            if let Some(target) = &call.target
                && (target.basis != TargetBasis::RustSelfReceiver
                    || language != "rust"
                    || *coverage != Coverage::Syntax
                    || call.owner.is_none()
                    || facts.get(target.declaration).is_none_or(|fact| {
                        fact.kind != "function" || call.name.as_deref() != Some(fact.name.as_str())
                    }))
            {
                return Err(ModelError("invalid source-local call destination".into()));
            }
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
