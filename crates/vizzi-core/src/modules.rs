//! Declared language-module identity over immutable captured inputs; no filesystem IO.
use crate::{
    model::{Analysis, Coverage, Document, Snapshot, WorkspaceId},
    policy::Classification,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};
use tree_sitter::{Node, Parser};

mod package;

const DOCUMENTS: usize = 8192;
const STATES: usize = 4096;
const SOURCE_BYTES: usize = 1024 * 1024;
const TOTAL_BYTES: usize = 8 * SOURCE_BYTES;
const MANIFEST_BYTES: usize = 128 * 1024;

/// Supported language and provenance pair with its presentation text.
#[derive(Clone, Copy, Serialize)]
pub struct ModuleKind {
    language: &'static str,
    basis: &'static str,
    tooltip: &'static str,
}
impl ModuleKind {
    fn describe(self, id: String, label: String, root: String) -> Module {
        Module {
            id,
            label,
            root,
            language: self.language,
            basis: self.basis,
        }
    }
}

/// Describes module kinds emitted by native resolution.
pub fn module_kinds() -> impl Iterator<Item = ModuleKind> {
    MODULE_RULES.iter().map(|rule| rule.kind)
}

/// Acquires bounded evidence from the same immutable revision as a selected document.
pub trait ModuleEvidence {
    /// Captures the selected document and the classified inputs admitted by the file limits.
    fn captured_files(
        &self,
        selected: &Arc<Document>,
        file_limit: fn(&str, &Classification) -> Option<usize>,
    ) -> Option<Snapshot>;

    /// Reports the requested paths that exist as regular files in the revision.
    fn regular_paths(&self, paths: &mut dyn Iterator<Item = String>) -> Option<BTreeSet<String>>;

    /// Captures one regular file from the revision, subject to the source policy and byte limit.
    fn captured_file(&self, path: &str, limit: usize) -> Option<Document>;
}

type FactResolver = fn(&ModuleIndex, &Document, &Analysis, usize) -> Option<Module>;
type HistoryResolver = fn(&WorkspaceId, &Arc<Document>, &dyn ModuleEvidence) -> Option<ModuleIndex>;

struct ModuleRule {
    languages: &'static [&'static str],
    kind: ModuleKind,
    fact: FactResolver,
    history: Option<HistoryResolver>,
}

const RUST: ModuleRule = ModuleRule {
    languages: &["rust"],
    kind: ModuleKind {
        language: "rust",
        basis: "rust_declarations",
        tooltip: "Rust module",
    },
    fact: ModuleIndex::rust_fact,
    history: Some(ModuleIndex::rust_history),
};
const PYTHON: ModuleRule = ModuleRule {
    languages: &["python"],
    kind: ModuleKind {
        language: "python",
        basis: "python_module",
        tooltip: "Python module",
    },
    fact: |index, doc, _, _| index.python.get(doc.path()).cloned(),
    history: Some(ModuleIndex::python_history),
};
const RUBY: ModuleRule = ModuleRule {
    languages: &["ruby"],
    kind: ModuleKind {
        language: "ruby",
        basis: "ruby_declarations",
        tooltip: "Ruby module",
    },
    fact: |index, _, analysis, ordinal| index.ruby_fact(analysis, ordinal),
    history: None,
};
const MODULE_RULES: &[&ModuleRule] = &[&RUST, &PYTHON, &RUBY, &package::RULE];

fn module_rule(language: &str) -> Option<&'static ModuleRule> {
    MODULE_RULES
        .iter()
        .copied()
        .find(|rule| rule.languages.contains(&language))
}

/// Language-aware source identity, independent of source versions or presentation groups.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Module {
    /// Stable opaque identity within the source workspace.
    pub id: String,
    /// Qualified declared name, bounded to 512 UTF-8 bytes.
    pub label: String,
    /// Language supplying these semantics.
    pub language: &'static str,
    /// Captured target/module root, used to disambiguate equal labels.
    pub root: String,
    /// Capability/provenance, not configured compilation or import resolution.
    pub basis: &'static str,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Base {
    seed: String,
    label: String,
    root: String,
    names: Vec<String>,
}
#[derive(Clone)]
struct Edge {
    scope: Vec<String>,
    target: Target,
}
#[derive(Clone)]
enum Target {
    Module(String, Option<String>),
    Include(String),
}
/// Bounded metadata cache; no trees, source bytes or retired snapshots are retained.
#[derive(Default)]
pub struct ModuleCache {
    rust: BTreeMap<String, (String, Vec<Edge>)>,
}
/// Module bindings for one immutable source observation.
pub struct ModuleIndex {
    workspace: String,
    rust: BTreeMap<String, Option<Base>>,
    python: BTreeMap<String, Module>,
    packages: BTreeMap<String, Arc<Module>>,
}

fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(p, _)| p)
}
fn join(base: &str, path: &str) -> Option<String> {
    if path.starts_with('/') || path.contains(['\\', ':', '\0', '\n', '\r']) {
        return None;
    }
    let mut parts = base
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    for p in path.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(p),
        }
    }
    let result = parts.join("/");
    (!result.is_empty() && result.len() <= 4096).then_some(result)
}
fn name(value: &str) -> String {
    value.strip_prefix("r#").unwrap_or(value).into()
}
fn identifier(value: &str) -> bool {
    let mut c = value.chars();
    c.next()
        .is_some_and(|v| v == '_' || v.is_ascii_alphabetic())
        && c.all(|v| v == '_' || v.is_ascii_alphanumeric())
        && value.len() <= 128
}

fn python_ancestors(path: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(path.rsplit_once('/').map(|(dir, _)| dir), |dir| {
        dir.rsplit_once('/').map(|(parent, _)| parent)
    })
}

fn python_initializer(dir: &str) -> String {
    format!("{dir}/__init__.py")
}

fn python_module(
    workspace: &str,
    path: &str,
    has_initializer: impl Fn(&str) -> bool,
) -> Option<Module> {
    let file = path.rsplit('/').next()?;
    let stem = file
        .strip_suffix(".py")
        .or_else(|| file.strip_suffix(".pyi"))?;
    let mut names = Vec::new();
    if stem != "__init__" {
        names.push(stem);
    }
    for dir in python_ancestors(path).take_while(|dir| has_initializer(&python_initializer(dir))) {
        names.push(dir.rsplit('/').next()?);
    }
    if names.is_empty() || !names.iter().all(|n| identifier(n)) {
        return None;
    }
    names.reverse();
    let label = names.join(".");
    (label.len() <= 512).then(|| {
        PYTHON.kind.describe(
            blake3::hash(format!("{workspace}\0{}\0{path}", PYTHON.kind.language).as_bytes())
                .to_hex()
                .to_string(),
            label,
            path.into(),
        )
    })
}

