use super::{MapFilter, View, ViewQuery, browse};
use crate::{
    model::{
        Analysis, BULK_SOURCE_LIMIT, Document, DocumentVersion, EntityId, FileId, Snapshot,
        SnapshotId, SourceReadError,
    },
    structure_search::{Query, Rank},
};
use regex::{Regex, RegexBuilder};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    iter,
    sync::{Arc, Mutex, MutexGuard},
};

const PAGE_SIZE: usize = 24;
const RETAINED_BYTES: usize = 8 * 1024 * 1024;

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
    name: &'a str,
    destination: Destination,
}

struct Destination {
    document: usize,
    declaration: Option<(usize, usize)>,
}

impl Destination {
    fn project(&self, docs: &[Arc<Document>]) -> Match {
        let doc = &docs[self.document];
        let (name, kind, line, entity) = self.declaration.map_or_else(
            || (basename(doc.path()).into(), "file".into(), 1, None),
            |(analysis_index, index)| {
                let analysis = &doc.analyses()[analysis_index];
                let fact = &analysis.facts[index];
                (
                    qualified_name(analysis, index),
                    fact.kind.clone(),
                    doc.lines(&fact.span).start,
                    Some(doc.entity_id(analysis, fact)),
                )
            },
        );
        Match {
            name,
            kind,
            line,
            entity,
            path: doc.path().into(),
            file: doc.id().clone(),
            version: doc.version().clone(),
        }
    }
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
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
                let filename = basename(path);
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

/// Reuses a bounded ranking across linked views and match pages of a pinned search.
#[derive(Default)]
pub struct SourceSearch {
    retained: Mutex<Retained>,
}

#[derive(Default)]
struct Retained {
    request: u64,
    entry: Option<Cached>,
}

struct Cached {
    last_used: u64,
    key: Key,
    ranked: Arc<Ranked>,
}

struct Key {
    snapshot: SnapshotId,
    filter: String,
    directory: String,
    types: String,
    regex: bool,
    case_sensitive: bool,
}

impl Key {
    fn matches(&self, snapshot: &Snapshot, query: &ViewQuery) -> bool {
        self.snapshot == *snapshot.id()
            && self.filter == query.search_text()
            && self.directory == query.search_scope()
            && self.types == query.types
            && self.regex == query.search_regex
            && self.case_sensitive == query.search_case_sensitive
    }
}

impl SourceSearch {
    /// Projects the overview, ranked matches and selected source from one observation.
    pub fn view(&self, snapshot: &Snapshot, query: &ViewQuery) -> Result<View, SourceReadError> {
        snapshot.project_view(query, BULK_SOURCE_LIMIT, Some(self))
    }

    /// Projects only the requested match page from the supplied observation.
    pub fn matches(&self, snapshot: &Snapshot, query: &ViewQuery) -> MatchPage {
        if query.search_text().is_empty() {
            return Ranked::default().page(snapshot.documents(), 0);
        }
        self.rank(snapshot, query)
            .page(snapshot.documents(), query.results_offset)
    }

    pub(super) fn rank(&self, snapshot: &Snapshot, query: &ViewQuery) -> Arc<Ranked> {
        let request = {
            let mut retained = self.retained();
            retained.request += 1;
            let request = retained.request;
            if let Some(cached) = retained
                .entry
                .as_mut()
                .filter(|cached| cached.key.matches(snapshot, query))
            {
                cached.last_used = request;
                return Arc::clone(&cached.ranked);
            }
            request
        };

        let ranked = Arc::new(Ranked::build(snapshot, query));
        let bytes = ranked
            .retained_bytes()
            .saturating_add(query.search_text().len())
            .saturating_add(query.search_scope().len())
            .saturating_add(query.types.len());
        if ranked.error.is_none() && bytes <= RETAINED_BYTES {
            let cached = Cached {
                last_used: request,
                key: Key {
                    snapshot: snapshot.id().clone(),
                    filter: query.search_text().into(),
                    directory: query.search_scope().into(),
                    types: query.types.clone(),
                    regex: query.search_regex,
                    case_sensitive: query.search_case_sensitive,
                },
                ranked: Arc::clone(&ranked),
            };
            let previous = {
                let mut retained = self.retained();
                if retained
                    .entry
                    .as_ref()
                    .is_some_and(|entry| entry.last_used > request)
                {
                    None
                } else {
                    retained.entry.replace(cached)
                }
            };
            drop(previous);
        }
        ranked
    }

