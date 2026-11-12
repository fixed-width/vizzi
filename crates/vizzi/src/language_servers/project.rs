//! Captured-source mirror and semantic lookup shared by non-Cargo adapters.
use super::inputs::{Inputs, Profile};
use crate::{
    definitions::{self, Answer, Location, Provider},
    dependency_sources::{Capture, Catalog, Input},
    lsp::Client,
};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use vizzi_core::{
    model::{Document, Snapshot},
    structure::{CallLookup, Symbol},
};

const FILE_BYTES: usize = 8 * 1024 * 1024;
const WORKSPACE_BYTES: usize = 128 * 1024 * 1024;
pub(super) struct ProjectServer {
    root: PathBuf,
    cache: PathBuf,
    executable: PathBuf,
    profile: Profile,
    sources: Arc<Catalog>,
    state: Mutex<Option<Workspace>>,
}
struct Workspace {
    directory: tempfile::TempDir,
    client: Client,
    configuration: String,
    files: BTreeMap<String, String>,
    opened: BTreeSet<PathBuf>,
    next: i32,
}
fn source(doc: &Document) -> Result<String, String> {
    doc.captured()
        .read_text(FILE_BYTES)
        .map_err(|e| e.to_string())?
        .map(|text| text.into_owned())
        .ok_or_else(|| "Captured semantic source is not UTF-8".into())
}
fn language(doc: &Document) -> Option<&str> {
    doc.analyses()
        .first()
        .map(|analysis| analysis.context.language.as_str())
}
fn write(root: &Path, path: &str, bytes: &[u8]) -> Result<(), String> {
    if path.is_empty()
        || path.contains('\\')
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err("Invalid captured project path".into());
    }
    let target = root.join(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(target, bytes).map_err(|e| e.to_string())
}
impl ProjectServer {
    pub(super) fn new(
        root: PathBuf,
        cache: PathBuf,
        executable: PathBuf,
        profile: Profile,
        sources: Arc<Catalog>,
    ) -> Self {
        Self {
            root,
            cache,
            executable,
            profile,
            sources,
            state: Mutex::new(None),
        }
    }
    fn workspace(&self, snapshot: &Snapshot, inputs: &Inputs) -> Result<Workspace, String> {
        fs::create_dir_all(&self.cache).map_err(|e| e.to_string())?;
        let directory = tempfile::Builder::new()
            .prefix("captured-")
            .tempdir_in(&self.cache)
            .map_err(|e| e.to_string())?;
        let mut files = BTreeMap::new();
        let mut total = 0;
        for doc in snapshot
            .documents()
            .iter()
            .filter(|doc| language(doc).is_some_and(|lang| self.profile.accepts(lang)))
        {
            let text = source(doc)?;
            total += text.len();
            if total > WORKSPACE_BYTES {
                return Err("Captured workspace exceeds semantic allowance".into());
            }
            write(directory.path(), doc.path(), text.as_bytes())?;
            files.insert(doc.path().into(), doc.version().as_str().into());
        }
        let setup = inputs.prepare(&self.root, directory.path(), &self.profile)?;
        let client = Client::configured(&self.executable, directory.path(), setup)?;
        Ok(Workspace {
            directory,
            client,
            configuration: inputs.digest(),
            files,
            opened: BTreeSet::new(),
            next: 0,
        })
    }
    fn external_input(
        &self,
        path: &Path,
        configuration: &str,
        hint: &str,
    ) -> Result<Input, String> {
        let path = fs::canonicalize(path).map_err(|e| e.to_string())?;
        let language = match path.extension().and_then(|v| v.to_str()) {
            Some("py" | "pyi") => "python",
            Some("rb" | "rake" | "gemspec" | "ru") => "ruby",
            Some("c") => "c",
            Some("cc" | "cpp" | "cxx" | "hh" | "hpp" | "hxx" | "tcc" | "ipp" | "inl") => "cpp",
            Some("h" | "inc") | None if matches!(hint, "c" | "cpp") => hint,
            _ => return Err("Unsupported external source language".into()),
        };
        if !self.profile.accepts(language) {
            return Err("External source language differs".into());
        }
        let mut root = path.parent().ok_or("External source has no directory")?;
        if language == "ruby" {
            for candidate in path.ancestors().skip(1).take(32) {
                if candidate
                    .parent()
                    .and_then(Path::file_name)
                    .is_some_and(|name| name == "gems")
                {
                    root = candidate;
                    break;
                }
            }
        }
        if language == "python" {
            for _ in 0..16 {
                let Some(parent) = root.parent() else {
                    break;
                };
                if (root.join("__init__.py").is_file() || root.join("__init__.pyi").is_file())
                    && (parent.join("__init__.py").is_file()
                        || parent.join("__init__.pyi").is_file())
                {
                    root = parent;
                    continue;
                }
                break;
            }
        }
        let package = if language == "python" {
            if root.join("__init__.py").is_file() || root.join("__init__.pyi").is_file() {
                root.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Python source")
            } else {
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Python source")
            }
        } else {
            if language == "ruby" {
                root.parent()
                    .and_then(Path::file_name)
                    .filter(|name| *name == "gems")
                    .and_then(|_| root.file_name())
                    .and_then(|name| name.to_str())
                    .unwrap_or("Ruby library source")
            } else {
                "C/C++ source"
            }
        };
        Ok(Input {
            root: root.into(),
            path: path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("External filename is not UTF-8")?
                .replace('\\', "/"),
            language: language.into(),
            package: package.into(),
            version: None,
            configuration: configuration.into(),
        })
    }
    fn lookup(
        &self,
        snapshot: Arc<Snapshot>,
        doc: &Document,
        byte: usize,
        external: Option<&Capture>,
        references: bool,
    ) -> Result<Answer, String> {
        let lang = language(doc).ok_or("Semantic language is unavailable")?;
        if !self.profile.accepts(lang) {
            return Err("Unsupported semantic language".into());
        }
        let text = source(doc)?;
        let position = definitions::position(&text, byte)?;
        let inputs = Inputs::load(&self.root, &self.profile)?;
        let configuration = inputs.digest();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut workspace = match state.take() {
            Some(workspace) if workspace.configuration == configuration => workspace,
            _ => self.workspace(&snapshot, &inputs)?,
        };
        workspace.sync(&snapshot, &self.profile)?;
        let path = if let Some(capture) = external {
            if capture.input.configuration != configuration {
                return Err("Dependency configuration changed; reopen from its caller".into());
            }
            capture.verify()?;
            capture.path()
        } else {
            workspace.directory.path().join(doc.path())
        };
        workspace.open(&path, lang, &text)?;
        let captures = self.sources.sources()?;
        let retained = captures
            .iter()
            .filter(|capture| {
                self.profile.accepts(&capture.input.language)
                    && capture.input.configuration == configuration
                    && capture.verify().is_ok()
            })
            .map(|capture| capture.path())
            .collect::<BTreeSet<_>>();
        workspace.prune_external(&retained)?;
        for capture in captures {
            if self.profile.accepts(&capture.input.language)
                && capture.input.configuration == configuration
                && capture.verify().is_ok()
            {
                workspace.open(
                    &capture.path(),
                    &capture.input.language,
                    &source(capture.doc())?,
                )?;
            }
        }
        let mut locations = vec![];
        let mut outside = 0;
        let mut signatures = 0;
        let mut stable = false;
        for _ in 0..3 {
            let raw = if references {
                workspace.client.references(&path, position)?
            } else {
                workspace.client.definitions(&path, position)?
            };
            locations.clear();
            outside = 0;
            signatures = 0;
            let mut changed = false;
            for (target, position) in raw {
                let Some(target) = target.map(|path| fs::canonicalize(&path).unwrap_or(path))
                else {
                    outside += 1;
                    continue;
                };
                if lang == "ruby"
                    && !references
                    && target
                        .extension()
                        .is_some_and(|extension| extension == "rbs")
                {
                    signatures += 1;
                    continue;
                }
                if let Ok(relative) = target.strip_prefix(workspace.directory.path()) {
                    let relative = relative
                        .to_str()
                        .ok_or("Semantic path is not UTF-8")?
                        .replace('\\', "/");
                    if snapshot.file_by_path(&relative).is_some() {
                        locations.push(Location {
                            capture: None,
                            path: relative,
                            position,
                        });
                    } else {
                        outside += 1;
                    }
                } else {
                    let input = match self.external_input(&target, &configuration, lang) {
                        Ok(input) => input,
                        Err(_) => {
                            outside += 1;
                            continue;
                        }
                    };
                    let captured = match self.sources.capture(input) {
                        Ok(capture) => capture,
                        Err(_) => {
                            outside += 1;
                            continue;
                        }
                    };
                    captured.verify()?;
                    changed |= workspace.open(
                        &captured.path(),
                        &captured.input.language,
                        &source(captured.doc())?,
                    )?;
                    locations.push(Location {
                        capture: Some(captured.origin.capture.clone()),
                        path: captured.doc().path().into(),
                        position,
                    });
                }
            }
            if !changed {
                stable = true;
                break;
            }
        }
        if !stable {
            return Err("Dependency source did not stabilize during semantic lookup".into());
        }
        let mut verified = Vec::with_capacity(locations.len());
        for location in locations {
            if let Some(id) = &location.capture
                && !self
                    .sources
                    .get(id)?
                    .is_some_and(|source| source.verify().is_ok())
            {
                outside += 1;
                continue;
            }
            verified.push(location);
        }
        let mut locations = verified;
        let mut seen_locations = BTreeSet::new();
        locations.retain(|location| {
            seen_locations.insert((
                location.capture.clone(),
                location.path.clone(),
                location.position.line,
                location.position.character,
            ))
        });
        if locations.is_empty() {
            outside += signatures;
        }
        if references && lang == "ruby" {
            let mut checked = Vec::new();
            for location in locations {
                let external = location
                    .capture
                    .as_ref()
                    .map(|id| self.sources.get(id))
                    .transpose()?
                    .flatten();
                let target_doc = external
                    .as_ref()
                    .map(|c| c.doc())
                    .or_else(|| snapshot.file_by_path(&location.path).map(AsRef::as_ref))
                    .ok_or("Ruby reference source expired")?;
                let target_text = source(target_doc)?;
                let byte = definitions::byte_offset(&target_text, location.position)?;
                if target_doc
                    .analyses()
                    .iter()
                    .flat_map(|a| &a.facts)
                    .any(|fact| {
                        fact.name_span
                            .as_ref()
                            .is_some_and(|span| span.contains(&byte))
                    })
                {
                    continue;
                }
                let target_path = external
                    .as_ref()
                    .map(|c| c.path())
                    .unwrap_or_else(|| workspace.directory.path().join(&location.path));
                workspace.open(&target_path, "ruby", &target_text)?;
                let definitions = workspace
                    .client
                    .definitions(&target_path, location.position)?;
                if super::ruby::matches_definition(definitions, &path, position) {
                    checked.push(location);
                }
            }
            locations = checked;
        }
        let retained = self
            .sources
            .sources()?
            .into_iter()
            .filter(|capture| {
                self.profile.accepts(&capture.input.language)
                    && capture.input.configuration == configuration
                    && capture.verify().is_ok()
            })
            .map(|capture| capture.path())
            .collect();
        workspace.prune_external(&retained)?;
        if Inputs::load(&self.root, &self.profile)?.digest() != configuration {
            return Err("Project inputs changed during semantic lookup".into());
        }
        let mut hash = blake3::Hasher::new();
        hash.update(include_bytes!("project.rs"));
        hash.update(include_bytes!("inputs.rs"));
        hash.update(include_bytes!("../lsp.rs"));
        for value in [
            snapshot.id().as_str(),
            &configuration,
            &workspace.client.version,
            workspace
                .directory
                .path()
                .to_str()
                .ok_or("Semantic directory is not UTF-8")?,
        ] {
            hash.update(value.as_bytes());
        }
        for (path, version) in workspace.files.iter().filter(|(path, _)| {
            Path::new(path).is_absolute()
                && !Path::new(path).starts_with(workspace.directory.path())
        }) {
            hash.update(path.as_bytes());
            hash.update(version.as_bytes());
        }
        let context = hash.finalize().to_hex().to_string();
        *state = Some(workspace);
        Ok(Answer {
            locations,
            outside,
            context,
        })
    }
}
impl Provider for ProjectServer {
    fn dependencies(&self) -> Option<&Catalog> {
        Some(&self.sources)
    }
    fn resolve(&self, snapshot: Arc<Snapshot>, call: &CallLookup) -> Result<Answer, String> {
        let external = if snapshot.file(&call.invocation.source.file).is_none() {
            self.sources
                .sources()?
                .into_iter()
                .find(|source| source.index.call(&call.invocation.id).is_some())
        } else {
            None
        };
        let doc = external
            .as_ref()
            .map(|source| source.doc())
            .or_else(|| {
                snapshot
                    .file(&call.invocation.source.file)
                    .map(AsRef::as_ref)
            })
            .ok_or("Captured caller is unavailable")?;
        if doc.version().as_str() != call.invocation.source.version {
            return Err("Captured caller version differs".into());
        }
        let span = call
            .identifier
            .as_ref()
            .ok_or("Captured call identifier is unavailable")?;
        if source(doc)?.get(span.clone()) != call.invocation.name.as_deref() {
            return Err("Captured call identifier differs".into());
        }
        self.lookup(
            snapshot.clone(),
            doc,
            span.start,
            external.as_deref(),
            false,
        )
    }
    fn references(
        &self,
        snapshot: Arc<Snapshot>,
        symbol: &Symbol,
        identifier: usize,
    ) -> Result<Answer, String> {
        let external = symbol
            .origin
            .as_ref()
            .map(|origin| self.sources.get(&origin.capture))
            .transpose()?
            .flatten();
        let doc = external
            .as_ref()
            .map(|source| source.doc())
            .or_else(|| snapshot.file(&symbol.source.file).map(AsRef::as_ref))
            .ok_or("Reference target is unavailable")?;
        if doc.version().as_str() != symbol.source.version {
            return Err("Reference target version differs".into());
        }
        let name = doc
            .analyses()
            .iter()
            .filter(|analysis| analysis.context.id.as_str() == symbol.source.context)
            .flat_map(|analysis| &analysis.facts)
            .find(|fact| {
                fact.kind == symbol.kind
                    && fact
                        .name_span
                        .as_ref()
                        .is_some_and(|span| span.start == identifier)
            })
            .map(|fact| fact.name.as_str())
            .ok_or("Reference declaration is unavailable")?;
        if !name.starts_with(&symbol.name)
            || source(doc)?.get(identifier..identifier.saturating_add(name.len())) != Some(name)
        {
            return Err("Reference target identifier differs".into());
        }
        let offset = definitions::reference_offset(language(doc), name);
        self.lookup(
            snapshot.clone(),
            doc,
            identifier + offset,
            external.as_deref(),
            true,
        )
    }
}
impl Workspace {
    fn prune_external(&mut self, retained: &BTreeSet<PathBuf>) -> Result<(), String> {
        let removed = self
            .opened
            .iter()
            .filter(|path| !path.starts_with(self.directory.path()) && !retained.contains(*path))
            .cloned()
            .collect::<Vec<_>>();
        for path in removed {
            self.client.notify(
                "textDocument/didClose",
                json!({"textDocument":{"uri":crate::lsp::uri(&path)?}}),
            )?;
            self.opened.remove(&path);
            self.files.remove(&path.to_string_lossy().to_string());
        }
        Ok(())
    }
    fn open(&mut self, path: &Path, language: &str, text: &str) -> Result<bool, String> {
        // Versioned text, rather than didOpen count, decides whether an external capture changed.
        let version = blake3::hash(text.as_bytes()).to_hex().to_string();
        let key = path.to_string_lossy().to_string();
        if self.files.get(&key).is_some_and(|old| old == &version) {
            return Ok(false);
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or("Semantic document versions exhausted")?;
        let uri = crate::lsp::uri(path)?;
        // Reopen full captures when the server requires ranged incremental edits.
        let reopen = self.client.reopen_document_sync && self.opened.contains(path);
        if reopen {
            self.client
                .notify("textDocument/didClose", json!({"textDocument":{"uri":uri}}))?;
        }
        if self.opened.insert(path.into()) || reopen {
            self.client.notify("textDocument/didOpen",json!({"textDocument":{"uri":uri,"languageId":language,"version":self.next,"text":text}}))?;
        } else {
            self.client.notify("textDocument/didChange",json!({"textDocument":{"uri":uri,"version":self.next},"contentChanges":[{"text":text}]}))?;
        }
        self.client.wait_document(path, self.next)?;
        self.files.insert(key, version);
        Ok(true)
    }
    fn sync(&mut self, snapshot: &Snapshot, profile: &Profile) -> Result<(), String> {
        let docs = snapshot
            .documents()
            .iter()
            .filter(|doc| language(doc).is_some_and(|lang| profile.accepts(lang)))
            .collect::<Vec<_>>();
        let current = docs.iter().map(|doc| doc.path()).collect::<BTreeSet<_>>();
        let deleted = self
            .files
            .keys()
            .filter(|path| !Path::new(path).is_absolute() && !current.contains(path.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        for path in deleted {
            let absolute = self.directory.path().join(&path);
            fs::remove_file(&absolute).map_err(|e| e.to_string())?;
            if self.opened.remove(&absolute) {
                self.client.notify(
                    "textDocument/didClose",
                    json!({"textDocument":{"uri":crate::lsp::uri(&absolute)?}}),
                )?;
            }
            self.client.notify(
                "workspace/didChangeWatchedFiles",
                json!({"changes":[{"uri":crate::lsp::uri(&absolute)?,"type":3}]}),
            )?;
            self.files.remove(&path);
            self.files.remove(&absolute.to_string_lossy().to_string());
        }
        let mut total = 0;
        for doc in docs {
            total += doc.captured().len();
            if total > WORKSPACE_BYTES {
                return Err("Captured workspace exceeds semantic allowance".into());
            }
            if self
                .files
                .get(doc.path())
                .is_some_and(|version| version == doc.version().as_str())
            {
                continue;
            }
            let text = source(doc)?;
            write(self.directory.path(), doc.path(), text.as_bytes())?;
            let absolute = self.directory.path().join(doc.path());
            self.client.notify("workspace/didChangeWatchedFiles",json!({"changes":[{"uri":crate::lsp::uri(&absolute)?,"type":if self.files.contains_key(doc.path()){2}else{1}}]}))?;
            self.open(
                &absolute,
                language(doc).ok_or("Source language missing")?,
                &text,
            )?;
            self.files
                .insert(doc.path().into(), doc.version().as_str().into());
        }
        Ok(())
    }
}
