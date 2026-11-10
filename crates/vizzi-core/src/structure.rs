//! Finite inventories and call destinations over captured metadata; no source IO.
use crate::{
    calls::TargetBasis,
    comparison::ReviewPath,
    model::{Coverage, Document, Snapshot},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

const FILES: usize = 4096;
const ITEMS: usize = 65536;
const AREAS: usize = 8192;
const PAGE: usize = 24;
const SITES: usize = 8;
/// Exact native document/context location, independent of a display label.
#[derive(Clone, Debug, Serialize)]
pub struct Source {
    /// Relative path.
    pub path: String,
    /// Native file identity.
    pub file: String,
    /// Exact document version.
    pub version: String,
    /// Analyzer context identity.
    pub context: String,
    /// First represented source line.
    pub line: usize,
    /// Last represented source line.
    pub end_line: usize,
}
/// Captured declaration, not a resolved runtime entity.
#[derive(Clone, Debug, Serialize)]
pub struct Symbol {
    /// Tracking identity; duplicate lexical keys are scoped to captured content.
    pub tracking: String,
    /// Opaque identity for this observation/version/context.
    pub id: String,
    /// Literal declaration name.
    pub name: String,
    /// Raw declaration kind.
    pub kind: String,
    /// Enclosing declaration names, outermost first, bounded to sixteen.
    pub parents: Vec<String>,
    /// Captured definition location.
    pub source: Source,
}
/// Projection allowances and unavailable capability, not a completeness guarantee.
#[derive(Default, Clone, Debug, Serialize)]
pub struct CoverageInfo {
    /// Documents in the native source observation.
    pub available_files: usize,
    /// Documents admitted to this projection.
    pub indexed_files: usize,
    /// Documents outside path/count allowance.
    pub omitted_files: usize,
    /// Omitted declarations.
    pub omitted_symbols: usize,
    /// Omitted call sites, including extraction omissions.
    pub omitted_calls: usize,
    /// Documents with unavailable call evidence.
    pub unavailable_call_files: usize,
    /// Documents requiring grammar recovery.
    pub partial_files: usize,
    /// Areas outside the area metadata allowance.
    pub omitted_areas: usize,
}
/// One finite, deterministic page.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    /// Exact native observation.
    pub snapshot: String,
    /// Shared scope/capability limits.
    pub coverage: CoverageInfo,
    /// Total matches in the admitted projection.
    pub matching: usize,
    /// Current page.
    pub items: Vec<T>,
    /// Continuation offset, absent when exhausted.
    pub next: Option<usize>,
}
/// A source directory or file that can be selected without entering a path.
#[derive(Default, Clone, Debug, Serialize)]
pub struct Area {
    /// Normalized relative path; empty means repository root.
    pub path: String,
    /// Admitted files contained here.
    pub files: usize,
}
/// Inventory entry with ambiguous terminal-name usage hints.
#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    /// Declaration.
    pub symbol: Symbol,
    /// Written calls with the same terminal name; not proven uses of this declaration.
    pub candidate_uses: usize,
    /// Number of declarations sharing the terminal name.
    pub same_name_definitions: usize,
}
/// One written invocation and finite candidate destinations.
#[derive(Clone, Debug, Serialize)]
pub struct Invocation {
    /// Provider/configuration/source identity for language-server evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_context: Option<String>,
    /// Exact invocation identity for this observation and captured callee range.
    pub id: String,
    /// Language evidence selecting the sole destination, otherwise name-based candidates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<TargetBasis>,
    /// Exact callee spelling.
    pub spelling: String,
    /// Terminal callee name when recognizable.
    pub name: Option<String>,
    /// Captured call-site location.
    pub source: Source,
    /// Nearest emitted enclosing declaration, if admitted.
    pub owner: Option<String>,
    /// Sole source-local destination when binding is present, otherwise name candidates.
    pub candidates: Vec<Symbol>,
    /// All matching candidate declarations before the response allowance.
    pub candidate_count: usize,
    /// Site is outside the selected source area.
    pub outside_area: bool,
}
/// A captured identifier range, with UTF-8 byte columns within one source line.
#[derive(Clone, Debug, Serialize)]
pub struct CallPosition {
    /// One-based source line containing the identifier.
    pub line: usize,
    /// Inclusive UTF-8 byte column.
    pub start: usize,
    /// Exclusive UTF-8 byte column.
    pub end: usize,
}
/// Lightweight source action; destinations are retrieved by native identity on selection.
#[derive(Clone, Debug, Serialize)]
pub struct CallAction {
    /// Observation-bound call identity.
    pub id: String,
    /// Exact identifier text.
    pub name: String,
    /// Captured location to decorate without text-name matching.
    pub position: CallPosition,
    /// Source-local evidence, when already known.
    pub binding: Option<TargetBasis>,
}
struct Site {
    position: Option<CallPosition>,
    id: String,
    identifier: Option<std::ops::Range<usize>>,
    language: String,
    target: Option<(String, TargetBasis)>,
    spelling: String,
    name: Option<String>,
    source: Source,
    owner: Option<String>,
}
/// Metadata index owning no source text or native snapshot Arc.
pub struct Structure {
    tracking: BTreeMap<String, Option<usize>>,
    search: Vec<crate::structure_search::Name>,
    snapshot: String,
    coverage: CoverageInfo,
    areas: BTreeMap<String, Area>,
    symbols: Vec<Symbol>,
    ids: BTreeMap<String, usize>,
    names: BTreeMap<String, Vec<usize>>,
    sites: Vec<Site>,
    owners: BTreeMap<String, Vec<usize>>,
    uses: BTreeMap<String, Vec<usize>>,
    spans: Vec<Option<std::ops::Range<usize>>>,
    files: BTreeMap<String, Vec<usize>>,
    call_ids: BTreeMap<String, usize>,
}
/// Captured call witness used by a host definition provider.
#[derive(Clone, Debug)]
pub struct CallLookup {
    /// Native invocation, including owner and version-bound source.
    pub invocation: Invocation,
    /// Terminal identifier byte range, absent in legacy/dynamic calls.
    pub identifier: Option<std::ops::Range<usize>>,
    /// Analyzer language, independent of extension.
    pub language: String,
}
/// Invalid scope, allowance or expired symbol identity.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(pub String);
fn within(path: &str, area: &str) -> bool {
    area.is_empty()
        || path == area
        || path
            .strip_prefix(area)
            .is_some_and(|suffix| suffix.starts_with('/'))
}
fn validate(area: &str, search: &str) -> Result<(), Error> {
    if area.len() > 4096 || search.len() > 256 {
        return Err(Error(
            "Structural scope/search exceeds its byte allowance".into(),
        ));
    }
    if !area.is_empty() {
        if area.contains('\\') || area.split('/').any(|p| matches!(p, "" | "." | "..")) {
            return Err(Error("Choose a normalized source area".into()));
        }
        ReviewPath::parse(area.as_bytes().to_vec()).map_err(|e| Error(e.to_string()))?;
    }
    Ok(())
}
fn source(doc: &Document, context: &str, span: &std::ops::Range<usize>) -> Source {
    let lines = doc.lines(span);
    Source {
        path: doc.path().into(),
        file: doc.id().as_str().into(),
        version: doc.version().as_str().into(),
        context: context.into(),
        line: lines.start,
        end_line: lines.end,
    }
}
fn bounded(text: &str) -> String {
    let mut end = text.len().min(128);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].into()
}
impl Structure {
    /// Builds from already-validated captured metadata, without reading or parsing source.
    pub fn new(snapshot: &Snapshot) -> Self {
        let mut index = Self {
            tracking: BTreeMap::new(),
            search: vec![],
            snapshot: snapshot.id().as_str().into(),
            coverage: CoverageInfo {
                available_files: snapshot.documents().len(),
                ..Default::default()
            },
            areas: BTreeMap::new(),
            symbols: vec![],
            ids: BTreeMap::new(),
            names: BTreeMap::new(),
            sites: vec![],
            owners: BTreeMap::new(),
            uses: BTreeMap::new(),
            spans: vec![],
            files: BTreeMap::new(),
            call_ids: BTreeMap::new(),
        };
        for doc in snapshot.documents() {
            if index.coverage.indexed_files == FILES || doc.path().len() > 4096 {
                index.coverage.omitted_files += 1;
                continue;
            }
            index.coverage.indexed_files += 1;
            let mut path = doc.path();
            loop {
                if let Some(area) = index.areas.get_mut(path) {
                    area.files += 1;
                } else if index.areas.len() < AREAS {
                    index.areas.insert(
                        path.into(),
                        Area {
                            path: path.into(),
                            files: 1,
                        },
                    );
                } else {
                    index.coverage.omitted_areas += 1;
                }
                if path.is_empty() {
                    break;
                }
                path = path.rsplit_once('/').map_or("", |(parent, _)| parent);
            }
            let Some(analysis) = doc
                .analyses()
                .iter()
                .find(|a| matches!(a.coverage, Coverage::Syntax | Coverage::Partial))
            else {
                index.coverage.unavailable_call_files += 1;
                continue;
            };
            index.coverage.partial_files += usize::from(analysis.coverage == Coverage::Partial);
            let mut mapping = BTreeMap::new();
            for (ordinal, fact) in analysis.facts.iter().enumerate() {
                if index.symbols.len() == ITEMS
                    || fact.name.is_empty()
                    || fact.name.len() > 128
                    || fact.kind.len() > 128
                {
                    index.coverage.omitted_symbols += 1;
                    continue;
                }
                let mut hash = blake3::Hasher::new();
                for value in [
                    index.snapshot.as_str(),
                    doc.id().as_str(),
                    doc.version().as_str(),
                    analysis.context.id.as_str(),
                    fact.key.as_str(),
                ] {
                    hash.update(&(value.len() as u64).to_le_bytes());
                    hash.update(value.as_bytes());
                }
                let id = hash.finalize().to_hex().to_string();
                let mut parents = vec![];
                let mut parent = fact.parent;
                for _ in 0..16 {
                    let Some(p) = parent.and_then(|n| analysis.facts.get(n)) else {
                        break;
                    };
                    parents.push(bounded(&p.name));
                    parent = p.parent;
                }
                parents.reverse();
                let n = index.symbols.len();
                index.ids.insert(id.clone(), n);
                index.spans.push(fact.name_span.clone());
                index.files.entry(doc.path().into()).or_default().push(n);
                index.names.entry(fact.name.clone()).or_default().push(n);
                mapping.insert(ordinal, id.clone());
                let symbol = Symbol {
                    tracking: doc.entity_id(analysis, fact).as_str().into(),
                    id,
                    name: fact.name.clone(),
                    kind: fact.kind.clone(),
                    parents,
                    source: source(doc, analysis.context.id.as_str(), &fact.span),
                };
                index
                    .search
                    .push(crate::structure_search::Name::new(&symbol));
                index
                    .tracking
                    .entry(symbol.tracking.clone())
                    .and_modify(|n| *n = None)
                    .or_insert(Some(n));
                index.symbols.push(symbol);
            }
            let Some(calls) = &analysis.calls else {
                index.coverage.unavailable_call_files += 1;
                continue;
            };
            index.coverage.omitted_calls += calls.omitted;
            for call in &calls.sites {
                if index.sites.len() == ITEMS {
                    index.coverage.omitted_calls += 1;
                    continue;
                }
                let owner = call.owner.and_then(|n| mapping.get(&n)).cloned();
                let n = index.sites.len();
                if let Some(owner) = &owner {
                    index.owners.entry(owner.clone()).or_default().push(n);
                }
                if let Some(name) = &call.name {
                    index.uses.entry(name.clone()).or_default().push(n);
                }
                index.sites.push(Site {
                    position: call.name_span.as_ref().and_then(|span| {
                        let lines = doc.lines(span);
                        let line = doc.captured().line_range(lines.start)?;
                        (lines.start == lines.end && span.end <= line.end).then_some(CallPosition {
                            line: lines.start,
                            start: span.start - line.start,
                            end: span.end - line.start,
                        })
                    }),
                    id: blake3::hash(
                        format!(
                            "{}:{}:{}:{}:{}:{}",
                            index.snapshot,
                            doc.id().as_str(),
                            doc.version().as_str(),
                            analysis.context.id.as_str(),
                            call.span.start,
                            call.span.end
                        )
                        .as_bytes(),
                    )
                    .to_hex()
                    .to_string(),
                    identifier: call.name_span.clone(),
                    language: analysis.context.language.clone(),
                    target: call.target.as_ref().and_then(|target| {
                        mapping
                            .get(&target.declaration)
                            .map(|id| (id.clone(), target.basis))
                    }),
                    spelling: call.spelling.clone(),
                    name: call.name.clone(),
                    source: source(doc, analysis.context.id.as_str(), &call.span),
                    owner,
                });
            }
        }
        index.call_ids = index
            .sites
            .iter()
            .enumerate()
            .map(|(n, site)| (site.id.clone(), n))
            .collect();
        index
    }
    fn page<I: IntoIterator, T>(
        &self,
        values: I,
        offset: usize,
        limit: usize,
        project: impl Fn(I::Item) -> T,
    ) -> Page<T> {
        let mut matching = 0;
        let mut items = vec![];
        for value in values {
            if matching >= offset && items.len() < limit {
                items.push(project(value));
            }
            matching += 1;
        }
        let next = (offset.saturating_add(items.len()) < matching)
            .then_some(offset.saturating_add(items.len()));
        Page {
            snapshot: self.snapshot.clone(),
            coverage: self.coverage.clone(),
            matching,
            items,
            next,
        }
    }
    /// Finds source areas by a case-insensitive literal substring; no intent interpretation.
    pub fn areas(&self, search: &str, offset: usize) -> Result<Page<Area>, Error> {
        validate("", search)?;
        let search = search.trim().to_lowercase();
        let query = crate::structure_search::Query::new(&search);
        let matching_files = self
            .symbols
            .iter()
            .enumerate()
            .filter(|(n, _)| query.rank(&self.search[*n]).is_some())
            .map(|(_, s)| s.source.path.as_str())
            .collect::<BTreeSet<_>>();
        Ok(self.page(
            self.areas.values().filter(|a| {
                a.path.to_lowercase().contains(&search) || matching_files.contains(a.path.as_str())
            }),
            offset,
            PAGE,
            Clone::clone,
        ))
    }
    /// Searches declarations, ranking exact/qualified names before prefixes, substrings and paths.
    pub fn inventory(&self, area: &str, search: &str, offset: usize) -> Result<Page<Entry>, Error> {
        self.search_inventory(area, search, offset, false)
    }
    /// Searches only functions, filtering before ranking and paging.
    pub fn functions(&self, area: &str, search: &str, offset: usize) -> Result<Page<Entry>, Error> {
        self.search_inventory(area, search, offset, true)
    }
    fn search_inventory(
        &self,
        area: &str,
        search: &str,
        offset: usize,
        functions: bool,
    ) -> Result<Page<Entry>, Error> {
        validate(area, search)?;
        let query = crate::structure_search::Query::new(search);
        let mut matches = self
            .symbols
            .iter()
            .enumerate()
            .filter(|(_, s)| within(&s.source.path, area) && (!functions || s.kind == "function"))
            .filter_map(|(n, s)| query.rank(&self.search[n]).map(|rank| (rank, n, s)))
            .collect::<Vec<_>>();
        if !query.empty() || functions {
            matches.sort_by_key(|(rank, n, s)| {
                (
                    *rank,
                    self.search[*n].simple.as_str(),
                    self.search[*n].qualified.as_str(),
                    self.search[*n].path.as_str(),
                    s.source.path.as_str(),
                    s.source.line,
                    *n,
                )
            });
        }
        Ok(self.page(matches, offset, PAGE, |(_, _, s)| Entry {
            symbol: s.clone(),
            candidate_uses: self.uses.get(&s.name).map_or(0, Vec::len),
            same_name_definitions: self.names.get(&s.name).map_or(0, Vec::len),
        }))
    }
    fn symbol(&self, id: &str) -> Result<&Symbol, Error> {
        self.ids
            .get(id)
            .map(|n| &self.symbols[*n])
            .ok_or_else(|| Error("Symbol is absent from this observation".into()))
    }
    fn invocation(&self, site: &Site, area: &str) -> Invocation {
        if let Some((id, basis)) = &site.target
            && let Ok(target) = self.symbol(id)
        {
            return Invocation {
                semantic_context: None,
                id: site.id.clone(),
                binding: Some(*basis),
                spelling: site.spelling.clone(),
                name: site.name.clone(),
                source: site.source.clone(),
                owner: site.owner.clone(),
                candidates: vec![target.clone()],
                candidate_count: 1,
                outside_area: !within(&site.source.path, area),
            };
        }
        let mut candidates = site
            .name
            .as_ref()
            .and_then(|name| self.names.get(name))
            .into_iter()
            .flatten()
            .map(|n| &self.symbols[*n])
            .filter(|s| {
                matches!(
                    s.kind.as_str(),
                    "function" | "struct" | "enum" | "type" | "class"
                )
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|s| {
            (
                !within(&s.source.path, area),
                s.source.file != site.source.file,
                s.source.path.as_str(),
                s.id.as_str(),
            )
        });
        let candidate_count = candidates.len();
        Invocation {
            semantic_context: None,
            id: site.id.clone(),
            binding: None,
            spelling: site.spelling.clone(),
            name: site.name.clone(),
            source: site.source.clone(),
            owner: site.owner.clone(),
            candidates: candidates.into_iter().take(SITES).cloned().collect(),
            candidate_count,
            outside_area: !within(&site.source.path, area),
        }
    }
    /// Retrieves a call by native identity without loading source or accepting a browser position.
    pub fn call(&self, id: &str) -> Option<CallLookup> {
        let site = &self.sites[*self.call_ids.get(id)?];
        Some(CallLookup {
            invocation: self.invocation(site, ""),
            identifier: site.identifier.clone(),
            language: site.language.clone(),
        })
    }
    /// Maps a definition position to its narrowest admitted declaration; ties remain ambiguous.
    pub fn definition(&self, path: &str, byte: usize) -> Option<Symbol> {
        let mut matches = self
            .files
            .get(path)?
            .iter()
            .filter_map(|n| {
                let span = self.spans[*n].as_ref()?;
                span.contains(&byte)
                    .then_some((span.len(), &self.symbols[*n]))
            })
            .collect::<Vec<_>>();
        matches.sort_by_key(|(len, _)| *len);
        let first = matches.first()?;
        if matches.get(1).is_some_and(|other| other.0 == first.0) {
            return None;
        }
        Some(first.1.clone())
    }
    fn anchored(&self, id: &str, area: &str) -> Result<&Symbol, Error> {
        validate(area, "")?;
        if area.is_empty() {
            return Err(Error("Choose a source area before following calls".into()));
        }
        self.symbol(id)
    }
    /// Lists invocations syntactically enclosed by a declaration, without automatic expansion.
    pub fn outgoing(&self, id: &str, area: &str, offset: usize) -> Result<Page<Invocation>, Error> {
        self.anchored(id, area)?;
        Ok(self.page(
            self.owners.get(id).into_iter().flatten(),
            offset,
            SITES,
            |n| self.invocation(&self.sites[*n], area),
        ))
    }
    /// Returns identifier actions in a bounded visible section, without source IO or name expansion.
    pub fn actions(
        &self,
        id: &str,
        line_start: usize,
        line_count: usize,
        offset: usize,
    ) -> Result<Page<CallAction>, Error> {
        self.symbol(id)?;
        if line_start == 0 || !(1..=96).contains(&line_count) {
            return Err(Error(
                "Call actions require a positive source line and at most 96 lines".into(),
            ));
        }
        let end = line_start.saturating_add(line_count);
        Ok(self.page(
            self.owners.get(id).into_iter().flatten().filter_map(|n| {
                let site = &self.sites[*n];
                let position = site.position.as_ref()?;
                let name = site.name.as_ref()?;
                (position.line >= line_start && position.line < end)
                    .then_some((site, position, name))
            }),
            offset,
            256,
            |(site, position, name)| CallAction {
                id: site.id.clone(),
                name: name.clone(),
                position: position.clone(),
                binding: site.target.as_ref().map(|(_, basis)| *basis),
            },
        ))
    }
    /// Lists sites sharing a declaration's terminal name; same-name ambiguity is preserved.
    pub fn uses(&self, id: &str, area: &str, offset: usize) -> Result<Page<Invocation>, Error> {
        let symbol = self.anchored(id, area)?;
        Ok(self.page(
            self.uses.get(&symbol.name).into_iter().flatten(),
            offset,
            SITES,
            |n| self.invocation(&self.sites[*n], area),
        ))
    }
}

/// Fresh evidence for a retained visual relationship.
#[derive(Debug, Serialize)]
pub struct LinkCheck {
    /// Current, removed, or unverified; uncertainty never establishes a call target.
    pub state: &'static str,
    /// Fresh source-local or reader-selected candidate evidence kind.
    pub kind: Option<&'static str>,
    /// Current written call location when uniquely available.
    pub source: Option<Source>,
}
impl Structure {
    /// Resolves a declaration tracking reference, with exact-ID migration for older clients.
    pub fn resolve(&self, tracking: Option<&str>, id: &str) -> Option<&Symbol> {
        let n = match tracking {
            Some(key) => self.tracking.get(key).copied().flatten()?,
            None => *self.ids.get(id)?,
        };
        self.symbols.get(n)
    }
    /// Revalidates one unique written site without carrying old language-server certainty forward.
    pub fn check_link(
        &self,
        from: &str,
        to: &str,
        spelling: &str,
        kind: &str,
    ) -> Result<LinkCheck, Error> {
        self.symbol(from)?;
        let target = self.symbol(to)?;
        let mut sites = self
            .owners
            .get(from)
            .into_iter()
            .flatten()
            .map(|n| &self.sites[*n])
            .filter(|s| s.spelling == spelling);
        let Some(site) = sites.next() else {
            return Ok(LinkCheck {
                state: "removed",
                kind: None,
                source: None,
            });
        };
        let evidence = if sites.next().is_some() {
            None
        } else if site
            .target
            .as_ref()
            .is_some_and(|(id, basis)| id == to && *basis == TargetBasis::RustSelfReceiver)
        {
            Some("self")
        } else if kind == "candidate"
            && site.name.as_deref() == Some(target.name.as_str())
            && site.target.is_none()
        {
            Some("candidate")
        } else {
            None
        };
        Ok(LinkCheck {
            state: if evidence.is_some() {
                "current"
            } else {
                "unverified"
            },
            kind: evidence,
            source: Some(site.source.clone()),
        })
    }
}