    fn retained(&self) -> MutexGuard<'_, Retained> {
        self.retained
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[derive(Default)]
pub(super) struct Ranked {
    hits: Box<[Destination]>,
    files: Box<[usize]>,
    pub(super) file_type_counts: BTreeMap<&'static str, usize>,
    error: Option<String>,
}

impl Ranked {
    pub(super) fn build(snapshot: &Snapshot, query: &ViewQuery) -> Self {
        #[cfg(test)]
        tests::observe_build();
        let mut filter = MapFilter::for_types(&query.types);
        let docs = snapshot.documents();
        let scope = browse::exact_scope(docs, query.search_scope());
        let mut matcher = match Matcher::new(query) {
            Ok(matcher) => matcher,
            Err(error) => {
                for doc in &docs[scope] {
                    filter.collect_type(doc);
                }
                let message = error.to_string();
                let detail = message.lines().last().unwrap_or(&message);
                return Self {
                    file_type_counts: filter.file_type_counts,
                    error: Some(detail.strip_prefix("error: ").unwrap_or(detail).into()),
                    ..Self::default()
                };
            }
        };
        let mut hits = Vec::new();
        let mut files = Vec::new();
        for document in scope {
            let doc = &docs[document];
            if !filter.collect_type(doc) {
                continue;
            }
            let previous = hits.len();
            if let Some(rank) = matcher.rank_file(doc.path()) {
                hits.push(Hit {
                    rank,
                    name: basename(doc.path()),
                    destination: Destination {
                        document,
                        declaration: None,
                    },
                });
            }
            for (analysis_index, analysis) in doc.analyses().iter().enumerate() {
                for (index, fact) in analysis.facts.iter().enumerate() {
                    if let Some(rank) = matcher.rank_declaration(analysis, index) {
                        hits.push(Hit {
                            rank,
                            name: &fact.name,
                            destination: Destination {
                                document,
                                declaration: Some((analysis_index, index)),
                            },
                        });
                    }
                }
            }
            if hits.len() > previous {
                files.push(document);
            }
        }
        // Document positions are in path order, and the stable sort preserves exact ties.
        hits.sort_by_key(|hit| (hit.rank, hit.name, hit.destination.document));
        Self {
            files: files.into_boxed_slice(),
            hits: hits.into_iter().map(|hit| hit.destination).collect(),
            file_type_counts: filter.file_type_counts,
            error: None,
        }
    }

    fn retained_bytes(&self) -> usize {
        size_of_val(self.hits.as_ref()) + size_of_val(self.files.as_ref())
    }

    pub(super) fn files<'doc>(
        &self,
        docs: &'doc [Arc<Document>],
    ) -> impl ExactSizeIterator<Item = &'doc Document> + Clone {
        self.files.iter().map(|&index| docs[index].as_ref())
    }

