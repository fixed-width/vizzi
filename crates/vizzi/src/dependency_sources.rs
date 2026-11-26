//! Bounded captured source admitted by native language providers, never browser paths.
use crate::{
    definitions::Position,
    review_source::{DirectoryInput, ReadEntry, ReviewInput},
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use vizzi_core::{
    analysis::{Analyzer, Languages},
    comparison::{EntryKind, ReviewPath},
    model::{
        Analysis, CapturePolicyId, ContentId, Context, ContextId, Coverage, Document, Fact,
        Snapshot, WorkspaceId,
    },
    policy::{Classification, Provenance},
    structure::{SourceOrigin, Structure, Symbol},
};
pub(crate) const FILE_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
const ARCHIVE_CANDIDATES: usize = 32;
pub(crate) const ARCHIVE_REPLAY_DEPTH: usize = 8;
const TOTAL_BYTES: usize = 32 * 1024 * 1024;
pub(crate) const FILES: usize = 32;
const SYMBOLS: usize = 32768;

/// Native admission input; no transport accepts this type from a browser.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// Canonical package directory or source archive selected by a language provider.
    pub root: PathBuf,
    /// Normalized source-root-relative file.
    pub path: String,
    /// Registered analysis language.
    pub language: String,
    /// Human-readable package label.
    pub package: String,
    /// Locally known package version.
    pub version: Option<String>,
    /// Provider project configuration that admitted the source.
    pub configuration: String,
    /// Relative path reported by a managed server for a mirror-backed package alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_path: Option<String>,
    /// Installed package resolution evidence local to the native provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installation: Option<PackageContext>,
    /// Expected provider source bytes when the input names an archive member.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive: Option<ContentId>,
}

/// Evidence needed to re-use an installed package capture in semantic navigation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageContext {
    /// Fingerprint of the installed package manifest that supplied this context.
    pub manifest: ContentId,
    /// Captured repository-relative file whose project resolved this package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importer: Option<String>,
    /// Original editor URI used by its provider, including a package alias if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_uri: Option<String>,
}