fn cargo_manifest(path: &str) -> bool {
    path.rsplit('/').next() == Some("Cargo.toml")
}
fn literal(value: &str) -> Option<String> {
    let value = value.trim();
    let body = if let Some(rest) = value.strip_prefix('r') {
        let hashes = rest.bytes().take_while(|b| *b == b'#').count();
        let quoted = rest.get(hashes..)?;
        let suffix = format!("\"{}", "#".repeat(hashes));
        quoted.strip_prefix('"')?.strip_suffix(&suffix)?
    } else {
        value.strip_prefix('"')?.strip_suffix('"')?
    };
    (!body.contains(['\\', '"', '\n', '\r', '\0'])).then(|| body.to_owned())
}
fn attribute_path(attrs: &[&str]) -> Option<Option<String>> {
    let mut path = None;
    for a in attrs {
        let body = a.strip_prefix("#[")?.strip_suffix(']')?.trim();
        if body.starts_with("cfg_attr") {
            return None;
        }
        if let Some(rest) = body.strip_prefix("path") {
            if path.is_some() {
                return None;
            }
            path = Some(literal(rest.trim_start().strip_prefix('=')?)?);
        }
    }
    Some(path)
}
fn declarations(node: Node<'_>, text: &str, scope: &[String], out: &mut Vec<Edge>) {
    if scope.len() > 16 || out.len() >= 512 {
        return;
    }
    let mut cursor = node.walk();
    let mut attrs = vec![];
    for child in node.named_children(&mut cursor) {
        let child = if child.kind() == "expression_statement" {
            child
                .named_child(0)
                .filter(|n| n.kind() == "macro_invocation")
                .unwrap_or(child)
        } else {
            child
        };
        let Some(value) = text.get(child.byte_range()) else {
            continue;
        };
        match child.kind() {
            "attribute_item" => {
                attrs.push(value);
                continue;
            }
            "line_comment" | "block_comment" => continue,
            "mod_item" => {
                if let (Some(n), Some(path)) = (
                    child
                        .child_by_field_name("name")
                        .and_then(|n| text.get(n.byte_range())),
                    attribute_path(&attrs),
                ) {
                    if let Some(body) = child.child_by_field_name("body") {
                        if path.is_none() {
                            let mut nested = scope.to_vec();
                            nested.push(name(n));
                            declarations(body, text, &nested, out);
                        }
                    } else if out.len() < 512 {
                        out.push(Edge {
                            scope: scope.to_vec(),
                            target: Target::Module(name(n), path),
                        });
                    }
                }
            }
            "macro_invocation" => {
                if child
                    .child_by_field_name("macro")
                    .and_then(|n| text.get(n.byte_range()))
                    == Some("include")
                    && let Some(start) = value.find('(')
                    && let Some(args) = value[start + 1..].strip_suffix(')')
                    && let Some(path) = literal(args.trim().trim_end_matches(','))
                    && out.len() < 512
                {
                    out.push(Edge {
                        scope: scope.to_vec(),
                        target: Target::Include(path),
                    });
                }
            }
            _ => {}
        }
        attrs.clear();
    }
}
impl ModuleCache {
    fn edges(&mut self, doc: &Document, budget: &mut usize) -> Vec<Edge> {
        if let Some((version, edges)) = self.rust.get(doc.path())
            && version == doc.version().as_str()
        {
            return edges.clone();
        }
        let mut edges = vec![];
        if let Some(text) = doc
            .resident_source()
            .filter(|s| s.len() <= SOURCE_BYTES && s.len() <= *budget)
        {
            *budget -= text.len();
            if text.contains("mod") || text.contains("include") {
                let mut parser = Parser::new();
                if parser
                    .set_language(&tree_sitter_rust::LANGUAGE.into())
                    .is_ok()
                    && let Some(tree) = parser.parse(text, None)
                    && !tree.root_node().has_error()
                {
                    declarations(tree.root_node(), text, &[], &mut edges);
                }
            }
        }
        self.rust.insert(
            doc.path().into(),
            (doc.version().as_str().into(), edges.clone()),
        );
        edges
    }
}
fn root_targets(
    path: &str,
    text: &str,
    docs: &BTreeMap<&str, &Document>,
) -> Vec<(String, String, String)> {
    let Ok(value) = text.parse::<toml::Table>() else {
        return vec![];
    };
    let Some(package) = value.get("package").and_then(toml::Value::as_table) else {
        return vec![];
    };
    let Some(package_name) = package.get("name").and_then(toml::Value::as_str) else {
        return vec![];
    };
    let crate_name = package_name.replace('-', "_");
    if !identifier(&crate_name)
        || ["autolib", "autobins"]
            .iter()
            .any(|k| package.get(*k).is_some_and(|v| !v.is_bool()))
        || value.get("lib").is_some_and(|v| !v.is_table())
        || value.get("bin").is_some_and(|v| !v.is_array())
    {
        return vec![];
    }
    let edition = package
        .get("edition")
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            if package
                .get("edition")
                .and_then(|v| v.get("workspace"))
                .and_then(toml::Value::as_bool)
                != Some(true)
            {
                return None;
            }
            let mut dir = parent(path);
            loop {
                let candidate = if dir.is_empty() {
                    "Cargo.toml".to_owned()
                } else {
                    format!("{dir}/Cargo.toml")
                };
                if let Some(text) = docs
                    .get(candidate.as_str())
                    .and_then(|d| d.resident_source())
                    .filter(|s| s.len() <= 128 * 1024)
                    && let Ok(v) = text.parse::<toml::Table>()
                    && let Some(e) = v
                        .get("workspace")
                        .and_then(|v| v.get("package"))
                        .and_then(|v| v.get("edition"))
                        .and_then(toml::Value::as_str)
                {
                    return Some(e.to_owned());
                }
                if dir.is_empty() {
                    return None;
                }
                dir = parent(dir);
            }
        });
    let manual = ["lib", "bin", "example", "test", "bench"]
        .iter()
        .any(|k| value.contains_key(*k));
    let auto_bins = package
        .get("autobins")
        .and_then(toml::Value::as_bool)
        .unwrap_or(!manual || matches!(edition.as_deref(), Some("2018" | "2021" | "2024")));
    let directory = parent(path);
    let mut roots = vec![];
    let mut add = |kind: &str, label: &str, relative: &str| {
        let label = label.replace('-', "_");
        if identifier(&label)
            && let Some(p) = join(directory, relative).filter(|p| docs.contains_key(p.as_str()))
        {
            roots.push((p, label, kind.to_owned()));
        }
    };
    if let Some(lib) = value.get("lib").and_then(toml::Value::as_table) {
        if ["name", "path"]
            .iter()
            .any(|k| lib.get(*k).is_some_and(|v| !v.is_str()))
        {
            return vec![];
        }
        add(
            "lib",
            lib.get("name")
                .and_then(toml::Value::as_str)
                .unwrap_or(&crate_name),
            lib.get("path")
                .and_then(toml::Value::as_str)
                .unwrap_or("src/lib.rs"),
        );
    } else if package.get("autolib").and_then(toml::Value::as_bool) != Some(false) {
        add("lib", &crate_name, "src/lib.rs");
    }
    let mut explicit_names = BTreeSet::new();
    if let Some(bins) = value.get("bin").and_then(toml::Value::as_array) {
        for bin in bins {
            let Some(label) = bin.get("name").and_then(toml::Value::as_str) else {
                return vec![];
            };
            if bin.get("path").is_some_and(|v| !v.is_str()) {
                return vec![];
            }
            explicit_names.insert(label.to_owned());
            if let Some(path) = bin.get("path").and_then(toml::Value::as_str) {
                add("bin", label, path);
            } else {
                let mut candidates = vec![
                    format!("src/bin/{label}.rs"),
                    format!("src/bin/{label}/main.rs"),
                ];
                if label == package_name {
                    candidates.push("src/main.rs".into());
                }
                candidates
                    .retain(|p| join(directory, p).is_some_and(|p| docs.contains_key(p.as_str())));
                if candidates.len() == 1 {
                    add("bin", label, &candidates[0]);
                }
            }
        }
    }
    if auto_bins {
        let mut candidates: BTreeMap<String, Vec<String>> = BTreeMap::new();
        if join(directory, "src/main.rs").is_some_and(|p| docs.contains_key(p.as_str())) {
            candidates
                .entry(package_name.into())
                .or_default()
                .push("src/main.rs".into());
        }
        let prefix = if directory.is_empty() {
            "src/bin/".into()
        } else {
            format!("{directory}/src/bin/")
        };
        for candidate in docs.keys().filter_map(|p| p.strip_prefix(&prefix)) {
            let label = candidate
                .strip_suffix("/main.rs")
                .filter(|p| !p.contains('/'))
                .or_else(|| candidate.strip_suffix(".rs").filter(|p| !p.contains('/')));
            if let Some(label) = label {
                candidates
                    .entry(label.into())
                    .or_default()
                    .push(format!("src/bin/{candidate}"));
            }
        }
        for (label, paths) in candidates {
            if !explicit_names.contains(&label) && paths.len() == 1 {
                add("bin", &label, &paths[0]);
            }
        }
    }
    roots
}
impl ModuleIndex {
    /// Builds only from captured resident text/metadata, reusing unchanged declaration edges.
    pub fn new(snapshot: &Snapshot, cache: &mut ModuleCache) -> Self {
        Self::build(snapshot, cache, |doc| {
            doc.analyses()
                .iter()
                .any(|a| a.context.language == RUST.kind.language)
        })
    }

