//! Bounded topic-gated source pools for complementary discovery intents.
use crate::{Context, Corpus, Error, SourceRole, SourceScope, compact, excerpt, require};
use std::{cmp::Reverse, collections::BTreeSet, ops::Range};

/// Suggested pool origin, not a resolved semantic role or relevance guarantee.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Declaration or constructor starting point.
    Definition,
    /// Potential entry or use location.
    Entry,
    /// Potential registration or extension mechanism.
    Registration,
    /// Supporting implementation example.
    Example,
    /// Potential check or rejection location.
    Validation,
}
struct Plan {
    topics: BTreeSet<String>,
    facets: [BTreeSet<String>; 2],
    roles: [Role; 2],
}
fn plan(query: &BTreeSet<String>) -> Option<Plan> {
    let (facets, roles) =
        if !query.is_disjoint(&compact::words("add extend register registration plugin")) {
            (
                [
                    "register registry registration table prototype dispatch macro arity signature",
                    "implement implementation example handler context result",
                ],
                [Role::Registration, Role::Example],
            )
        } else if !query.is_disjoint(&compact::words(
            "new create created creation construct constructor instantiate",
        )) {
            (
                [
                    "new create created creation constructor construct instantiate init initial",
                    "start begin open attempt allocate spawn make",
                ],
                [Role::Definition, Role::Entry],
            )
        } else if !query.is_disjoint(&compact::words("codepath trace")) {
            (
                [
                    "commit submit send dispatch deliver publish",
                    "validate validation check conflict reject rejection ensure guard",
                ],
                [Role::Entry, Role::Validation],
            )
        } else {
            return None;
        };
    let ignored = compact::words(
        "show me code codepath path take takes all check checks they go through please find locate location located happen happens source implementation function method functions methods new create created creation construct constructor instantiate add extend extension register registration plugin start",
    );
    let topics = query
        .difference(&ignored)
        .filter(|word| word.chars().count() >= 3)
        .take(8)
        .cloned()
        .collect::<BTreeSet<_>>();
    if topics.is_empty() {
        return None;
    }
    Some(Plan {
        topics,
        facets: facets.map(compact::words),
        roles,
    })
}
fn matches(topic: &str, term: &str) -> bool {
    term == topic || (topic.len() >= 4 && term.starts_with(topic) && term.len() <= topic.len() + 4)
}
fn bits(term: &str, keys: &BTreeSet<String>, prefix: bool) -> u64 {
    keys.iter()
        .enumerate()
        .filter(|(_, key)| {
            if prefix {
                matches(key, term)
            } else {
                key.as_str() == term
            }
        })
        .fold(0, |mask, (index, _)| mask | (1 << index))
}
fn text_bits(text: &str, keys: &BTreeSet<String>, prefix: bool) -> u64 {
    compact::words(text)
        .iter()
        .fold(0, |mask, term| mask | bits(term, keys, prefix))
}
fn ownership(scope: Option<&SourceScope>, topics: &BTreeSet<String>) -> u64 {
    scope.map_or(0, |scope| {
        std::iter::once(&scope.name)
            .chain(scope.parents.iter())
            .filter(|name| name.chars().all(|c| c.is_alphanumeric() || c == '_'))
            .fold(0, |mask, name| {
                mask | bits(&compact::stem(&name.to_lowercase()), topics, false)
            })
    })
}
fn tier(path: &str, role: SourceRole) -> u8 {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    if name == "tests.rs" || name.ends_with("_tests.rs") {
        return 2;
    }
    match role {
        SourceRole::Code => 0,
        SourceRole::Guide | SourceRole::Policy | SourceRole::Configuration => 1,
        SourceRole::Test | SourceRole::Mixed => 2,
        SourceRole::Historical => 3,
    }
}
type Priority = (
    u8,
    Reverse<u32>,
    Reverse<u32>,
    Reverse<u32>,
    Reverse<u32>,
    Reverse<u32>,
);
struct Candidate<'a> {
    index: usize,
    scope: usize,
    family: Option<(&'a str, &'a str)>,
    priority: Priority,
}
fn diversify<'a>(pool: Vec<Candidate<'a>>) -> Vec<Candidate<'a>> {
    let mut seen = BTreeSet::new();
    let mut first = vec![];
    let mut repeated = vec![];
    for candidate in pool {
        if candidate.family.is_none_or(|name| seen.insert(name)) {
            first.push(candidate);
        } else {
            repeated.push(candidate);
        }
    }
    first.extend(repeated);
    first
}
impl Corpus {
    /// Retrieves complementary topic-bound source locations for creation, extension and flow.
    ///
    /// Uses literal postings and raw scopes, without resolving calls or expanding macros.
    /// Other question forms retain compact selection. Bounds match [`Self::retrieve_compact`].
    pub fn retrieve_search(
        &self,
        question: &str,
        limit: usize,
        budget: usize,
    ) -> Result<Context, Error> {
        require(
            !question.trim().is_empty() && question.len() <= 4096,
            "Question must contain 1–4,096 UTF-8 bytes",
        )?;
        require(
            (1..=8).contains(&limit) && (1..=16000).contains(&budget),
            "Invalid QA context allowance",
        )?;
        let query = compact::words(question);
        let proposed = plan(&query);
        let Some(plan) = proposed
            .filter(|_| compact::query_intent(self, question, &query) == compact::Intent::Behavior)
        else {
            return self.retrieve_compact(question, limit, budget);
        };
        let mut masks = vec![[0u64; 3]; self.windows.len()];
        for (term, postings) in &self.postings {
            let term = compact::stem(term);
            let flags = [
                bits(&term, &plan.topics, true),
                bits(&term, &plan.facets[0], false),
                bits(&term, &plan.facets[1], false),
            ];
            if flags.iter().all(|&mask| mask == 0) {
                continue;
            }
            for &(index, _) in postings {
                for (slot, flag) in masks[index].iter_mut().zip(flags) {
                    *slot |= flag;
                }
            }
        }
        let mut pools: [Vec<Candidate<'_>>; 2] = [vec![], vec![]];
        for (index, window) in self.windows.iter().enumerate() {
            let file = &window.file;
            let scope = file.metadata.scope(window.range.start);
            let owned = ownership(scope.map(|s| &s.label), &plan.topics);
            let topic = masks[index][0] | owned;
            if topic == 0 {
                continue;
            }
            let scope_text = scope
                .map(|s| {
                    format!(
                        "{} {} {}",
                        s.label.kind,
                        s.label.name,
                        s.label.parents.join(" ")
                    )
                })
                .unwrap_or_default();
            let scope_topic = text_bits(&scope_text, &plan.topics, true).count_ones();
            for (facet, pool) in pools.iter_mut().enumerate() {
                let body = masks[index][facet + 1];
                let contextual = text_bits(&scope_text, &plan.facets[facet], false).count_ones();
                if body == 0 && contextual == 0 {
                    continue;
                }
                let role = plan.roles[facet];
                let named = scope
                    .filter(|s| matches!(s.label.kind.as_str(), "function" | "method"))
                    .map_or(0, |s| {
                        text_bits(&s.label.name, &plan.facets[facet], false).count_ones()
                    })
                    + u32::from(
                        role == Role::Registration
                            && scope.is_some_and(|s| s.label.kind == "macro"),
                    );
                let definition_owned = if matches!(role, Role::Definition | Role::Registration) {
                    owned.count_ones()
                } else {
                    0
                };
                pool.push(Candidate {
                    index,
                    scope: scope.map_or(window.range.start, |s| s.range.start),
                    family: scope.map(|s| (s.label.kind.as_str(), s.label.name.as_str())),
                    priority: (
                        tier(&file.path, file.metadata.role_for(&window.range)),
                        Reverse(named),
                        Reverse(definition_owned),
                        Reverse(body.count_ones()),
                        Reverse(topic.count_ones()),
                        Reverse(scope_topic),
                    ),
                });
            }
        }
        for pool in &mut pools {
            pool.sort_by_key(|c| (c.priority, c.index));
        }
        let focus = plan
            .facets
            .each_ref()
            .map(|facet| plan.topics.union(facet).cloned().collect::<BTreeSet<_>>());
        let mut pools = pools.map(|pool| diversify(pool).into_iter());
        let mut evidence = vec![];
        let mut roles = vec![];
        let mut seen = BTreeSet::new();
        let mut ranges: Vec<(String, Range<usize>)> = vec![];
        let mut used = 0;
        let mut represented = 0;
        let baseline = self.retrieve_compact(question, limit, budget)?;
        for mut source in baseline.evidence {
            let owned = ownership(source.scope.as_ref(), &plan.topics);
            let location = source
                .scope
                .as_ref()
                .map(|s| format!("{} {} {}", s.kind, s.name, s.parents.join(" ")))
                .unwrap_or_default();
            let facet = plan.facets.iter().position(|facet| {
                text_bits(&format!("{} {location}", source.text), facet, false) != 0
            });
            if tier(&source.path, source.role) != 0 || owned & !represented == 0 || facet.is_none()
            {
                continue;
            }
            let file = self
                .files
                .get(&source.path)
                .ok_or_else(|| Error("Baseline source is unavailable".into()))?;
            let key = file
                .metadata
                .scope(source.range.start)
                .map_or(source.range.start, |s| s.range.start);
            seen.insert((file.path.as_str(), key));
            ranges.push((source.path.clone(), source.range.clone()));
            used += source.range.len();
            represented |= owned;
            source.label = evidence.len() + 1;
            if let Some(facet) = facet {
                roles.push(plan.roles[facet]);
            }
            evidence.push(source);
            if evidence.len() == limit {
                break;
            }
        }
        let allowance = (budget / limit).clamp(128, 1000).min(budget);
        while evidence.len() < limit {
            let mut added = false;
            for (facet, pool) in pools.iter_mut().enumerate() {
                for candidate in pool.by_ref() {
                    let window = &self.windows[candidate.index];
                    let file = &window.file;
                    if seen.contains(&(file.path.as_str(), candidate.scope)) {
                        continue;
                    }
                    let range = compact::select_range(
                        file,
                        compact::scope_range(file, &window.range),
                        &focus[facet],
                        allowance,
                    );
                    if range.is_empty() || used + range.len() > budget {
                        continue;
                    }
                    let body = &file.text[range.clone()];
                    let location = file
                        .metadata
                        .scope(range.start)
                        .map(|s| {
                            format!(
                                "{} {} {}",
                                s.label.kind,
                                s.label.name,
                                s.label.parents.join(" ")
                            )
                        })
                        .unwrap_or_default();
                    if text_bits(
                        &format!("{} {body} {location}", file.path),
                        &plan.topics,
                        true,
                    ) == 0
                        || text_bits(&format!("{body} {location}"), &plan.facets[facet], false) == 0
                    {
                        continue;
                    }
                    if ranges.iter().any(|(path, other)| {
                        path == &file.path
                            && other
                                .end
                                .min(range.end)
                                .saturating_sub(other.start.max(range.start))
                                * 2
                                > other.len().min(range.len())
                    }) {
                        continue;
                    }
                    used += range.len();
                    evidence.push(excerpt(file, &range, evidence.len() + 1));
                    roles.push(plan.roles[facet]);
                    seen.insert((file.path.as_str(), candidate.scope));
                    ranges.push((file.path.clone(), range));
                    added = true;
                    break;
                }
                if evidence.len() == limit {
                    break;
                }
            }
            if !added {
                break;
            }
        }
        Ok(Context {
            snapshot: self.snapshot.clone(),
            evidence,
            roles,
        })
    }
}
