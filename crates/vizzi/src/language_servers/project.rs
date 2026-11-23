//! Captured-source mirror and semantic lookup shared by non-Cargo adapters.
use super::inputs::Inputs;
use crate::{
    definitions::{self, Answer, Location, Position, Provider},
    dependency_sources::{Capture, Catalog, FILES as DEPENDENCY_FILES, Input},
    ecmascript_dependencies::manifest_id,
    lsp::{Client, DocumentSync},
};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use vizzi_core::{
    model::{ContentId, Document, Snapshot},
    structure::{CallLookup, Symbol},
};

const FILE_BYTES: usize = 8 * 1024 * 1024;
const WORKSPACE_BYTES: usize = 128 * 1024 * 1024;

pub(super) trait Profile: Send + Sync {
    fn languages(&self) -> &'static [&'static str];

    fn accepts(&self, language: &str) -> bool {
        self.languages().contains(&language)
    }

    fn includes_path(&self, _path: &str) -> bool {
        true
    }

    fn includes_document(&self, doc: &Document) -> bool {
        self.includes_path(doc.path()) && language(doc).is_some_and(|lang| self.accepts(lang))
    }

    fn inputs<'a>(&'a self, root: &'a Path, snapshot: &'a Snapshot) -> Result<Inputs<'a>, String>;
    fn external_source<'a>(&self, _path: &'a Path, _hint: &'a str) -> Option<ExternalSource<'a>> {
        None
    }

    fn dependency_input(
        &self,
        _workspace: &Path,
        path: &Path,
        configuration: &str,
        hint: &str,
    ) -> Result<Input, String> {
        let ExternalSource {
            root,
            language,
            package,
        } = self
            .external_source(path, hint)
            .ok_or("Unsupported external source language")?;
        if !self.accepts(language) {
            return Err("External source language differs".into());
        }
        Ok(Input {
            root: root.into(),
            path: path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_str()
                .ok_or("External filename is not UTF-8")?
                .replace('\\', "/"),
            language: language.into(),
            package: package.into(),
            version: None,
            configuration: configuration.into(),
            mirror_path: None,
            installation: None,
        })
    }

    fn document_language<'a>(&self, _path: &Path, language: &'a str) -> &'a str {
        language
    }

    fn project_error(
        &self,
        _client: &mut Client,
        _path: &Path,
        _rejected_configs: &BTreeMap<PathBuf, String>,
    ) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn signature_only(&self, _path: &Path) -> bool {
        false
    }

    fn filter_references(
        &self,
        _lookup: ReferenceLookup<'_>,
        locations: Vec<Location>,
    ) -> Result<Vec<Location>, String> {
        Ok(locations)
    }
}

pub(super) struct ExternalSource<'a> {
    pub root: &'a Path,
    pub language: &'a str,
    pub package: &'a str,
}

pub(super) struct ProjectServer {
    root: PathBuf,
    cache: PathBuf,
    executable: PathBuf,
    profile: Box<dyn Profile>,
    sources: Arc<Catalog>,
    state: Mutex<Option<Workspace>>,
}
struct Workspace {
    client: Client,
    directory: tempfile::TempDir,
    _private_state: Option<tempfile::TempDir>,
    configuration: String,
    rejected_configs: BTreeMap<PathBuf, String>,
    files: BTreeMap<String, String>,
    opened: BTreeSet<PathBuf>,
    external: BTreeSet<PathBuf>,
    reached_manifests: BTreeMap<PathBuf, ContentId>,
    next: i32,
}

type DefinitionLocations = Vec<(Option<PathBuf>, Position)>;

pub(super) struct ReferenceLookup<'a> {
    pub path: &'a Path,
    pub position: Position,
    language: &'a str,
    snapshot: &'a Snapshot,
    sources: &'a Catalog,
    workspace: &'a mut Workspace,
}

