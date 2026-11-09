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
    root: PathBuf,
    cache: PathBuf,
    executable: PathBuf,
    state: Mutex<Option<Workspace>>,
}
impl RustAnalyzer {
    /// Configures an explicitly enabled provider without starting project tooling.
    pub fn new(root: PathBuf, cache: PathBuf, executable: PathBuf) -> Self {
        Self {
            root,
            cache,
            executable,
            state: Mutex::new(None),
        }
    }
}
struct Workspace {
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
            write(directory.path(), path, bytes)?;
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
            "readonly-rust-analyzer-definitions-v1",
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
        hash.finalize().to_hex().to_string()
    }
}
impl Provider for RustAnalyzer {
    fn resolve(&self, snapshot: Arc<Snapshot>, call: &CallLookup) -> Result<Answer, String> {
        if call.language != "rust" {
            return Err("No definition provider for this language".into());
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let source = &call.invocation.source;
        let doc = snapshot
            .file(&source.file)
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
        if !workspace.opened.contains_key(&source.path) {
            workspace.open(&source.path, &caller)?;
        }
        let definitions = workspace
            .client
            .definitions(&workspace.directory.path().join(&source.path), position)?;
        if digest(&inputs(&self.root, &snapshot)?) != configuration {
            return Err("Project configuration changed during definition lookup".into());
        }
        let mut locations = vec![];
        let mut outside = 0;
        for (path, position) in definitions {
            if let Some(path) = path
                && let Ok(relative) = path.strip_prefix(workspace.directory.path())
            {
                let path = relative
                    .to_str()
                    .ok_or("Definition path is not UTF-8")?
                    .replace('\\', "/");
                relative_path(&path)?;
                locations.push(Location { path, position });
            } else {
                outside += 1;
            }
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
