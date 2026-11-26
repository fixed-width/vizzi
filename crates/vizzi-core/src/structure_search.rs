//! Lexical declaration matching and ordering; no source reads or semantic resolution.
use crate::structure::{NAME_BYTES, Symbol};
use std::{borrow::Cow, collections::BTreeMap};
use tree_sitter::Node;

#[derive(Default)]
pub(crate) struct RustImplNames<'a> {
    parser: Option<tree_sitter::Parser>,
    names: BTreeMap<&'a str, Option<Vec<String>>>,
}

impl<'a> RustImplNames<'a> {
    pub(crate) fn without_arguments(&mut self, label: &'a str) -> &[String] {
        if !label.contains('<') {
            return &[];
        }
        self.names
            .entry(label)
            .or_insert_with(|| {
                if self.parser.is_none() {
                    let mut parser = tree_sitter::Parser::new();
                    parser
                        .set_language(&tree_sitter_rust::LANGUAGE.into())
                        .ok()?;
                    self.parser = Some(parser);
                }
                rust_impl_aliases(self.parser.as_mut()?, label)
            })
            .as_deref()
            .unwrap_or_default()
    }
}

fn rust_impl_aliases(parser: &mut tree_sitter::Parser, label: &str) -> Option<Vec<String>> {
    let text = format!("impl {label} {{}}");
    let tree = parser.parse(&text, None)?;
    let implementation = tree.root_node().named_child(0)?;
    if implementation.has_error() {
        return None;
    }
    let target = implementation.child_by_field_name("type")?;
    let short_target = without_type_arguments(target, &text)?;
    let trait_alias = implementation
        .child_by_field_name("trait")
        .and_then(|node| text.get(node.byte_range()))
        .map(|trait_name| format!("{trait_name} for {short_target}"));
    Some(
        std::iter::once(short_target)
            .chain(trait_alias)
            .filter(|alias| alias.len() <= NAME_BYTES)
            .collect(),
    )
}

fn without_type_arguments(target: Node<'_>, text: &str) -> Option<String> {
    let mut cursor = target.walk();
    let mut kept_from = target.start_byte();
    let mut shortened = String::new();
    loop {
        let node = cursor.node();
        if node.kind() == "type_arguments" {
            // Skip argument subtrees and keep the enclosing target syntax.
            if let Some(parent) = node
                .parent()
                .filter(|parent| parent.kind() == "generic_type")
            {
                let base = parent.child_by_field_name("type")?;
                shortened.push_str(text.get(kept_from..base.end_byte())?);
                kept_from = node.end_byte();
            }
        } else if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                if kept_from == target.start_byte() {
                    return None;
                }
                shortened.push_str(text.get(kept_from..target.end_byte())?);
                return Some(shortened);
            }
        }
    }
}

fn qualified<'a>(parents: impl Iterator<Item = &'a str>, name: &'a str) -> String {
    parents
        .chain([name])
        .collect::<Vec<_>>()
        .join("::")
        .replace(['.', '#'], "::")
        .to_lowercase()
}

/// Cached case-folded search text for one admitted declaration.
pub(crate) struct Name {
    pub(crate) simple: String,
    pub(crate) qualified: String,
    impl_aliases: Box<[String]>,
    pub(crate) path: String,
}
impl Name {
    pub(crate) fn new(symbol: &Symbol, impl_aliases: &[String]) -> Self {
        let outer = symbol
            .parents
            .split_last()
            .map_or(&[][..], |(_, outer)| outer);
        Self {
            simple: symbol.name.to_lowercase(),
            qualified: qualified(symbol.parents.iter().map(String::as_str), &symbol.name),
            impl_aliases: impl_aliases
                .iter()
                .map(|alias| {
                    qualified(
                        outer.iter().map(String::as_str).chain([alias.as_str()]),
                        &symbol.name,
                    )
                })
                .collect(),
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
    case_sensitive: bool,
}
impl Query {
    pub(crate) fn new(input: &str) -> Self {
        Self::with_case(input, false)
    }

    pub(crate) fn with_case(input: &str, case_sensitive: bool) -> Self {
        let literal = case_text(input.trim(), case_sensitive).into_owned();
        let qualified = literal.replace(['.', '#'], "::");
        let suffix = format!("::{qualified}");
        Self {
            literal,
            qualified,
            suffix,
            case_sensitive,
        }
    }
    pub(crate) fn empty(&self) -> bool {
        self.literal.is_empty()
    }
    pub(crate) fn rank(&self, name: &Name) -> Option<Rank> {
        let qualified_names = std::iter::once(name.qualified.as_str())
            .chain(name.impl_aliases.iter().map(String::as_str));
        self.rank_names(&name.simple, qualified_names)
            .or_else(|| name.path.contains(&self.literal).then_some(Rank::Path))
    }

    pub(crate) fn uses_qualification(&self) -> bool {
        self.qualified.contains("::")
    }

    pub(crate) fn rank_declaration(
        &self,
        name: &str,
        qualified_name: Option<&str>,
    ) -> Option<Rank> {
        let qualified_name = qualified_name.map(|name| name.replace(['.', '#'], "::"));
        let qualified_name = qualified_name
            .as_deref()
            .map(|name| case_text(name, self.case_sensitive));
        self.rank_names(
            &case_text(name, self.case_sensitive),
            qualified_name.as_deref().into_iter(),
        )
    }

    pub(crate) fn rank_file(&self, path: &str) -> Option<Rank> {
        let path = case_text(path, self.case_sensitive);
        let filename = path.rsplit('/').next().unwrap_or(&path);
        if path == self.literal {
            Some(Rank::Exact)
        } else {
            self.rank_names(filename, std::iter::empty())
                .or_else(|| path.contains(&self.literal).then_some(Rank::Path))
        }
    }

    fn rank_names<'a>(
        &self,
        simple: &str,
        mut qualified_names: impl Iterator<Item = &'a str> + Clone,
    ) -> Option<Rank> {
        if simple == self.literal || qualified_names.clone().any(|text| text == self.qualified) {
            Some(Rank::Exact)
        } else if qualified_names
            .clone()
            .any(|text| text.ends_with(&self.suffix))
        {
            Some(Rank::QualifiedSuffix)
        } else if simple.starts_with(&self.literal) {
            Some(Rank::Prefix)
        } else if simple.contains(&self.literal) {
            Some(Rank::Substring)
        } else if qualified_names.clone().any(|text| {
            text.starts_with(&self.qualified)
                || text
                    .match_indices("::")
                    .any(|(i, _)| text[i + 2..].starts_with(&self.qualified))
        }) {
            Some(Rank::ScopePrefix)
        } else if qualified_names.any(|text| text.contains(&self.qualified)) {
            Some(Rank::ScopeSubstring)
        } else {
            None
        }
    }
}

fn case_text(text: &str, case_sensitive: bool) -> Cow<'_, str> {
    if case_sensitive {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(text.to_lowercase())
    }
}
