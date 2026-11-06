//! Source-role hints and raw lexical scope, independent of the question library.
use serde::Serialize;
use std::ops::Range;
use vizzi_core::model::Analysis;

/// Source role used to expose context and guide evidence selection, not semantic authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    /// Ordinary source outside an identified test scope.
    Code,
    /// Test path or raw lexical test scope.
    Test,
    /// Current descriptive or task documentation.
    Guide,
    /// Contributor/design/behavior contract, classified by path convention.
    Policy,
    /// Manifest or configuration text.
    Configuration,
    /// Historical measurement, experiment or progress report.
    Historical,
    /// An excerpt crosses identified source-role boundaries.
    Mixed,
}
/// Bounded lexical declaration context; this is not a resolved call relationship.
#[derive(Clone, Debug, Serialize)]
pub struct SourceScope {
    /// Declaration name.
    pub name: String,
    /// Raw declaration kind.
    pub kind: String,
    /// Enclosing emitted scope names, outermost first.
    pub parents: Vec<String>,
}
pub(crate) struct Scope {
    pub range: Range<usize>,
    pub label: SourceScope,
    pub test: bool,
}
pub(crate) struct Metadata {
    pub role: SourceRole,
    pub basis: &'static str,
    pub scopes: Vec<Scope>,
}
fn bounded(text: &str) -> String {
    let end = text
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .take_while(|&i| i <= 128)
        .last()
        .unwrap_or(0);
    text[..end].into()
}
fn path_role(path: &str) -> SourceRole {
    let lower = path.to_lowercase();
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    if lower.split('/').any(|p| p == "tests" || p == "test")
        || name.starts_with("test_")
        || name.ends_with("_test.rs")
        || name.ends_with(".test.js")
    {
        return SourceRole::Test;
    }
    if name.ends_with(".md") {
        if lower.starts_with("experiments/")
            || name.contains("results")
            || name.contains("history")
            || name.contains("worklog")
        {
            return SourceRole::Historical;
        }
        if [
            "agents.md",
            "contributing.md",
            "design.md",
            "product.md",
            "tech.md",
        ]
        .contains(&name)
            || lower.contains("/prototype/")
        {
            return SourceRole::Policy;
        }
        return SourceRole::Guide;
    }
    if ["toml", "yaml", "yml", "json"]
        .iter()
        .any(|ext| name.ends_with(&format!(".{ext}")))
    {
        return SourceRole::Configuration;
    }
    SourceRole::Code
}
fn test_attribute(text: &str, start: usize) -> bool {
    let mut before = start.saturating_sub(256);
    while !text.is_char_boundary(before) {
        before += 1;
    }
    for line in text[before..start].lines().rev().take(8) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("///") {
            continue;
        }
        if !line.starts_with("#[") {
            break;
        }
        if line.contains("cfg(test)") || line.contains("[test]") || line.contains("::test") {
            return true;
        }
    }
    false
}
impl Metadata {
    pub fn new(path: &str, text: &str, analysis: Option<&Analysis>) -> Self {
        let role = path_role(path);
        let mut scopes: Vec<Scope> = vec![];
        if let Some(analysis) = analysis {
            for fact in analysis.facts.iter().take(4096) {
                let parent = fact.parent.and_then(|n| scopes.get(n));
                let mut parents = parent.map(|p| p.label.parents.clone()).unwrap_or_default();
                if let Some(parent) = parent
                    && parents.len() < 16
                {
                    parents.push(parent.label.name.clone());
                }
                let test = role == SourceRole::Test
                    || parent.is_some_and(|p| p.test)
                    || test_attribute(text, fact.span.start);
                scopes.push(Scope {
                    range: fact.span.clone(),
                    label: SourceScope {
                        name: bounded(&fact.name),
                        kind: bounded(&fact.kind),
                        parents,
                    },
                    test,
                });
            }
        }
        let basis = if role == SourceRole::Code && analysis.is_some() {
            "raw_lexical_scope"
        } else {
            "path_convention"
        };
        Self {
            role,
            basis,
            scopes,
        }
    }
    pub fn scope(&self, offset: usize) -> Option<&Scope> {
        self.scopes
            .iter()
            .filter(|s| s.range.contains(&offset))
            .min_by_key(|s| s.range.len())
    }
    pub fn role_for(&self, range: &Range<usize>) -> SourceRole {
        if self.role != SourceRole::Code {
            return self.role;
        }
        let mut intersects = false;
        for scope in self.scopes.iter().filter(|s| s.test) {
            if scope.range.start <= range.start && range.end <= scope.range.end {
                return SourceRole::Test;
            }
            intersects |= scope.range.start < range.end && range.start < scope.range.end;
        }
        if intersects {
            SourceRole::Mixed
        } else {
            SourceRole::Code
        }
    }
}
