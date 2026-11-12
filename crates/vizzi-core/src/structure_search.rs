//! Lexical declaration matching and ordering; no source reads or semantic resolution.
use crate::structure::Symbol;

/// Cached case-folded search text for one admitted declaration.
pub(crate) struct Name {
    pub(crate) simple: String,
    pub(crate) qualified: String,
    pub(crate) path: String,
}
impl Name {
    pub(crate) fn new(symbol: &Symbol) -> Self {
        Self {
            simple: symbol.name.to_lowercase(),
            qualified: symbol
                .parents
                .iter()
                .map(String::as_str)
                .chain([symbol.name.as_str()])
                .collect::<Vec<_>>()
                .join("::")
                .replace(['.', '#'], "::")
                .to_lowercase(),
            path: symbol.source.path.to_lowercase(),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Rank {
    Exact,
    QualifiedSuffix,
    Prefix,
    Substring,
    ScopePrefix,
    ScopeSubstring,
    Path,
}
pub(crate) struct Query {
    literal: String,
    qualified: String,
    suffix: String,
}
impl Query {
    pub(crate) fn new(input: &str) -> Self {
        let literal = input.trim().to_lowercase();
        let qualified = literal.replace(['.', '#'], "::");
        let suffix = format!("::{qualified}");
        Self {
            literal,
            qualified,
            suffix,
        }
    }
    pub(crate) fn empty(&self) -> bool {
        self.literal.is_empty()
    }
    pub(crate) fn rank(&self, name: &Name) -> Option<Rank> {
        if name.simple == self.literal || name.qualified == self.qualified {
            Some(Rank::Exact)
        } else if name.qualified.ends_with(&self.suffix) {
            Some(Rank::QualifiedSuffix)
        } else if name.simple.starts_with(&self.literal) {
            Some(Rank::Prefix)
        } else if name.simple.contains(&self.literal) {
            Some(Rank::Substring)
        } else if name.qualified.starts_with(&self.qualified)
            || name
                .qualified
                .match_indices("::")
                .any(|(i, _)| name.qualified[i + 2..].starts_with(&self.qualified))
        {
            Some(Rank::ScopePrefix)
        } else if name.qualified.contains(&self.qualified) {
            Some(Rank::ScopeSubstring)
        } else if name.path.contains(&self.literal) {
            Some(Rank::Path)
        } else {
            None
        }
    }
}