    /// Resolves a historical file from its declarations and any required revision evidence.
    pub fn for_history(
        workspace: &WorkspaceId,
        selected: &Arc<Document>,
        language: &str,
        evidence: &dyn ModuleEvidence,
    ) -> Self {
        module_rule(language)
            .and_then(|rule| rule.history)
            .and_then(|resolve| resolve(workspace, selected, evidence))
            .unwrap_or_else(|| Self::empty(workspace))
    }

    fn rust_history(
        _: &WorkspaceId,
        selected: &Arc<Document>,
        evidence: &dyn ModuleEvidence,
    ) -> Option<Self> {
        let snapshot = evidence.captured_files(selected, |path, classification| {
            if cargo_manifest(path) {
                Some(MANIFEST_BYTES)
            } else {
                (classification.assigned_language() == Some(RUST.kind.language))
                    .then_some(SOURCE_BYTES)
            }
        })?;
        Some(
            Self::build(&snapshot, &mut ModuleCache::default(), |doc| {
                doc.classification().assigned_language() == Some(RUST.kind.language)
            })
            .for_file(selected.path()),
        )
    }

    fn python_history(
        workspace: &WorkspaceId,
        selected: &Arc<Document>,
        evidence: &dyn ModuleEvidence,
    ) -> Option<Self> {
        let mut index = Self::empty(workspace);
        let path = selected.path();
        let initializers =
            evidence.regular_paths(&mut python_ancestors(path).map(python_initializer))?;
        if let Some(module) = python_module(&index.workspace, path, |p| initializers.contains(p)) {
            index.python.insert(path.into(), module);
        }
        Some(index)
    }

