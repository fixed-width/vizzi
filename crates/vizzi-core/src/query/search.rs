use super::{MapFilter, ViewQuery};
use crate::{
    model::{Analysis, Document, DocumentVersion, EntityId, FileId},
    structure_search::{Query, Rank},
};
use regex::{Regex, RegexBuilder};
use serde::Serialize;
use std::{iter, sync::Arc};

const PAGE_SIZE: usize = 24;

/// One page of ranked source destinations.
#[derive(Debug, Serialize)]
pub struct MatchPage {
    /// Total file and declaration matches.
    pub total: usize,
    /// Offset of this page in the ranked sequence.
    pub offset: usize,
    /// Page size, including when the last page is short.
    pub size: usize,
    /// Destinations in this page.
    pub items: Vec<Match>,
    /// Pattern compilation error, when the search cannot run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A captured file or lexical declaration.
#[derive(Debug, Serialize)]
pub struct Match {
    /// File name or qualified declaration name.
    pub name: String,
    /// Declaration kind or `file`.
    pub kind: String,
    /// Workspace-relative file path.
    pub path: String,
    /// Start line in the captured source.
    pub line: usize,
    /// Captured file identity.
    pub file: FileId,
    /// Captured document version.
    pub version: DocumentVersion,
    /// Exact declaration, when the match is a declaration.
    pub entity: Option<EntityId>,
}

struct Hit<'a> {
    rank: Rank,
    doc: &'a Document,
    name: &'a str,
    declaration: Option<(&'a Analysis, usize)>,
}

impl Hit<'_> {
    fn project(&self) -> Match {
        let (name, kind, line, entity) = self.declaration.map_or_else(
            || (self.name.into(), "file".into(), 1, None),
            |(analysis, index)| {
                let fact = &analysis.facts[index];
                (
                    qualified_name(analysis, index),
                    fact.kind.clone(),
                    self.doc.lines(&fact.span).start,
                    Some(self.doc.entity_id(analysis, fact)),
                )
            },
        );
        Match {
            name,
            kind,
            line,
            entity,
            path: self.doc.path().into(),
            file: self.doc.id().clone(),
            version: self.doc.version().clone(),
        }
    }
}

fn qualified_name(analysis: &Analysis, index: usize) -> String {
    let mut names = iter::successors(Some(index), |&index| analysis.facts[index].parent)
        .map(|index| analysis.facts[index].name.as_str())
        .collect::<Vec<_>>();
    names.reverse();
    names.join("::")
}

enum Matcher {
    Plain(Query),
    Pattern(Regex),
}

impl Matcher {
    fn new(query: &ViewQuery) -> Result<Self, regex::Error> {
        if query.search_regex {
            RegexBuilder::new(&query.filter)
                .case_insensitive(!query.search_case_sensitive)
                .build()
                .map(Self::Pattern)
        } else {
            Ok(Self::Plain(Query::with_case(
                &query.filter,
                query.search_case_sensitive,
            )))
        }
    }

    fn rank_file(&mut self, path: &str) -> Option<Rank> {
        match self {
            Self::Plain(query) => query.rank_file(path),
            Self::Pattern(regex) => {
                let filename = path.rsplit('/').next().unwrap_or(path);
                if regex.is_match(filename) {
                    Some(Rank::Substring)
                } else {
                    regex.is_match(path).then_some(Rank::Path)
                }
            }
        }
    }

    fn rank_declaration(&mut self, analysis: &Analysis, index: usize) -> Option<Rank> {
        let fact = &analysis.facts[index];
        match self {
            Self::Plain(query) => {
                let qualified = query
                    .uses_qualification()
                    .then(|| qualified_name(analysis, index));
                query.rank_declaration(&fact.name, qualified.as_deref())
            }
            Self::Pattern(regex) => {
                if regex.is_match(&fact.name) {
                    Some(Rank::Substring)
                } else if fact.parent.is_some() && regex.is_match(&qualified_name(analysis, index))
                {
                    Some(Rank::ScopeSubstring)
                } else {
                    None
                }
            }
        }
    }
}

pub(super) fn project<'a>(
    docs: &'a [Arc<Document>],
    filter: &mut MapFilter<'_>,
    query: &ViewQuery,
) -> (MatchPage, Vec<&'a Document>) {
    let mut matcher = match Matcher::new(query) {
        Ok(matcher) => matcher,
        Err(error) => {
            for doc in docs {
                filter.collect_type(doc);
            }
            let message = error.to_string();
            let detail = message.lines().last().unwrap_or(&message);
            return (
                MatchPage {
                    total: 0,
                    offset: 0,
                    size: PAGE_SIZE,
                    items: vec![],
                    error: Some(detail.strip_prefix("error: ").unwrap_or(detail).into()),
                },
                vec![],
            );
        }
    };
    let mut hits = Vec::new();
    let mut files = Vec::new();
    for doc in docs
        .iter()
        .map(Arc::as_ref)
        .filter(|doc| filter.collect_type(doc))
    {
        let previous = hits.len();
        if let Some(rank) = matcher.rank_file(doc.path()) {
            hits.push(Hit {
                rank,
                doc,
                name: doc.path().rsplit('/').next().unwrap_or(doc.path()),
                declaration: None,
            });
        }
        for analysis in doc.analyses() {
            for (index, fact) in analysis.facts.iter().enumerate() {
                if let Some(rank) = matcher.rank_declaration(analysis, index) {
                    hits.push(Hit {
                        rank,
                        doc,
                        name: &fact.name,
                        declaration: Some((analysis, index)),
                    });
                }
            }
        }
        if hits.len() > previous {
            files.push(doc);
        }
    }
    hits.sort_by_key(|hit| (hit.rank, hit.name, hit.doc.path()));
    let total = hits.len();
    let offset = query.results_offset.min(total.saturating_sub(1)) / PAGE_SIZE * PAGE_SIZE;
    let items = hits
        .iter()
        .skip(offset)
        .take(PAGE_SIZE)
        .map(Hit::project)
        .collect();
    (
        MatchPage {
            total,
            offset,
            size: PAGE_SIZE,
            items,
            error: None,
        },
        files,
    )
}
