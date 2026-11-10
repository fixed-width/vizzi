//! Bounded captured source admitted by native language providers, never browser paths.
use crate::review_source::{DirectoryInput, ReadEntry, ReviewInput};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use vizzi_core::{
    analysis::{Analyzer, Languages},
    comparison::{EntryKind, ReviewPath},
    model::{CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance},
    structure::{SourceOrigin, Structure, Symbol},
};
const FILE_BYTES: usize = 2 * 1024 * 1024;
const TOTAL_BYTES: usize = 32 * 1024 * 1024;
const FILES: usize = 32;
const SYMBOLS: usize = 32768;

/// Native admission input; no transport accepts this type from a browser.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// Canonical package/source root selected by a language provider.
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
    {
        return Err("Invalid dependency source descriptor".into());
    }
    let path = ReviewPath::parse(input.path.as_bytes().to_vec()).map_err(|e| e.to_string())?;
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
    /// Exact native file path for the configured provider/editor adapter.
    pub fn path(&self) -> PathBuf {
        self.input.root.join(&self.input.path)
    }
}
thread_local! {static ANALYZERS: RefCell<BTreeMap<String, Box<dyn Analyzer>>> = RefCell::new(BTreeMap::new());}
fn capture(input: Input) -> Result<Arc<Capture>, String> {
    if !input.root.is_absolute()
        || fs::canonicalize(&input.root).map_err(|e| e.to_string())? != input.root
    {
        return Err("Dependency root must be canonical".into());
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
            vec![analysis],
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
    let snapshot = Arc::new(
        Snapshot::new(
            scope.clone(),
            vec![doc.clone()],
            CapturePolicyId::derive(&[b"dependency-capture-v1", input.configuration.as_bytes()]),
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
    };
    Ok(Arc::new(Capture {
        snapshot,
        index,
        origin,
        input,
        document: doc,
    }))
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    capture: String,
    input: Input,
}
#[derive(Default)]
struct State {
    loaded: bool,
    sources: Vec<Arc<Capture>>,
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
                if let Ok(source) = capture(r.input)
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
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.load(&mut state)?;
        if let Some(n) = state
            .sources
            .iter()
            .rposition(|s| s.input == input && s.verify().is_ok())
        {
            let source = state.sources.remove(n);
            state.sources.push(source.clone());
            return Ok(source);
        }
        let source = capture(input)?;
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