    /// Creates an index without file-level bindings.
    pub fn empty(workspace: &WorkspaceId) -> Self {
        Self {
            workspace: workspace.as_str().into(),
            rust: BTreeMap::new(),
            python: BTreeMap::new(),
            packages: BTreeMap::new(),
        }
    }

    /// Retains the module bindings needed to project declarations in one captured file.
    pub fn for_file(&self, path: &str) -> Self {
        Self {
            workspace: self.workspace.clone(),
            rust: self
                .rust
                .get_key_value(path)
                .map(|(k, v)| (k.clone(), v.clone()))
                .into_iter()
                .collect(),
            python: self
                .python
                .get_key_value(path)
                .map(|(k, v)| (k.clone(), v.clone()))
                .into_iter()
                .collect(),
            packages: self
                .packages
                .get_key_value(path)
                .map(|(k, v)| (k.clone(), v.clone()))
                .into_iter()
                .collect(),
        }
    }

    fn build(
        snapshot: &Snapshot,
        cache: &mut ModuleCache,
        is_rust: impl Fn(&Document) -> bool,
    ) -> Self {
        let docs = snapshot
            .documents()
            .iter()
            .take(DOCUMENTS)
            .map(|d| (d.path(), d.as_ref()))
            .collect::<BTreeMap<_, _>>();
        cache.rust.retain(|p, _| docs.contains_key(p.as_str()));
        let mut rust = BTreeMap::new();
        let mut python = BTreeMap::new();
        let mut budget = TOTAL_BYTES;
        let mut queue = VecDeque::new();
        for (path, doc) in &docs {
            if cargo_manifest(path)
                && let Some(text) = doc.resident_source().filter(|s| s.len() <= MANIFEST_BYTES)
            {
                for (file, label, kind) in root_targets(path, text, &docs) {
                    let seed = format!(
                        "{}\0{}\0{path}\0{kind}\0{file}\0{label}",
                        snapshot.workspace().as_str(),
                        RUST.kind.language
                    );
                    let base = Base {
                        seed,
                        label,
                        root: file.clone(),
                        names: vec![],
                    };
                    let dir = parent(&file).to_owned();
                    queue.push_back((file, base, dir, true, 0));
                }
            }
            if doc
                .analyses()
                .iter()
                .any(|a| a.context.language == PYTHON.kind.language)
                && let Some(module) = python_module(snapshot.workspace().as_str(), path, |p| {
                    docs.contains_key(p)
                })
            {
                python.insert((*path).into(), module);
            }
        }
        let mut seen = BTreeSet::new();
        while let Some((file, base, dir, follow, depth)) = queue.pop_front() {
            if depth > 32 || !seen.insert((file.clone(), base.seed.clone(), base.names.clone())) {
                continue;
            }
            if seen.len() > STATES {
                rust.clear();
                break;
            }
            let Some(doc) = docs.get(file.as_str()).filter(|doc| is_rust(doc)) else {
                continue;
            };
            rust.entry(file.clone())
                .and_modify(|old: &mut Option<Base>| {
                    if old.as_ref() != Some(&base) {
                        *old = None;
                    }
                })
                .or_insert_with(|| Some(base.clone()));
            if !follow {
                continue;
            }
            for edge in cache.edges(doc, &mut budget) {
                let mut next = base.clone();
                next.names.extend(edge.scope.iter().cloned());
                let scoped = if edge.scope.is_empty() {
                    Some(dir.clone())
                } else {
                    join(&dir, &edge.scope.join("/"))
                };
                match edge.target {
                    Target::Module(n, path) => {
                        let target = if let Some(path) = path {
                            let origin = if edge.scope.is_empty() {
                                parent(&file).to_owned()
                            } else {
                                scoped.clone().unwrap_or_default()
                            };
                            join(&origin, &path).filter(|p| docs.contains_key(p.as_str()))
                        } else {
                            scoped.as_deref().and_then(|dir| {
                                let a = join(dir, &format!("{n}.rs"))?;
                                let b = join(dir, &format!("{n}/mod.rs"))?;
                                match (docs.contains_key(a.as_str()), docs.contains_key(b.as_str()))
                                {
                                    (true, false) => Some(a),
                                    (false, true) => Some(b),
                                    _ => None,
                                }
                            })
                        };
                        if let Some(target) = target {
                            next.names.push(n);
                            let next_dir = if target.ends_with("/mod.rs") {
                                parent(&target).to_owned()
                            } else {
                                target.strip_suffix(".rs").unwrap_or(&target).to_owned()
                            };
                            queue.push_back((target, next, next_dir, true, depth + 1));
                        }
                    }
                    Target::Include(path) => {
                        if let Some(target) =
                            join(parent(&file), &path).filter(|p| docs.contains_key(p.as_str()))
                        {
                            queue.push_back((target, next, dir.clone(), false, depth + 1));
                        }
                    }
                }
            }
        }
        Self {
            workspace: snapshot.workspace().as_str().into(),
            rust,
            python,
            packages: package::bindings(snapshot, docs.into_values()),
        }
    }
    /// Finds the enclosing language module; types and implementations are not modules.
    pub fn for_fact(&self, doc: &Document, analysis: &Analysis, ordinal: usize) -> Option<Module> {
        if analysis.coverage != Coverage::Syntax {
            return None;
        }
        (module_rule(&analysis.context.language)?.fact)(self, doc, analysis, ordinal)
    }