/// A provider-reported function range bound to exact saved source bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    /// Exact contents to which the provider range applies.
    pub content: ContentId,
    /// Identifier text, verified against the captured source.
    pub name: String,
    /// Complete function range in UTF-8 bytes.
    pub span: std::ops::Range<usize>,
    /// Exact identifier range in UTF-8 bytes.
    pub identifier: std::ops::Range<usize>,
}
impl Declaration {
    fn analysis(&self, syntax: &Analysis) -> Result<Analysis, String> {
        if !label(&self.name, 256) {
            return Err("Invalid provider declaration name".into());
        }
        // Document::capture checks the content digest, ranges and identifier witness.
        let descriptor = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        Ok(Analysis {
            has_entry_point: false,
            content: self.content.clone(),
            context: Context {
                language: syntax.context.language.clone(),
                description: "Provider function range; raw call analysis unavailable".into(),
                id: ContextId::derive(&[b"provider-function-range-v1", include_bytes!("dependency_sources.rs"), syntax.context.id.as_str().as_bytes(), &descriptor]),
            },
            coverage: Coverage::Partial,
            facts: vec![Fact {
                declaration_span: None,
                definition_anchor: None,
                name_span: Some(self.identifier.clone()), parent: None,
                key: format!("provider-function:{}:{}", self.identifier.start, self.name),
                name: self.name.clone(), kind: "function".into(), span: self.span.clone(), unique: true,
            }],
            calls: None,
            coloring: syntax.coloring.clone(),
            diagnostics: vec!["Function range supplied by the editor; native syntax and call coverage remain limited".into()],
        })
    }
}
fn opaque(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn label(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.contains(['\0', '\r', '\n'])
}
fn read(input: &Input) -> Result<Vec<u8>, String> {
    if input.path.len() > 4096
        || !label(&input.package, 256)
        || !label(&input.language, 64)
        || input.version.as_deref().is_some_and(|v| !label(v, 128))
        || !opaque(&input.configuration)
        || input.mirror_path.as_ref().is_some_and(|path| {
            path.len() > 4096
                || path.contains([':', '\\'])
                || ReviewPath::parse(path.as_bytes().to_vec()).is_err()
        })
        || input.installation.as_ref().is_some_and(|package| {
            !opaque(package.manifest.as_str())
                || package.importer.as_ref().is_some_and(|path| {
                    path.len() > 4096
                        || path.contains([':', '\\'])
                        || ReviewPath::parse(path.as_bytes().to_vec()).is_err()
                })
                || package
                    .editor_uri
                    .as_ref()
                    .is_some_and(|uri| !label(uri, 16 * 1024))
        })
    {
        return Err("Invalid dependency source descriptor".into());
    }
    let path = ReviewPath::parse(input.path.as_bytes().to_vec()).map_err(|e| e.to_string())?;
    if let Some(content) = &input.archive {
        if input.installation.is_some() || input.mirror_path.is_some() {
            return Err("Archive sources cannot have an installation alias".into());
        }
        let bytes = read_archive_member(input)?;
        if ContentId::derive(&[&bytes]) != *content {
            return Err("Source archive does not match the language server's document".into());
        }
        return Ok(bytes);
    }
    let mut directory = DirectoryInput::open(&input.root).map_err(|e| e.to_string())?;
    match directory
        .read(&path, FILE_BYTES, &|| false)
        .map_err(|e| e.to_string())?
    {
        ReadEntry::Present {
            kind: EntryKind::Regular { .. },
            bytes,
            ..
        } => Ok(bytes),
        _ => Err("Dependency source is unavailable or is not a regular file".into()),
    }
}

pub(crate) fn open_archive(path: &Path) -> Result<fs::File, String> {
    #[cfg(unix)]
    let file = {
        use rustix::fs::{Mode, OFlags, open};
        fs::File::from(
            open(
                path,
                OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
                Mode::empty(),
            )
            .map_err(|e| e.to_string())?,
        )
    };
    #[cfg(not(unix))]
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > ARCHIVE_BYTES {
        return Err("Source archive is unavailable or exceeds allowance".into());
    }
    Ok(file)
}

fn read_archive_member(input: &Input) -> Result<Vec<u8>, String> {
    let mut archive =
        zip::ZipArchive::new(open_archive(&input.root)?).map_err(|e| e.to_string())?;
    let entry = archive.by_name(&input.path).map_err(|e| e.to_string())?;
    read_archive_entry(entry)
}

pub(crate) fn matching_archive_member(
    archive: &Path,
    member: &str,
    expected: &[u8],
) -> Result<String, String> {
    let mut archive = zip::ZipArchive::new(open_archive(archive)?).map_err(|e| e.to_string())?;
    let candidates = archive
        .file_names()
        .filter_map(Result::ok)
        .filter(|name| {
            name.strip_suffix(member)
                .is_some_and(|prefix| prefix.is_empty() || prefix.ends_with('/'))
                && name.len() <= 4096
                && !name.contains('\\')
                && ReviewPath::parse(name.as_bytes().to_vec()).is_ok()
        })
        .take(ARCHIVE_CANDIDATES + 1)
        .map(std::borrow::Cow::into_owned)
        .collect::<Vec<_>>();
    if candidates.len() > ARCHIVE_CANDIDATES {
        return Err("Source archive member search exceeds allowance".into());
    }
    let mut matched = None;
    for name in candidates {
        let entry = archive.by_name(&name).map_err(|e| e.to_string())?;
        if !entry.is_file()
            || entry.size() != expected.len() as u64
            || read_archive_entry(entry)? != expected
        {
            continue;
        }
        if matched.replace(name).is_some() {
            return Err("Source archive contains multiple matching source members".into());
        }
    }
    matched.ok_or_else(|| "No source archive member matches the language server's document".into())
}

fn read_archive_entry(entry: zip::read::ZipFile<'_, fs::File>) -> Result<Vec<u8>, String> {
    if !entry.is_file() || entry.size() > FILE_BYTES as u64 {
        return Err("Source archive member is unavailable or exceeds allowance".into());
    }
    let mut bytes = Vec::new();
    entry
        .take(FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > FILE_BYTES {
        return Err("Source archive member exceeds allowance".into());
    }
    Ok(bytes)
}
/// One immutable dependency file and its structural facts.
pub struct Capture {
    /// Independent source observation.
    pub snapshot: Arc<Snapshot>,
    /// Bounded native declaration/call projection.
    pub index: Structure,
    /// Public package provenance.
    pub origin: SourceOrigin,
    /// Native source input retained for verification, never serialized to the viewer.
    pub input: Input,
    document: Arc<Document>,
    declaration: Option<Declaration>,
    pub(crate) archive_caller: Option<ArchiveCaller>,
    pub(crate) editor_archive: Option<EditorArchive>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EditorArchive {
    pub uri: String,
    pub binary: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArchiveCaller {
    pub source: ArchiveCallerSource,
    pub position: Position,
    pub destination: Position,
    pub imported: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ArchiveCallerSource {
    Repository { path: String, content: ContentId },
    Archive { capture: String },
}
impl Capture {
    /// Written calls whose same-file candidates retain dependency provenance.
    pub fn calls(
        &self,
        symbol: &str,
        offset: usize,
    ) -> Result<vizzi_core::structure::Page<vizzi_core::structure::Invocation>, String> {
        let mut page = self
            .index
            .outgoing(symbol, self.doc().path(), offset)
            .map_err(|e| e.to_string())?;
        for site in &mut page.items {
            site.candidates = site.candidates.drain(..).map(|s| self.symbol(s)).collect();
        }
        Ok(page)
    }
    /// Returns the captured file without consulting mutable disk bytes.
    pub fn doc(&self) -> &Document {
        &self.document
    }
    /// Adds source provenance to an already captured native declaration.
    pub fn symbol(&self, mut symbol: Symbol) -> Symbol {
        symbol.origin = Some(self.origin.clone());
        symbol
    }
    /// Verifies an explicit semantic/editor action against current local contents.
    pub fn verify(&self) -> Result<(), String> {
        let bytes = read(&self.input)?;
        if vizzi_core::model::ContentId::derive(&[&bytes]) != *self.document.content() {
            return Err("Dependency source changed; reopen it from its caller".into());
        }
        Ok(())
    }
    /// Exact native file path for file-backed captures.
    pub fn path(&self) -> Option<PathBuf> {
        self.input
            .archive
            .is_none()
            .then(|| self.input.root.join(&self.input.path))
    }
}
thread_local! {static ANALYZERS: RefCell<BTreeMap<String, Box<dyn Analyzer>>> = RefCell::new(BTreeMap::new());}
fn capture(
    input: Input,
    declaration: Option<Declaration>,
    archive_caller: Option<ArchiveCaller>,
    editor_archive: Option<EditorArchive>,
) -> Result<Arc<Capture>, String> {
    if !input.root.is_absolute()
        || fs::canonicalize(&input.root).map_err(|e| e.to_string())? != input.root
    {
        return Err("Dependency root must be canonical".into());
    }
    if let Some(caller) = &archive_caller {
        let valid = match &caller.source {
            ArchiveCallerSource::Repository { path, .. } => {
                path.len() <= 4096
                    && !path.contains('\\')
                    && ReviewPath::parse(path.as_bytes().to_vec()).is_ok()
            }
            ArchiveCallerSource::Archive { capture } => opaque(capture),
        };
        if input.archive.is_none() || !valid {
            return Err("Invalid archive caller".into());
        }
    }
    if let Some(editor) = &editor_archive
        && (input.archive.is_none()
            || input.language != "java"
            || !editor.binary.is_absolute()
            || editor.binary.as_os_str().as_encoded_bytes().len() > 4096
            || url::Url::parse(&editor.uri)
                .ok()
                .and_then(|uri| crate::java_archive::destination(&uri))
                .is_none())
    {
        return Err("Invalid editor archive context".into());
    }
    let bytes = read(&input)?;
    let analysis = ANALYZERS.with_borrow_mut(|analyzers| {
        if !analyzers.contains_key(&input.language) {
            let languages = Languages::builtin().map_err(|e| e.to_string())?;
            let analyzer = languages
                .select(&input.language)
                .ok_or("Dependency analysis is unavailable for this language")?
                .create()
                .map_err(|e| e.to_string())?;
            analyzers.insert(input.language.clone(), analyzer);
        }
        analyzers
            .get_mut(&input.language)
            .ok_or("Dependency analyzer unavailable")?
            .analyze(&bytes)
            .map_err(|e| e.to_string())
    })?;
    let additional = declaration
        .as_ref()
        .map(|d| d.analysis(&analysis))
        .transpose()?;
    let analyses = match additional {
        Some(provider) => vec![provider, analysis],
        None => vec![analysis],
    };
    let root = input.root.to_str().ok_or("Dependency path is not UTF-8")?;
    let scope = WorkspaceId::derive(&[
        b"dependency-scope-v1",
        root.as_bytes(),
        input.language.as_bytes(),
        input.package.as_bytes(),
        input.version.as_deref().unwrap_or_default().as_bytes(),
    ]);
    let doc = Arc::new(
        Document::capture(
            scope.clone(),
            input.path.clone(),
            &bytes,
            analyses,
            Classification::Assigned {
                language: input.language.clone(),
                provenance: Provenance::ExplicitScope { scope: ".".into() },
            },
        )
        .map_err(|e| e.to_string())?,
    );
    if doc.declaration_count() > SYMBOLS {
        return Err("Dependency declaration allowance exceeded".into());
    }
    let mut capture_policy = vec![
        b"dependency-capture-v1".as_slice(),
        input.configuration.as_bytes(),
    ];
    if let Some(path) = &input.mirror_path {
        capture_policy.push(path.as_bytes());
    }
    if let Some(package) = &input.installation {
        capture_policy.extend([b"installed-package", package.manifest.as_str().as_bytes()]);
        if let Some(uri) = &package.editor_uri {
            capture_policy.push(uri.as_bytes());
        }
    }
    if let Some(editor) = &editor_archive {
        capture_policy.extend([
            b"editor-archive",
            editor.uri.as_bytes(),
            editor.binary.as_os_str().as_encoded_bytes(),
        ]);
    }
    let snapshot = Arc::new(
        Snapshot::new(
            scope.clone(),
            vec![doc.clone()],
            CapturePolicyId::derive(&capture_policy),
        )
        .map_err(|e| e.to_string())?,
    );
    let index = Structure::new(&snapshot);
    let origin = SourceOrigin {
        id: scope.as_str().into(),
        capture: snapshot.id().as_str().into(),
        label: input.package.clone(),
        version: input.version.clone(),
        language: input.language.clone(),
        archive: input.archive.is_some(),
        editor_archive: editor_archive.is_some(),
    };
    Ok(Arc::new(Capture {
        snapshot,
        index,
        origin,
        input,
        document: doc,
        declaration,
        archive_caller,
        editor_archive,
    }))
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    capture: String,
    input: Input,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    declaration: Option<Declaration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    archive_caller: Option<ArchiveCaller>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    editor_archive: Option<EditorArchive>,
}
#[derive(Default)]
struct State {
    loaded: bool,
    sources: Vec<Arc<Capture>>,
}
impl State {
    fn archive_trail_available<'a>(
        &'a self,
        snapshot: &Snapshot,
        mut caller: &'a ArchiveCaller,
        imported: bool,
        destination: Option<&Capture>,
        language: &str,
    ) -> bool {
        for _ in 0..ARCHIVE_REPLAY_DEPTH {
            if caller.imported && !imported {
                return false;
            }
            match &caller.source {
                ArchiveCallerSource::Repository { path, content } => {
                    return snapshot.file_by_path(path).is_some_and(|doc| {
                        doc.content() == content && matches!(
                            doc.classification(), Classification::Assigned { language: assigned, .. } if assigned == language
                        )
                    });
                }
                ArchiveCallerSource::Archive { capture } => {
                    if destination.is_some_and(|source| &source.origin.capture == capture) {
                        return false;
                    }
                    let Some(parent) = self.sources.iter().find(|source| {
                        &source.origin.capture == capture
                            && source.input.archive.is_some()
                            && source.input.language == language
                    }) else {
                        return false;
                    };
                    let Some(previous) = &parent.archive_caller else {
                        return false;
                    };
                    if parent.verify().is_err() {
                        return false;
                    }
                    caller = previous;
                }
            }
        }
        false
    }
}
/// A small reusable catalog. Its cache allowance does not cap total process memory.
pub struct Catalog {
    references: PathBuf,
    state: Mutex<State>,
}
impl Catalog {
    /// Creates an idle catalog; acquisition and reference restoration are lazy.
    pub fn new(references: PathBuf) -> Self {
        Self {
            references,
            state: Mutex::new(State::default()),
        }
    }
    fn load(&self, state: &mut State) -> Result<(), String> {
        if state.loaded {
            return Ok(());
        }
        let file = match fs::File::open(&self.references) {
            Ok(f) => Some(f),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        if let Some(file) = file {
            let mut bytes = vec![];
            file.take(1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 1024 * 1024 {
                return Err("Dependency reference allowance exceeded".into());
            }
            let refs: Vec<Reference> = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if refs.len() > FILES {
                return Err("Dependency reference count exceeded".into());
            }
            for r in refs {
                if !opaque(&r.capture) {
                    return Err("Invalid dependency capture reference".into());
                }
                if let Ok(source) =
                    capture(r.input, r.declaration, r.archive_caller, r.editor_archive)
                    && source.origin.capture == r.capture
                {
                    admit(state, source);
                }
            }
        }
        state.loaded = true;
        Ok(())
    }
    fn persist(&self, state: &State) -> Result<(), String> {
        let parent = self
            .references
            .parent()
            .ok_or("Dependency cache needs a parent directory")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let refs = state
            .sources
            .iter()
            .map(|s| Reference {
                capture: s.origin.capture.clone(),
                input: s.input.clone(),
                declaration: s.declaration.clone(),
                archive_caller: s.archive_caller.clone(),
                editor_archive: s.editor_archive.clone(),
            })
            .collect::<Vec<_>>();
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec(&refs).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.persist(&self.references).map_err(|e| e.to_string())?;
        Ok(())
    }
    /// Captures one provider-admitted file; existing captured bytes are never mutated.
    pub fn capture(&self, input: Input) -> Result<Arc<Capture>, String> {
        self.capture_inner(input, None, None, None)
    }
    /// Captures source with a checked provider range when raw syntax cannot identify a function.
    pub fn capture_with_declaration(
        &self,
        input: Input,
        declaration: Declaration,
    ) -> Result<Arc<Capture>, String> {
        self.capture_inner(input, Some(declaration), None, None)
    }
    pub(crate) fn capture_archive(
        &self,
        input: Input,
        caller: Option<ArchiveCaller>,
        snapshot: &Snapshot,
    ) -> Result<Arc<Capture>, String> {
        self.capture_inner(input, None, caller.map(|caller| (caller, snapshot)), None)
    }
    pub(crate) fn capture_editor_archive(
        &self,
        input: Input,
        editor: EditorArchive,
    ) -> Result<Arc<Capture>, String> {
        self.capture_inner(input, None, None, Some(editor))
    }
    fn capture_inner(
        &self,
        input: Input,
        declaration: Option<Declaration>,
        archive_caller: Option<(ArchiveCaller, &Snapshot)>,
        editor_archive: Option<EditorArchive>,
    ) -> Result<Arc<Capture>, String> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.load(&mut state)?;
        let existing = state.sources.iter().rposition(|s| {
            s.input == input
                && s.declaration == declaration
                && s.editor_archive == editor_archive
                && s.verify().is_ok()
        });
        let previous = existing.map(|index| state.sources[index].as_ref());
        let archive_caller = archive_caller.and_then(|(caller, snapshot)| {
            let available = |witness| {
                state.archive_trail_available(
                    snapshot,
                    witness,
                    caller.imported,
                    previous,
                    &input.language,
                )
            };
            let replace = previous
                .and_then(|source| source.archive_caller.as_ref())
                .is_none_or(|old| {
                    old != &caller
                        && (matches!(caller.source, ArchiveCallerSource::Repository { .. })
                            || !available(old))
                });
            (replace && available(&caller)).then_some(caller)
        });
        if let Some(n) = existing
            && archive_caller.is_none()
        {
            let source = state.sources.remove(n);
            state.sources.push(source.clone());
            return Ok(source);
        }
        let source = capture(input, declaration, archive_caller, editor_archive)?;
        admit(&mut state, source.clone());
        self.persist(&state)?;
        Ok(source)
    }
    /// Gets a retained capture, restoring only byte/context/configuration-identical references.
    pub fn get(&self, id: &str) -> Result<Option<Arc<Capture>>, String> {
        if !opaque(id) {
            return Err("Invalid dependency capture identity".into());
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.load(&mut state)?;
        let Some(n) = state.sources.iter().position(|s| s.origin.capture == id) else {
            return Ok(None);
        };
        let source = state.sources.remove(n);
        state.sources.push(source.clone());
        Ok(Some(source))
    }
    /// Finds native facts across the finite set of previously reached sources.
    pub fn sources(&self) -> Result<Vec<Arc<Capture>>, String> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.load(&mut state)?;
        Ok(state.sources.clone())
    }
}
fn admit(state: &mut State, source: Arc<Capture>) {
    state
        .sources
        .retain(|s| s.origin.capture != source.origin.capture);
    while state.sources.len() >= FILES
        || state
            .sources
            .iter()
            .map(|s| s.doc().captured().len())
            .sum::<usize>()
            + source.doc().captured().len()
            > TOTAL_BYTES
        || state
            .sources
            .iter()
            .map(|s| s.doc().declaration_count())
            .sum::<usize>()
            + source.doc().declaration_count()
            > SYMBOLS
    {
        state.sources.remove(0);
    }
    state.sources.push(source);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive_input(root: &Path, path: &str, contents: &[u8]) -> Input {
        let archive = root.join(format!("{path}.jar"));
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        zip.start_file(path, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(contents).unwrap();
        zip.finish().unwrap();
        Input {
            root: archive.canonicalize().unwrap(),
            path: path.into(),
            language: "java".into(),
            package: "helper.jar".into(),
            version: None,
            configuration: "a".repeat(64),
            mirror_path: None,
            installation: None,
            archive: Some(ContentId::derive(&[contents])),
        }
    }

    fn repository_caller(contents: &[u8]) -> ArchiveCaller {
        ArchiveCaller {
            source: ArchiveCallerSource::Repository {
                path: "Caller.java".into(),
                content: ContentId::derive(&[contents]),
            },
            position: Position {
                line: 0,
                character: 27,
            },
            destination: Position {
                line: 0,
                character: 26,
            },
            imported: true,
        }
    }

    fn repository_snapshot(contents: &[u8]) -> Snapshot {
        let workspace = WorkspaceId::derive(&[b"archive replay fixture"]);
        let caller = Document::capture(
            workspace.clone(),
            "Caller.java".into(),
            contents,
            vec![],
            Classification::Assigned {
                language: "java".into(),
                provenance: Provenance::ExplicitScope { scope: ".".into() },
            },
        )
        .unwrap();
        Snapshot::new(
            workspace,
            vec![Arc::new(caller)],
            CapturePolicyId::derive(&[b"fixture"]),
        )
        .unwrap()
    }

    #[test]
    fn archive_replay_retains_the_latest_repository_caller() {
        let root = tempfile::tempdir().unwrap();
        let input = archive_input(
            root.path(),
            "Helper.java",
            b"class Helper { static int answer() { return 1; } }",
        );
        let cache = root.path().join("cache");
        let latest = repository_caller(b"current caller");
        let snapshot = repository_snapshot(b"current caller");
        let id = {
            let catalog = Catalog::new(cache.clone());
            let initial = catalog
                .capture_archive(
                    input.clone(),
                    Some(repository_caller(b"old caller")),
                    &repository_snapshot(b"old caller"),
                )
                .unwrap();
            let refreshed = catalog
                .capture_archive(input.clone(), Some(latest.clone()), &snapshot)
                .unwrap();
            assert_eq!(initial.origin.capture, refreshed.origin.capture);
            let parent = catalog
                .capture_archive(
                    archive_input(root.path(), "Parent.java", b"class Parent {}"),
                    Some(latest.clone()),
                    &snapshot,
                )
                .unwrap();
            let indirect = ArchiveCaller {
                source: ArchiveCallerSource::Archive {
                    capture: parent.origin.capture.clone(),
                },
                ..latest.clone()
            };
            catalog
                .capture_archive(input, Some(indirect), &snapshot)
                .unwrap();
            refreshed.origin.capture.clone()
        };
        let restored = Catalog::new(cache).get(&id).unwrap().unwrap();
        assert_eq!(restored.archive_caller, Some(latest));
    }

    #[test]
    fn archive_replay_replaces_an_expired_indirect_caller() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        let catalog = Catalog::new(cache.clone());
        let direct = repository_caller(b"working repository caller");
        let snapshot = repository_snapshot(b"working repository caller");
        let parent = |path, contents| {
            catalog
                .capture_archive(
                    archive_input(root.path(), path, contents),
                    Some(direct.clone()),
                    &snapshot,
                )
                .unwrap()
        };
        let first = parent("First.java", b"class First {}");
        let second = parent("Second.java", b"class Second {}");
        let indirect = |parent: &Capture| ArchiveCaller {
            source: ArchiveCallerSource::Archive {
                capture: parent.origin.capture.clone(),
            },
            ..direct.clone()
        };
        let input = archive_input(root.path(), "Helper.java", b"class Helper {}");
        let reached = catalog
            .capture_archive(input.clone(), Some(indirect(&first)), &snapshot)
            .unwrap();
        assert_eq!(reached.archive_caller, Some(indirect(&first)));
        fs::remove_file(&first.input.root).unwrap();
        let replacement = indirect(&second);
        let refreshed = catalog
            .capture_archive(input, Some(replacement.clone()), &snapshot)
            .unwrap();
        assert_eq!(reached.origin.capture, refreshed.origin.capture);
        let restored = Catalog::new(cache)
            .get(&reached.origin.capture)
            .unwrap()
            .unwrap();
        assert_eq!(restored.archive_caller, Some(replacement));
    }

    #[test]
    fn archive_member_match_requires_exact_bytes_and_unique_path_suffix() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("sources.jar");
        let contents = b"actual source";
        let member = "lib/Helper.java";
        let write_archive = |ambiguous| {
            let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
            let mut entries: Vec<(&str, &[u8])> = vec![
                (member, b"former source"),
                ("otherlib/Helper.java", contents),
                ("src/main/java/lib/Helper.java", contents),
            ];
            if ambiguous {
                entries.push(("src/test/java/lib/Helper.java", contents));
            }
            for (name, bytes) in entries {
                zip.start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(bytes).unwrap();
            }
            zip.finish().unwrap();
        };
        write_archive(false);
        assert_eq!(
            matching_archive_member(&archive, member, contents).unwrap(),
            "src/main/java/lib/Helper.java"
        );
        write_archive(true);
        assert!(
            matching_archive_member(&archive, member, contents)
                .unwrap_err()
                .contains("multiple matching")
        );
    }
}
