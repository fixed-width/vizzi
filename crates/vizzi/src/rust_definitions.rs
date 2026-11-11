//! Opt-in rust-analyzer definitions over a private captured-source mirror.
use crate::{
    definitions::{self, Answer, Location, Provider},
    lsp::Client,
};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use vizzi_core::{model::Snapshot, structure::CallLookup};
const FILE_BYTES: usize = 8 * 1024 * 1024;
const WORKSPACE_BYTES: usize = 128 * 1024 * 1024;

/// One lazy, serialized language-server session; ordinary indexing remains independent.
pub struct RustAnalyzer {
    sources: crate::dependency_sources::Catalog,
    root: PathBuf,
    cache: PathBuf,
    executable: PathBuf,
    state: Mutex<Option<Workspace>>,
}
impl RustAnalyzer {
    /// Configures an explicitly enabled provider without starting project tooling.
    pub fn new(root: PathBuf, cache: PathBuf, executable: PathBuf) -> Self {
        Self {
            sources: crate::dependency_sources::Catalog::new(
                cache.join("dependency-references.json"),
            ),
            root,
            cache,
            executable,
            state: Mutex::new(None),
        }
    }
}
struct Workspace {
    external_opened: BTreeMap<PathBuf, String>,
    directory: tempfile::TempDir,
    client: Client,
    configuration: String,
    files: BTreeMap<String, String>,
    opened: BTreeMap<String, i32>,
    next: i32,
}
fn relative(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.contains('\\')
        || path.split('/').any(|p| matches!(p, "" | "." | ".."))
    {
        Err("Invalid captured project path".into())
    } else {
        Ok(())
    }
}
fn inputs(root: &Path, snapshot: &Snapshot) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut directories = BTreeSet::from([String::new()]);
    for doc in snapshot.documents() {
        if rust(doc) {
            let mut path = doc.path();
            while let Some((parent, _)) = path.rsplit_once('/') {
                directories.insert(parent.to_owned());
                path = parent;
            }
        }
    }
    let mut values = BTreeMap::new();
    let mut total = 0;
    for dir in directories {
        for name in [
            "Cargo.toml",
            "Cargo.lock",
            ".cargo/config.toml",
            ".cargo/config",
        ] {
            let path = if dir.is_empty() {
                name.into()
            } else {
                format!("{dir}/{name}")
            };
            relative(&path)?;
            if name == "Cargo.toml" {
                if let Some(doc) = snapshot.file_by_path(&path) {
                    let bytes = doc
                        .captured()
                        .read_all(1024 * 1024)
                        .map_err(|e| e.to_string())?
                        .bytes()
                        .to_vec();
                    total += bytes.len();
                    if total > 16 * 1024 * 1024 {
                        return Err("Project configuration exceeds semantic allowance".into());
                    }
                    values.insert(path, bytes);
                }
                continue;
            }
            let absolute = root.join(&path);
            let metadata = match fs::symlink_metadata(&absolute) {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.to_string()),
            };
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err("Project inputs must be regular files".into());
            }
            let mut bytes = vec![];
            fs::File::open(absolute)
                .map_err(|e| e.to_string())?
                .take(1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            total += bytes.len();
            if bytes.len() > 1024 * 1024 || total > 16 * 1024 * 1024 {
                return Err("Project configuration exceeds semantic allowance".into());
            }
            values.insert(path, bytes);
        }
    }
    if !values.contains_key("Cargo.toml") {
        return Err("Type-aware Rust navigation needs a Cargo project root".into());
    }
    Ok(values)
}
fn digest(values: &BTreeMap<String, Vec<u8>>) -> String {
    let mut hash = blake3::Hasher::new();
    for (path, bytes) in values {
        hash.update(&(path.len() as u64).to_le_bytes());
        hash.update(path.as_bytes());
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    hash.finalize().to_hex().to_string()
}
fn rust(doc: &vizzi_core::model::Document) -> bool {
    doc.analyses().iter().any(|a| a.context.language == "rust")
}
fn text(doc: &vizzi_core::model::Document) -> Result<String, String> {
    doc.captured()
        .read_text(FILE_BYTES)
        .map_err(|e| e.to_string())?
        .map(|s| s.into_owned())
        .ok_or_else(|| "Captured Rust source is not UTF-8".into())
}
fn write(root: &Path, path: &str, bytes: &[u8]) -> Result<(), String> {
    relative(path)?;
    let destination = root.join(path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(destination, bytes).map_err(|e| e.to_string())
}
impl Workspace {
    fn open_external(&mut self, path: &Path, source: &str, version: &str) -> Result<bool, String> {
        if self.external_opened.get(path).is_some_and(|v| v == version) {
            return Ok(false);
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or("Semantic document versions exhausted")?;
        let uri = crate::lsp::uri(path)?;
        if self
            .external_opened
            .insert(path.into(), version.into())
            .is_some()
        {
            self.client.notify("textDocument/didChange",json!({"textDocument":{"uri":uri,"version":self.next},"contentChanges":[{"text":source}]}))?;
        } else {
            self.client.notify("textDocument/didOpen",json!({"textDocument":{"uri":uri,"languageId":"rust","version":self.next,"text":source}}))?;
        }
        Ok(true)
    }
    fn new(
        owner: &RustAnalyzer,
        snapshot: &Snapshot,
        values: &BTreeMap<String, Vec<u8>>,
        configuration: String,
    ) -> Result<Self, String> {
        fs::create_dir_all(&owner.cache).map_err(|e| e.to_string())?;
        let directory = tempfile::Builder::new()
            .prefix("definitions-")
            .tempdir_in(&owner.cache)
            .map_err(|e| e.to_string())?;
        for (path, bytes) in values {
            let transformed = if path.ends_with("Cargo.toml") {
                crate::rust_dependencies::manifest(bytes, path, &owner.root, directory.path())?
            } else {
                bytes.clone()
            };
            write(directory.path(), path, &transformed)?;
        }
        let mut files = BTreeMap::new();
        let mut total = 0;
        for doc in snapshot.documents().iter().filter(|d| rust(d)) {
            let source = text(doc)?;
            total += source.len();
            if total > WORKSPACE_BYTES {
                return Err("Captured Rust workspace exceeds semantic allowance".into());
            }
            write(directory.path(), doc.path(), source.as_bytes())?;
            files.insert(doc.path().into(), doc.version().as_str().into());
        }
        let mut client = Client::start(&owner.executable, directory.path())?;
        client.wait_ready()?;
        Ok(Self {
            external_opened: BTreeMap::new(),
            directory,
            client,
            configuration,
            files,
            opened: BTreeMap::new(),
            next: 0,
        })
    }
    fn open(&mut self, path: &str, source: &str) -> Result<(), String> {
        self.next = self
            .next
            .checked_add(1)
            .ok_or("Semantic document versions exhausted")?;
        let uri = crate::lsp::uri(&self.directory.path().join(path))?;
        if self.opened.insert(path.into(), self.next).is_some() {
            self.client.notify("textDocument/didChange",json!({"textDocument":{"uri":uri,"version":self.next},"contentChanges":[{"text":source}]}))
        } else {
            self.client.notify("textDocument/didOpen",json!({"textDocument":{"uri":uri,"languageId":"rust","version":self.next,"text":source}}))
        }
    }
    fn sync(&mut self, snapshot: &Snapshot) -> Result<(), String> {
        let current = snapshot
            .documents()
            .iter()
            .filter(|d| rust(d))
            .map(|d| d.path())
            .collect::<BTreeSet<_>>();
        for path in self
            .files
            .keys()
            .filter(|p| !current.contains(p.as_str()))
            .cloned()
            .collect::<Vec<_>>()
        {
            fs::remove_file(self.directory.path().join(&path)).map_err(|e| e.to_string())?;
            if self.opened.remove(&path).is_some() {
                self.client.notify("textDocument/didClose",json!({"textDocument":{"uri":crate::lsp::uri(&self.directory.path().join(&path))?}}))?;
            }
            self.client.notify("workspace/didChangeWatchedFiles",json!({"changes":[{"uri":crate::lsp::uri(&self.directory.path().join(&path))?,"type":3}]}))?;
            self.files.remove(&path);
        }
        let mut total = 0;
        for doc in snapshot.documents().iter().filter(|d| rust(d)) {
            total += doc.captured().len();
            if total > WORKSPACE_BYTES {
                return Err("Captured Rust workspace exceeds semantic allowance".into());
            }
            if self
                .files
                .get(doc.path())
                .is_some_and(|v| v == doc.version().as_str())
            {
                continue;
            }
            let source = text(doc)?;
            write(self.directory.path(), doc.path(), source.as_bytes())?;
            self.open(doc.path(), &source)?;
            self.files
                .insert(doc.path().into(), doc.version().as_str().into());
        }
        Ok(())
    }
    fn context(&self, snapshot: &Snapshot) -> String {
        let mut hash = blake3::Hasher::new();
        hash.update(include_bytes!("rust_definitions.rs"));
        hash.update(include_bytes!("lsp.rs"));
        hash.update(include_bytes!("definitions.rs"));
        for value in [
            "readonly-rust-analyzer-definitions-v2",
            snapshot.id().as_str(),
            self.configuration.as_str(),
            self.client.version.as_str(),
        ] {
            hash.update(&(value.len() as u64).to_le_bytes());
            hash.update(value.as_bytes());
        }
        if let Ok(bytes) = fs::read(self.directory.path().join("Cargo.lock")) {
            hash.update(&bytes);
        }
        for (path, version) in &self.external_opened {
            let path = path.as_os_str().as_encoded_bytes();
            hash.update(&(path.len() as u64).to_le_bytes());
            hash.update(path);
            hash.update(&(version.len() as u64).to_le_bytes());
            hash.update(version.as_bytes());
        }
        hash.finalize().to_hex().to_string()
    }
}
impl Provider for RustAnalyzer {
    fn references(
        &self,
        snapshot: Arc<Snapshot>,
        symbol: &vizzi_core::structure::Symbol,
        identifier: usize,
    ) -> Result<Answer, String> {
        let external = if let Some(origin) = &symbol.origin {
            self.sources.get(&origin.capture)?
        } else {
            None
        };
        let doc = external
            .as_ref()
            .map(|s| s.doc())
            .or_else(|| snapshot.file(&symbol.source.file).map(AsRef::as_ref))
            .ok_or("Reference target is unavailable")?;
        if !doc.analyses().iter().any(|a| a.context.language == "rust")
            || doc.version().as_str() != symbol.source.version
        {
            return Err("Reference target version or language differs".into());
        }
        let text = text(doc)?;
        if text.get(identifier..identifier + symbol.name.len()) != Some(symbol.name.as_str()) {
            return Err("Reference target identifier differs".into());
        }
        let position = definitions::position(&text, identifier)?;
        let values = inputs(&self.root, &snapshot)?;
        let configuration = digest(&values);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut workspace = match state.take() {
            Some(w) if w.configuration == configuration => w,
            _ => Workspace::new(self, &snapshot, &values, configuration.clone())?,
        };
        workspace.sync(&snapshot)?;
        let path = if let Some(source) = &external {
            if source.input.configuration != configuration {
                return Err("Dependencies changed; reopen the target from its caller".into());
            }
            source.verify()?;
            workspace.open_external(&source.path(), &text, doc.version().as_str())?;
            source.path()
        } else {
            if !workspace.opened.contains_key(&symbol.source.path) {
                workspace.open(&symbol.source.path, &text)?;
            }
            workspace.directory.path().join(&symbol.source.path)
        };
        let captures = self.sources.sources()?;
        for source in &captures {
            if source.input.configuration == configuration && source.verify().is_ok() {
                workspace.open_external(
                    &source.path(),
                    &crate::rust_definitions::text(source.doc())?,
                    source.doc().version().as_str(),
                )?;
            }
        }
        let raw = workspace.client.references(&path, position)?;
        let mut locations = vec![];
        let mut outside = 0;
        for (path, position) in raw {
            let Some(path) = path.map(|p| fs::canonicalize(&p).unwrap_or(p)) else {
                outside += 1;
                continue;
            };
            if let Ok(relative) = path.strip_prefix(workspace.directory.path()) {
                locations.push(Location {
                    capture: None,
                    path: relative
                        .to_str()
                        .ok_or("Reference path is not UTF-8")?
                        .replace('\\', "/"),
                    position,
                });
            } else if let Some(source) = captures.iter().find(|s| {
                s.path() == path && s.input.configuration == configuration && s.verify().is_ok()
            }) {
                locations.push(Location {
                    capture: Some(source.origin.capture.clone()),
                    path: source.doc().path().into(),
                    position,
                });
            } else {
                outside += 1;
            }
        }
        if digest(&inputs(&self.root, &snapshot)?) != configuration {
            return Err("Project configuration changed during reference lookup".into());
        }
        let context = workspace.context(&snapshot);
        *state = Some(workspace);
        Ok(Answer {
            locations,
            outside,
            context,
        })
    }
    fn dependencies(&self) -> Option<&crate::dependency_sources::Catalog> {
        Some(&self.sources)
    }
    fn resolve(&self, snapshot: Arc<Snapshot>, call: &CallLookup) -> Result<Answer, String> {
        if call.language != "rust" {
            return Err("No definition provider for this language".into());
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let source = &call.invocation.source;
        let external = if snapshot.file(&source.file).is_none() {
            self.sources
                .sources()?
                .into_iter()
                .find(|s| s.index.call(&call.invocation.id).is_some())
        } else {
            None
        };
        let doc = external
            .as_ref()
            .map(|s| s.doc())
            .or_else(|| snapshot.file(&source.file).map(AsRef::as_ref))
            .ok_or("Captured caller is absent")?;
        if doc.version().as_str() != source.version {
            return Err("Captured caller version differs".into());
        }
        let identifier = call
            .identifier
            .as_ref()
            .ok_or("Captured identifier is unavailable; reindex source")?;
        let caller = text(doc)?;
        if caller.get(identifier.clone()) != call.invocation.name.as_deref() {
            return Err("Captured identifier witness differs".into());
        }
        let position = definitions::position(&caller, identifier.start)?;
        let values = inputs(&self.root, &snapshot)?;
        let configuration = digest(&values);
        let mut workspace = match state.take() {
            Some(w) if w.configuration == configuration => w,
            _ => Workspace::new(self, &snapshot, &values, configuration.clone())?,
        };
        workspace.sync(&snapshot)?;
        if external.is_none() && !workspace.opened.contains_key(&source.path) {
            workspace.open(&source.path, &caller)?;
        }
        let caller_path = external
            .as_ref()
            .map(|s| s.path())
            .unwrap_or_else(|| workspace.directory.path().join(&source.path));
        if let Some(source) = &external {
            if source.input.configuration != configuration || source.verify().is_err() {
                let context = workspace.context(&snapshot);
                *state = Some(workspace);
                return Ok(Answer {
                    locations: vec![],
                    outside: 1,
                    context,
                });
            }
            workspace.open_external(&caller_path, &caller, doc.version().as_str())?;
        }
        let mut locations = vec![];
        let mut outside = 0;
        let mut verified = vec![];
        let mut stable = false;
        for _ in 0..3 {
            let definitions = workspace.client.definitions(&caller_path, position)?;
            locations.clear();
            outside = 0;
            verified.clear();
            let mut changed = false;
            for (path, position) in definitions {
                let Some(path) = path.map(|p| fs::canonicalize(&p).unwrap_or(p)) else {
                    outside += 1;
                    continue;
                };
                if let Ok(path) = path.strip_prefix(workspace.directory.path()) {
                    let path = path
                        .to_str()
                        .ok_or("Definition path is not UTF-8")?
                        .replace('\\', "/");
                    relative_path(&path)?;
                    locations.push(Location {
                        capture: None,
                        path,
                        position,
                    });
                    continue;
                }
                if let Ok(relative) = path.strip_prefix(&self.root) {
                    let relative = relative
                        .to_str()
                        .ok_or("Definition path is not UTF-8")?
                        .replace('\\', "/");
                    if let Some(doc) = snapshot.file_by_path(&relative) {
                        changed |=
                            workspace.open_external(&path, &text(doc)?, doc.version().as_str())?;
                        locations.push(Location {
                            capture: None,
                            path: relative,
                            position,
                        });
                        continue;
                    }
                    outside += 1;
                    continue;
                }
                let captured = crate::rust_dependencies::input(&path, &configuration)
                    .and_then(|i| self.sources.capture(i));
                match captured {
                    Ok(capture) => {
                        changed |= workspace.open_external(
                            &capture.path(),
                            &text(capture.doc())?,
                            capture.doc().version().as_str(),
                        )?;
                        locations.push(Location {
                            capture: Some(capture.origin.capture.clone()),
                            path: capture.doc().path().into(),
                            position,
                        });
                        verified.push(capture);
                    }
                    Err(_) => outside += 1,
                }
            }
            if !changed {
                stable = true;
                break;
            }
        }
        if !stable {
            outside += locations.len();
            locations.clear();
        }
        if verified.iter().any(|s| s.verify().is_err()) {
            outside += locations.len();
            locations.clear();
        }
        if digest(&inputs(&self.root, &snapshot)?) != configuration {
            return Err("Project configuration changed during definition lookup".into());
        }
        let retained = self
            .sources
            .sources()?
            .iter()
            .map(|s| s.path())
            .collect::<BTreeSet<_>>();
        for path in workspace
            .external_opened
            .keys()
            .filter(|p| !retained.contains(*p))
            .cloned()
            .collect::<Vec<_>>()
        {
            workspace.client.notify(
                "textDocument/didClose",
                json!({"textDocument":{"uri":crate::lsp::uri(&path)?}}),
            )?;
            workspace.external_opened.remove(&path);
        }
        let context = workspace.context(&snapshot);
        *state = Some(workspace);
        Ok(Answer {
            locations,
            outside,
            context,
        })
    }
}
fn relative_path(path: &str) -> Result<(), String> {
    relative(path)
}

#[cfg(test)]
mod captured_project_inputs {
    use super::*;
    use crate::{
        cache::Cache,
        engine::Engine,
        host::{Host, HostOptions},
        source::Directory,
    };
    #[test]
    fn manifests_follow_the_captured_observation_even_when_working_files_change_or_disappear() {
        let root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='captured'\n",
        )
        .unwrap();
        fs::write(root.path().join("lib.rs"), "fn run(){}\n").unwrap();
        let host = Host::start_with_options(
            Engine::new(
                Directory::open(root.path()).unwrap(),
                Cache::new(cache.path().to_path_buf()),
                1,
            )
            .unwrap(),
            HostOptions {
                watch: false,
                idle_maintenance: false,
                ..Default::default()
            },
        )
        .unwrap();
        let snapshot = host.application().snapshot().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='working'\n",
        )
        .unwrap();
        assert_eq!(
            inputs(root.path(), &snapshot).unwrap()["Cargo.toml"],
            b"[package]\nname='captured'\n"
        );
        fs::remove_file(root.path().join("Cargo.toml")).unwrap();
        assert_eq!(
            inputs(root.path(), &snapshot).unwrap()["Cargo.toml"],
            b"[package]\nname='captured'\n"
        );
    }
}