    pub(super) fn page(&self, docs: &[Arc<Document>], offset: usize) -> MatchPage {
        let total = self.hits.len();
        let offset = offset.min(total.saturating_sub(1)) / PAGE_SIZE * PAGE_SIZE;
        let items = self.hits[offset..]
            .iter()
            .take(PAGE_SIZE)
            .map(|hit| hit.project(docs))
            .collect();
        MatchPage {
            total,
            offset,
            size: PAGE_SIZE,
            items,
            error: self.error.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{WorkspaceId, fixture_document, fixture_snapshot},
        query::{MAX_MAP_FILES, Projection},
    };
    use std::{cell::RefCell, sync::mpsc, thread, time::Duration};

    #[derive(Default)]
    struct BuildProbe {
        count: usize,
        before_build: Option<Box<dyn FnOnce()>>,
    }

    thread_local! {
        static BUILDS: RefCell<BuildProbe> = RefCell::default();
    }

    pub(super) fn observe_build() {
        let before_build = BUILDS.with_borrow_mut(|probe| {
            probe.count += 1;
            probe.before_build.take()
        });
        if let Some(before_build) = before_build {
            before_build();
        }
    }

    fn build_count() -> usize {
        BUILDS.with_borrow(|probe| probe.count)
    }

    fn source_snapshot(paths: impl IntoIterator<Item = String>) -> Snapshot {
        let workspace = WorkspaceId::derive(&[b"source-search"]);
        let documents = paths
            .into_iter()
            .map(|path| Arc::new(fixture_document(workspace.clone(), path, b"", vec![]).unwrap()))
            .collect();
        fixture_snapshot(workspace, documents).unwrap()
    }

    fn search_query(filter: &str) -> ViewQuery {
        ViewQuery {
            filter: filter.into(),
            find: true,
            projection: Some(Projection::Summary),
            ..ViewQuery::default()
        }
    }

    #[test]
    fn linked_views_and_match_pages_build_one_ranking_for_paged_source() {
        let snapshot =
            source_snapshot((0..=MAX_MAP_FILES).map(|index| format!("src/item-{index:04}.rs")));
        let search = SourceSearch::default();
        let builds = build_count();
        let mut query = search_query("item-");
        let initial = search.view(&snapshot, &query).unwrap();
        let map_size = initial.browse.unwrap().size;
        assert_eq!(initial.files[0].path, "src/item-0000.rs");
        assert_eq!(initial.matches.unwrap().total, MAX_MAP_FILES + 1);

        query.results_offset = PAGE_SIZE;
        let page = search.matches(&snapshot, &query);
        assert_eq!(page.offset, PAGE_SIZE);
        assert_eq!(page.items[0].path, "src/item-0024.rs");

        query.offset = map_size;
        let paged = search.view(&snapshot, &query).unwrap();
        assert_eq!(paged.browse.unwrap().offset, map_size);
        assert_eq!(paged.files[0].path, "src/item-0100.rs");

        query.file = Some(page.items[0].file.as_str().into());
        query.reveal_match = true;
        let selected = search.view(&snapshot, &query).unwrap();
        assert_eq!(selected.selection.file.as_ref(), Some(&page.items[0].file));
        assert_eq!(selected.browse.unwrap().offset, 0);
        assert_eq!(build_count() - builds, 1);
    }

    #[test]
    fn a_slow_search_cannot_evict_a_ranking_requested_after_it_started() {
        let snapshot = source_snapshot(["earlier.rs".into(), "active.rs".into()]);
        let earlier = search_query("earlier");
        let active = search_query("active");
        let timeout = Duration::from_secs(5);

        for warm in [false, true] {
            let search = SourceSearch::default();
            if warm {
                search.view(&snapshot, &active).unwrap();
            }
            thread::scope(|scope| {
                let (started, wait_for_start) = mpsc::channel();
                let (finish, wait_to_finish) = mpsc::channel();
                let search = &search;
                let snapshot = &snapshot;
                let earlier = &earlier;
                let slow = scope.spawn(move || {
                    BUILDS.with_borrow_mut(|probe| {
                        probe.before_build = Some(Box::new(move || {
                            started.send(()).unwrap();
                            wait_to_finish.recv_timeout(timeout).unwrap();
                        }));
                    });
                    search.view(snapshot, earlier).unwrap()
                });

                wait_for_start.recv_timeout(timeout).unwrap();
                let current = search.view(snapshot, &active).unwrap();
                assert_eq!(current.matches.unwrap().items[0].path, "active.rs");
                let builds = build_count();
                finish.send(()).unwrap();
                let superseded = slow.join().unwrap();
                assert_eq!(superseded.matches.unwrap().items[0].path, "earlier.rs");

                let page = search.matches(snapshot, &active);
                assert_eq!(page.items[0].path, "active.rs");
                assert_eq!(build_count(), builds, "ranking already warm: {warm}");
            });
        }
    }

    #[test]
    fn oversized_query_bypasses_retention_without_displacing_the_cached_ranking() {
        let snapshot = fixture_snapshot(WorkspaceId::derive(&[b"budget"]), vec![]).unwrap();
        let search = SourceSearch::default();
        let ordinary = ViewQuery {
            filter: "reader".into(),
            ..ViewQuery::default()
        };
        let retained = search.rank(&snapshot, &ordinary);
        let oversized = ViewQuery {
            types: "x".repeat(RETAINED_BYTES),
            ..ordinary.clone()
        };
        assert_eq!(search.matches(&snapshot, &oversized).total, 0);
        assert!(Arc::ptr_eq(&retained, &search.rank(&snapshot, &ordinary)));
    }
}