    fn ruby_fact(&self, analysis: &Analysis, ordinal: usize) -> Option<Module> {
        let mut modules = vec![];
        let mut found_module = false;
        let mut next = Some(ordinal);
        for _ in 0..16 {
            let Some(n) = next else { break };
            let fact = analysis.facts.get(n)?;
            if fact.kind == "module" || (found_module && fact.kind == "class") {
                found_module |= fact.kind == "module";
                modules.push(fact.name.trim_start_matches("::"));
                if fact.name.starts_with("::") {
                    next = None;
                    break;
                }
            }
            next = fact.parent;
        }
        if next.is_some() || !found_module {
            return None;
        }
        modules.reverse();
        let label = modules.join("::");
        if label.len() > 512 {
            return None;
        }
        let root = label.split("::").next()?.to_owned();
        let id =
            blake3::hash(format!("{}\0{}\0{label}", self.workspace, RUBY.kind.language).as_bytes())
                .to_hex()
                .to_string();
        Some(RUBY.kind.describe(id, label, root))
    }

    fn rust_fact(&self, doc: &Document, analysis: &Analysis, ordinal: usize) -> Option<Module> {
        let base = self.rust.get(doc.path())?.as_ref()?;
        let mut names = vec![];
        let mut parent = analysis.facts.get(ordinal)?.parent;
        for _ in 0..16 {
            let Some(n) = parent else {
                break;
            };
            let fact = analysis.facts.get(n)?;
            if fact.kind == "module" {
                names.push(name(&fact.name));
            } else if fact.kind == "function" && !names.is_empty() {
                return None;
            }
            parent = fact.parent;
        }
        if parent.is_some() {
            return None;
        }
        names.reverse();
        let mut parts = vec![base.label.clone()];
        parts.extend(base.names.iter().cloned());
        parts.extend(names);
        let label = parts.join("::");
        if label.len() > 512 {
            return None;
        }
        let id = blake3::hash(format!("{}\0{label}", base.seed).as_bytes())
            .to_hex()
            .to_string();
        Some(RUST.kind.describe(id, label, base.root.clone()))
    }
}
