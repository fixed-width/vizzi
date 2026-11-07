//! Repository-independent role-aware ranking and literal compact excerpt selection.
use crate::{Context, Corpus, Error, IndexedFile, SourceRole, excerpt, floor, require, terms};
use std::{collections::BTreeSet, ops::Range};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Intent {
    Overview,
    Policy,
    Manifest,
    Configuration,
    Entry,
    Tests,
    Behavior,
}
pub(super) fn stem(word: &str) -> String {
    match word {
        "configuration" | "configured" | "configure" => return "config".into(),
        "environment" => return "env".into(),
        "error" | "errors" | "err" | "failed" | "fails" | "failure" => return "error".into(),
        "validation" | "validating" | "validate" => return "valid".into(),
        _ => {}
    }
    let mut s = word.to_owned();
    if s.len() > 5 && s.ends_with("ies") {
        s.truncate(s.len() - 3);
        s.push('y');
    } else if s.len() > 5 && s.ends_with("ing") {
        s.truncate(s.len() - 3);
        let chars = s.chars().rev().take(2).collect::<Vec<_>>();
        if chars.len() == 2 && chars[0] == chars[1] {
            s.pop();
        }
    } else if s.len() > 4 && s.ends_with('s') {
        s.pop();
    }
    if s.len() > 4 && s.ends_with('e') {
        s.pop();
    }
    s
}
pub(super) fn words(text: &str) -> BTreeSet<String> {
    terms(text).keys().map(|w| stem(w)).collect()
}
fn hits(a: &BTreeSet<String>, b: &BTreeSet<String>) -> usize {
    a.intersection(b).count()
}
fn intent(query: &BTreeSet<String>, named: bool) -> Intent {
    let has = |set: &[&str]| set.iter().any(|w| query.contains(*w));
    if has(&[
        "contributor",
        "policy",
        "unsafe",
        "submit",
        "format",
        "lint",
    ]) {
        Intent::Policy
    } else if has(&["edition", "language", "toolchain", "depend", "manifest"]) {
        Intent::Manifest
    } else if has(&["regression", "test"]) {
        Intent::Tests
    } else if has(&["entry", "executabl", "boot", "listener"])
        || (has(&["start"]) && has(&["program", "server"]))
    {
        Intent::Entry
    } else if has(&["config", "env", "setting"]) {
        Intent::Configuration
    } else if !named && has(&["project", "codebas", "intend", "purpos", "application"]) {
        Intent::Overview
    } else {
        Intent::Behavior
    }
}
fn package_words(corpus: &Corpus, path: &str) -> BTreeSet<String> {
    let mut parent = path.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
    loop {
        let manifest = if parent.is_empty() {
            "Cargo.toml".into()
        } else {
            format!("{parent}/Cargo.toml")
        };
        if let Some(file) = corpus.files.get(&manifest) {
            let mut section = false;
            let mut text = String::new();
            for line in file.text.lines() {
                let trimmed = line.trim();
                if trimmed == "[package]" {
                    section = true;
                    continue;
                }
                if trimmed.starts_with('[') && section {
                    break;
                }
                if section {
                    text.push_str(line);
                    text.push('\n');
                }
            }
            return words(&text);
        }
        if parent.is_empty() {
            return BTreeSet::new();
        }
        parent = parent.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
    }
}
pub(super) fn scope_range(file: &IndexedFile, window: &Range<usize>) -> Range<usize> {
    if let Some(scope) = file.metadata.scope(window.start) {
        if scope.range.len() <= 4096 {
            return scope.range.clone();
        }
        return window.start..window.end.min(scope.range.end);
    }
    window.clone()
}
pub(super) fn select_range(
    file: &IndexedFile,
    allowed: Range<usize>,
    query: &BTreeSet<String>,
    allowance: usize,
) -> Range<usize> {
    if allowed.len() <= allowance {
        return allowed;
    }
    let mut best = (0, allowed.start);
    let mut offset = allowed.start;
    for line in file.text[allowed.clone()].split_inclusive('\n') {
        let score = hits(query, &words(line));
        if score > best.0 {
            best = (score, offset);
        }
        offset += line.len();
    }
    let wanted = best.1.saturating_sub(allowance / 4).max(allowed.start);
    let mut start = floor(&file.text, wanted);
    if start > allowed.start
        && let Some(next) = file.text[start..best.1].find('\n')
    {
        start += next + 1;
    }
    let end = floor(&file.text, start.saturating_add(allowance).min(allowed.end));
    let end = if end < allowed.end {
        file.text[start..end]
            .rfind('\n')
            .filter(|n| *n > allowance / 2)
            .map_or(end, |n| start + n + 1)
    } else {
        end
    };
    start..end
}
fn role_weight(role: SourceRole, intent: Intent) -> f64 {
    match role {
        SourceRole::Historical => 0.18,
        SourceRole::Test => {
            if intent == Intent::Tests {
                2.8
            } else {
                0.28
            }
        }
        SourceRole::Mixed => 0.4,
        SourceRole::Policy => {
            if intent == Intent::Policy {
                2.4
            } else {
                1.1
            }
        }
        SourceRole::Guide => {
            if intent == Intent::Overview {
                2.2
            } else {
                1.05
            }
        }
        SourceRole::Configuration => {
            if matches!(intent, Intent::Configuration | Intent::Manifest) {
                1.8
            } else {
                0.9
            }
        }
        SourceRole::Code => 1.0,
    }
}
pub(super) fn query_intent(corpus: &Corpus, question: &str, query: &BTreeSet<String>) -> Intent {
    let lower = question.to_lowercase();
    let named = corpus
        .files
        .values()
        .flat_map(|f| f.metadata.scopes.iter())
        .any(|s| {
            s.label.name.len() >= 4
                && (s.label.name.contains('_')
                    || s.label.name.chars().skip(1).any(char::is_uppercase))
                && lower.contains(&s.label.name.to_lowercase())
        });
    intent(query, named)
}
impl Corpus {
    /// Selects role-aware, query-centred literal evidence under a compact byte allowance.
    pub fn retrieve_compact(
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
        let mut query = words(question);
        let plan = query_intent(self, question, &query);
        if plan == Intent::Policy && query.contains("check") {
            query.extend(["test".into(), "format".into(), "valid".into()]);
        }
        let legacy = self.lexical_scores(question);
        let mut candidates = vec![];
        let mut packages = std::collections::BTreeMap::new();
        for (i, window) in self.windows.iter().enumerate() {
            let file = &window.file;
            let scope = file.metadata.scope(window.range.start);
            let scope_words = scope
                .map(|s| words(&format!("{} {}", s.label.name, s.label.parents.join(" "))))
                .unwrap_or_default();
            let path_words = words(&file.path);
            let package = packages
                .entry(file.path.clone())
                .or_insert_with(|| package_words(self, &file.path));
            let main = file.path.rsplit('/').next() == Some("main.rs")
                || file.path.ends_with("__main__.py");
            let root_readme = file.path.eq_ignore_ascii_case("README.md");
            let seed = match plan {
                Intent::Overview if root_readme => 8.0,
                Intent::Manifest if file.path == "Cargo.toml" => 7.0,
                Intent::Policy if file.metadata.role == SourceRole::Policy => 3.0,
                Intent::Entry if main => 6.0 + hits(&query, package) as f64 * 2.0,
                _ => 0.0,
            };
            if !legacy.contains_key(&i) && seed == 0.0 && hits(&query, &scope_words) == 0 {
                continue;
            }
            let allowed = scope_range(file, &window.range);
            let allowance = (budget / limit).clamp(128, 1000).min(budget);
            let introduction = root_readme && window.range.start == 0 && plan == Intent::Overview;
            let range = if introduction {
                0..floor(&file.text, allowance)
            } else {
                select_range(file, allowed, &query, allowance)
            };
            if range.is_empty() {
                continue;
            }
            let body = words(&file.text[range.clone()]);
            let body_hits = hits(&query, &body);
            let scope_hits = hits(&query, &scope_words);
            let path_hits = hits(&query, &path_words);
            if body_hits + scope_hits + path_hits == 0 && seed == 0.0 {
                continue;
            }
            let role = file.metadata.role_for(&range);
            let header = if root_readme && window.range.start == 0 && plan == Intent::Overview {
                100.0
            } else {
                0.0
            };
            let score = (body_hits as f64 * 3.0
                + scope_hits as f64 * 2.0
                + path_hits as f64
                + legacy.get(&i).copied().unwrap_or(0.0) * 0.15
                + seed
                + header)
                * role_weight(role, plan);
            candidates.push((score, i, range));
        }
        candidates.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut evidence = vec![];
        let mut ranges: Vec<(String, Range<usize>)> = vec![];
        let mut used = 0;
        for (_, i, range) in candidates {
            let file = &self.windows[i].file;
            if used + range.len() > budget {
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
            ranges.push((file.path.clone(), range));
            if evidence.len() == limit {
                break;
            }
        }
        Ok(Context {
            snapshot: self.snapshot.clone(),
            evidence,
        })
    }
}