impl ReferenceLookup<'_> {
    pub(super) fn definitions_at(
        &mut self,
        location: &Location,
        include: impl FnOnce(&Document, usize) -> bool,
    ) -> Result<Option<DefinitionLocations>, String> {
        let external = location
            .capture
            .as_ref()
            .map(|id| self.sources.get(id))
            .transpose()?
            .flatten();
        let doc = external
            .as_ref()
            .map(|capture| capture.doc())
            .or_else(|| {
                self.snapshot
                    .file_by_path(&location.path)
                    .map(AsRef::as_ref)
            })
            .ok_or("Reference source expired")?;
        let text = source(doc)?;
        let byte = definitions::byte_offset(&text, location.position)?;
        if !include(doc, byte) {
            return Ok(None);
        }
        let path = match &external {
            Some(capture) => self.workspace.dependency_path(capture)?,
            None => self.workspace.directory.path().join(&location.path),
        };
        self.workspace.open(&path, self.language, &text)?;
        self.workspace
            .client
            .definitions(&path, location.position)
            .map(Some)
    }
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
        profile: impl Profile + 'static,
        sources: Arc<Catalog>,
    ) -> Self {
        Self {
            root,
            cache,
            executable,
            profile: Box::new(profile),
            sources,
            state: Mutex::new(None),
        }
    }
    fn workspace(&self, snapshot: &Snapshot, inputs: Inputs<'_>) -> Result<Workspace, String> {
        fs::create_dir_all(&self.cache).map_err(|e| e.to_string())?;
        let cache = self.cache.canonicalize().map_err(|e| e.to_string())?;
        let directory = tempfile::Builder::new()
            .prefix("captured-")
            .tempdir_in(cache)
            .map_err(|e| e.to_string())?;
        let mut files = BTreeMap::new();
        let mut total = 0;
        for doc in snapshot
            .documents()
            .iter()
            .filter(|doc| self.profile.includes_document(doc))
        {
            let text = source(doc)?;
            total += text.len();
            if total > WORKSPACE_BYTES {
                return Err("Captured workspace exceeds semantic allowance".into());
            }
            write(directory.path(), doc.path(), text.as_bytes())?;
            files.insert(doc.path().into(), doc.version().as_str().into());
        }
        let configuration = inputs.digest().to_owned();
        let prepared = inputs.prepare(directory.path())?;
        let client = Client::configured(&self.executable, directory.path(), prepared.setup)?;
        Ok(Workspace {
            directory,
            client,
            _private_state: prepared.private_state,
            configuration,
            rejected_configs: prepared.rejected_configs,
            files,
            opened: BTreeSet::new(),
            external: BTreeSet::new(),
            reached_manifests: BTreeMap::new(),
            next: 0,
        })
    }
    fn lookup_source(&self, doc: &Document, external: bool) -> Result<String, String> {
        if !external && !self.profile.includes_path(doc.path()) {
            return Err("Captured source is outside the configured source roots".into());
        }
        source(doc)
    }
    fn lookup(
        &self,
        snapshot: Arc<Snapshot>,
        doc: &Document,
        text: &str,
        byte: usize,
        external: Option<&Capture>,
        references: bool,
    ) -> Result<Answer, String> {
        let lang = language(doc).ok_or("Semantic language is unavailable")?;
        if !self.profile.accepts(lang) {
            return Err("Unsupported semantic language".into());
        }
        let position = definitions::position(text, byte)?;
        let inputs = self.profile.inputs(&self.root, &snapshot)?;
        let configuration = inputs.digest().to_owned();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut workspace = match state.take() {
            Some(workspace)
                if workspace.configuration == configuration
                    && workspace.reached_manifests.len() < DEPENDENCY_FILES
                    && workspace.manifests_unchanged() =>
            {
                workspace
            }
            _ => self.workspace(&snapshot, inputs)?,
        };
        workspace.sync(&snapshot, self.profile.as_ref())?;
        let importer = external.map_or(Some(doc.path()), |capture| {
            capture
                .input
                .installation
                .as_ref()
                .and_then(|package| package.importer.as_deref())
        });
        let path = if let Some(capture) = external {
            if capture.input.configuration != configuration {
                return Err("Dependency configuration changed; reopen from its caller".into());
            }
            capture.verify()?;
            let path = workspace.dependency_path(capture)?;
            if let Some(importer) = importer {
                let document = snapshot
                    .file_by_path(importer)
                    .ok_or("The package importer is outside the current capture")?;
                let lang = language(document)
                    .filter(|lang| self.profile.accepts(lang))
                    .ok_or("The package importer's language changed")?;
                let importer = workspace.directory.path().join(importer);
                workspace.open(
                    &importer,
                    self.profile.document_language(&importer, lang),
                    &source(document)?,
                )?;
            }
            workspace.remember_manifest(&capture.input)?;
            workspace.external.insert(path.clone());
            path
        } else {
            workspace.directory.path().join(doc.path())
        };
        workspace.open(&path, self.profile.document_language(&path, lang), text)?;
        if let Some(error) =
            self.profile
                .project_error(&mut workspace.client, &path, &workspace.rejected_configs)?
        {
            *state = Some(workspace);
            return Err(error);
        }
        let captures = self.sources.sources()?;
        let retained = captures
            .iter()
            .filter(|capture| {
                self.profile.accepts(&capture.input.language)
                    && capture.input.configuration == configuration
                    && capture.verify().is_ok()
            })
            .filter_map(|capture| workspace.dependency_path(capture).ok())
            .collect::<BTreeSet<_>>();
        workspace.prune_external(&retained)?;
        for capture in captures {
            if self.profile.accepts(&capture.input.language)
                && capture.input.configuration == configuration
                && capture.verify().is_ok()
                && let Ok(path) = workspace.dependency_path(&capture)
            {
                workspace.open_dependency(&path, &capture, self.profile.as_ref())?;
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
                let Some(reported) = target else {
                    outside += 1;
                    continue;
                };
                let target = fs::canonicalize(&reported).unwrap_or_else(|_| reported.clone());
                if !references && self.profile.signature_only(&target) {
                    signatures += 1;
                    continue;
                }
                if let Ok(relative) = target.strip_prefix(workspace.directory.path()) {
                    let relative = relative
                        .to_str()
                        .ok_or("Semantic path is not UTF-8")?
                        .replace('\\', "/");
                    if self.profile.includes_path(&relative)
                        && snapshot.file_by_path(&relative).is_some()
                    {
                        locations.push(Location {
                            capture: None,
                            path: relative,
                            position,
                        });
                    } else {
                        outside += 1;
                    }
                } else {
                    let mut input = match self.profile.dependency_input(
                        &self.root,
                        &target,
                        &configuration,
                        lang,
                    ) {
                        Ok(input) => input,
                        Err(_) => {
                            outside += 1;
                            continue;
                        }
                    };
                    input.mirror_path = reported
                        .strip_prefix(workspace.directory.path())
                        .ok()
                        .and_then(Path::to_str)
                        .map(|path| path.replace('\\', "/"));
                    if let Some(package) = &mut input.installation {
                        package.importer = importer.map(str::to_owned);
                    }
                    workspace.remember_manifest(&input)?;
                    let captured = match self.sources.capture(input) {
                        Ok(capture) => capture,
                        Err(_) => {
                            outside += 1;
                            continue;
                        }
                    };
                    captured.verify()?;
                    let path = workspace.dependency_path(&captured)?;
                    changed |=
                        workspace.open_dependency(&path, &captured, self.profile.as_ref())?;
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
                && !self.sources.get(id)?.is_some_and(|source| {
                    source.verify().is_ok() && workspace.dependency_path(&source).is_ok()
                })
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
        if references {
            locations = self.profile.filter_references(
                ReferenceLookup {
                    path: &path,
                    position,
                    language: lang,
                    snapshot: &snapshot,
                    sources: &self.sources,
                    workspace: &mut workspace,
                },
                locations,
            )?;
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
            .filter_map(|capture| workspace.dependency_path(&capture).ok())
            .collect();
        workspace.prune_external(&retained)?;
        if !workspace.manifests_unchanged() {
            return Err(
                "Installed package manifests changed during semantic lookup; retry navigation"
                    .into(),
            );
        }
        if self.profile.inputs(&self.root, &snapshot)?.digest() != configuration {
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
        for path in &workspace.external {
            let path = path.to_string_lossy();
            if let Some(version) = workspace.files.get(path.as_ref()) {
                hash.update(path.as_bytes());
                hash.update(version.as_bytes());
            }
        }
        for (root, manifest) in &workspace.reached_manifests {
            hash.update(root.as_os_str().as_encoded_bytes());
            hash.update(manifest.as_str().as_bytes());
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
        let text = self.lookup_source(doc, external.is_some())?;
        if text.get(span.clone()) != call.invocation.name.as_deref() {
            return Err("Captured call identifier differs".into());
        }
        self.lookup(
            snapshot.clone(),
            doc,
            &text,
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
        let text = self.lookup_source(doc, external.is_some())?;
        if !name.starts_with(&symbol.name)
            || text.get(identifier..identifier.saturating_add(name.len())) != Some(name)
        {
            return Err("Reference target identifier differs".into());
        }
        let offset = definitions::reference_offset(language(doc), name);
        self.lookup(
            snapshot.clone(),
            doc,
            &text,
            identifier + offset,
            external.as_deref(),
            true,
        )
    }
}
impl Workspace {
    fn manifests_unchanged(&self) -> bool {
        self.reached_manifests
            .iter()
            .all(|(root, expected)| manifest_id(root).ok().as_ref() == Some(expected))
    }

    fn remember_manifest(&mut self, input: &Input) -> Result<(), String> {
        if let Some(package) = &input.installation {
            let expected = self
                .reached_manifests
                .entry(input.root.clone())
                .or_insert_with(|| package.manifest.clone());
            if *expected != package.manifest {
                return Err(
                    "Installed package manifest changed during semantic lookup; retry navigation"
                        .into(),
                );
            }
        }
        Ok(())
    }

    fn dependency_path(&self, capture: &Capture) -> Result<PathBuf, String> {
        if let Some(package) = &capture.input.installation
            && manifest_id(&capture.input.root).ok().as_ref() != Some(&package.manifest)
        {
            return Err("Installed package manifest changed; reopen it from its caller".into());
        }
        let file = capture.path();
        let Some(relative) = &capture.input.mirror_path else {
            return Ok(file);
        };
        let path = self.directory.path().join(relative);
        if fs::canonicalize(&path).ok().as_ref() != Some(&file) {
            return Err("Dependency resolution path changed; reopen it from its caller".into());
        }
        Ok(path)
    }

    fn open_dependency(
        &mut self,
        path: &Path,
        capture: &Capture,
        profile: &dyn Profile,
    ) -> Result<bool, String> {
        self.remember_manifest(&capture.input)?;
        self.external.insert(path.to_owned());
        self.open(
            path,
            profile.document_language(path, &capture.input.language),
            &source(capture.doc())?,
        )
    }

    fn prune_external(&mut self, retained: &BTreeSet<PathBuf>) -> Result<(), String> {
        let removed = self
            .external
            .difference(retained)
            .cloned()
            .collect::<Vec<_>>();
        for path in removed {
            self.client.notify(
                "textDocument/didClose",
                json!({"textDocument":{"uri":crate::lsp::uri(&path)?}}),
            )?;
            self.opened.remove(&path);
            self.external.remove(&path);
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
        let reopen = self.client.document_sync == DocumentSync::ReopenWithSymbols
            && self.opened.contains(path);
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
    fn sync(&mut self, snapshot: &Snapshot, profile: &dyn Profile) -> Result<(), String> {
        let docs = snapshot
            .documents()
            .iter()
            .filter(|doc| profile.includes_document(doc))
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
                profile
                    .document_language(&absolute, language(doc).ok_or("Source language missing")?),
                &text,
            )?;
            self.files
                .insert(doc.path().into(), doc.version().as_str().into());
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn empty_snapshot() -> &'static Snapshot {
    use std::sync::OnceLock;
    use vizzi_core::model::{CapturePolicyId, WorkspaceId};
    static SNAPSHOT: OnceLock<Snapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        Snapshot::new(
            WorkspaceId::derive(&[b"project-input-test"]),
            vec![],
            CapturePolicyId::derive(&[b"fixture"]),
        )
        .unwrap()
    })
}
