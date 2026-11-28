//! Current-observation adapter for metadata-only structural operations.

/// Coherent live working-set refresh.
pub mod live;
/// Captured caller/reference queries and observation-bound result paging.
pub mod references;
use crate::{companion::CompanionError, host::Host};
use serde::Deserialize;
pub mod editor;
use serde_json::Value;
use std::sync::{Arc, Mutex, TryLockError};
use vizzi_core::{
    model::{Document, EntityId, Snapshot},
    modules::{ModuleCache, ModuleIndex},
    structure::Structure,
};
const SEMANTIC_FILE_PROJECTIONS: usize = 16;
/// One structural operation bound to the visible native observation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Expected current observation.
    pub snapshot: String,
    /// Exact captured path hint for a declaration outside the repository projection.
    #[serde(default)]
    pub path: Option<String>,
    /// Explicit operation; no natural-language intent is inferred.
    pub query: Operation,
}
/// Finite structural query operations.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    /// Decorate exact call identifiers in a captured source section.
    Actions {
        /// Declaration identity from this observation.
        symbol: String,
        /// First visible one-based line.
        line_start: usize,
        /// Visible line allowance, at most 96.
        line_count: usize,
        /// Continuation offset for the finite action page.
        offset: usize,
    },
    /// Decorate call and function names in a captured file window.
    SourceActions {
        /// Exact captured file identity.
        file: String,
        /// First visible one-based line.
        line_start: usize,
        /// Visible line allowance, at most 96.
        line_count: usize,
        /// Continuation offset for the finite action page.
        offset: usize,
    },
    /// Open an exact captured function declaration from the source browser.
    Function {
        /// Exact captured file identity.
        file: String,
        /// Observation-bound function identity.
        symbol: String,
    },
    /// Resolve a saved bubble declaration in the current capture of the same source.
    SavedDeclaration {
        /// Exact captured file identity.
        file: String,
        /// Document version verified for the return.
        version: String,
        /// First line of the saved declaration.
        line: usize,
        /// Declaration identity across observations.
        declaration: EntityId,
    },
    /// Find source directories/files.
    Areas {
        /// Literal name/path substring.
        search: String,
        /// Continuation offset.
        offset: usize,
    },
    /// Enumerate captured declarations in a source area.
    Inventory {
        /// Selected normalized source area.
        area: String,
        /// Literal declaration name or path filter.
        search: String,
        /// Continuation offset.
        offset: usize,
        /// Restrict results to functions and allow word abbreviations in the filter.
        #[serde(default)]
        functions_only: bool,
    },
    /// Follow written invocations in one declaration.
    Calls {
        /// Opaque declaration identity from this observation.
        symbol: String,
        /// Selected source area.
        area: String,
        /// Continuation offset.
        offset: usize,
    },
    /// Inspect same-name candidate uses.
    Uses {
        /// Opaque declaration identity from this observation.
        symbol: String,
        /// Selected source area.
        area: String,
        /// Continuation offset.
        offset: usize,
    },
}
struct Cached {
    snapshot: String,
    index: Structure,
    modules: ModuleIndex,
    module_cache: ModuleCache,
    scoped: Option<(String, Arc<Structure>)>,
}
impl Cached {
    fn rebuild(snapshot: &Snapshot, previous: Option<Self>) -> Self {
        let mut module_cache = previous.map(|c| c.module_cache).unwrap_or_default();
        let modules = ModuleIndex::new(snapshot, &mut module_cache);
        let index = Structure::with_modules(snapshot, &modules);
        Self {
            snapshot: snapshot.id().as_str().into(),
            index,
            modules,
            module_cache,
            scoped: None,
        }
    }
    fn for_file(
        &mut self,
        snapshot: &Snapshot,
        path: &str,
    ) -> Result<Option<Arc<Structure>>, CompanionError> {
        if let Some((cached_path, index)) = &self.scoped
            && cached_path == path
        {
            return Ok(Some(index.clone()));
        }
        let scoped = Structure::for_file(snapshot, path, &self.modules)
            .map_err(|e| CompanionError::Invalid(e.to_string()))?
            .map(Arc::new);
        if let Some(index) = &scoped {
            self.scoped = Some((path.into(), index.clone()));
        }
        Ok(scoped)
    }
    fn for_tracking(
        &mut self,
        snapshot: &Snapshot,
        tracking: Option<&str>,
        symbol: &str,
        file: Option<&str>,
    ) -> Result<Option<Arc<Structure>>, CompanionError> {
        if let Some(path) = self
            .index
            .resolve(tracking, symbol)
            .map(|s| s.source.path.clone())
        {
            return Ok(self
                .for_evidence(snapshot, &path)?
                .filter(|index| index.resolve(tracking, symbol).is_some()));
        }
        match file.and_then(|id| snapshot.file(id)) {
            Some(doc) => self.for_file(snapshot, doc.path()),
            None => Ok(None),
        }
    }
    fn for_evidence(
        &mut self,
        snapshot: &Snapshot,
        path: &str,
    ) -> Result<Option<Arc<Structure>>, CompanionError> {
        if snapshot
            .file_by_path(path)
            .is_some_and(|doc| self.index.omits_file_facts(doc))
        {
            self.for_file(snapshot, path)
        } else {
            Ok(None)
        }
    }
    fn for_symbol(
        &mut self,
        snapshot: &Snapshot,
        id: &str,
        path_hint: Option<&str>,
    ) -> Result<Option<Arc<Structure>>, CompanionError> {
        let known = self.index.resolve(None, id).map(|s| s.source.path.clone());
        let scoped = match known.as_deref().or(path_hint) {
            Some(path) => self.for_evidence(snapshot, path)?,
            None => self.scoped.as_ref().map(|(_, index)| index.clone()),
        };
        Ok(scoped.filter(|index| index.resolve(None, id).is_some()))
    }
    fn at_location<T>(
        &mut self,
        snapshot: &Snapshot,
        doc: &Document,
        remaining: &mut usize,
        project: impl FnOnce(&Structure) -> T,
    ) -> Result<Option<T>, CompanionError> {
        if !self.index.omits_file_facts(doc) {
            return Ok(Some(project(&self.index)));
        }
        if self
            .scoped
            .as_ref()
            .is_none_or(|(path, _)| path != doc.path())
        {
            let Some(next) = remaining.checked_sub(1) else {
                return Ok(None);
            };
            *remaining = next;
        }
        Ok(self
            .for_file(snapshot, doc.path())?
            .map(|index| project(&index)))
    }
}
pub(crate) enum DependencyEditorLocation {
    File(crate::editor::EditorLocation),
    Archive(crate::editor::ArchiveEditorLocation),
}

/// Current repository metadata and one on-demand file projection.
pub struct StructureHost {
    reference_sets: Mutex<Vec<references::ReferenceSet>>,
    host: Arc<Host>,
    cache: Mutex<Option<Cached>>,
    definitions: Option<Arc<dyn crate::definitions::Provider>>,
}
impl StructureHost {
    pub(crate) fn dependency(
        &self,
        id: &str,
    ) -> Result<Option<Arc<crate::dependency_sources::Capture>>, CompanionError> {
        self.definitions
            .as_ref()
            .and_then(|p| p.dependencies())
            .map(|c| c.get(id).map_err(CompanionError::Unavailable))
            .transpose()
            .map(Option::flatten)
    }
    fn external(
        &self,
        id: &str,
    ) -> Result<Option<Arc<crate::dependency_sources::Capture>>, CompanionError> {
        let Some(catalog) = self.definitions.as_ref().and_then(|p| p.dependencies()) else {
            return Ok(None);
        };
        Ok(catalog
            .sources()
            .map_err(CompanionError::Unavailable)?
            .into_iter()
            .find(|s| s.index.resolve(None, id).is_some() || s.index.call(id).is_some()))
    }
    /// Reads a version-bound dependency window; no filesystem pathname is accepted.
    pub fn dependency_window(
        &self,
        snapshot: &str,
        file: &str,
        version: &str,
        query: &vizzi_core::projection::WindowQuery,
    ) -> Result<Value, CompanionError> {
        let source = self.dependency(snapshot)?.ok_or_else(|| {
            CompanionError::Conflict("Dependency capture expired; reopen it from its caller".into())
        })?;
        if source.doc().id().as_str() != file || source.doc().version().as_str() != version {
            return Err(CompanionError::Invalid(
                "Dependency source reference differs".into(),
            ));
        }
        let mut value = serde_json::to_value(
            source
                .doc()
                .window(query, None)
                .map_err(|e| CompanionError::Unavailable(e.to_string()))?,
        )
        .map_err(|e| CompanionError::Unavailable(e.to_string()))?;
        value["snapshot"] = snapshot.into();
        Ok(value)
    }
    /// Verifies an explicit file-backed dependency editor location against the captured bytes.
    pub fn dependency_location(
        &self,
        request: &crate::companion::OpenSource,
    ) -> Result<crate::editor::EditorLocation, CompanionError> {
        match self.dependency_editor_location(request)? {
            DependencyEditorLocation::File(location) => Ok(location),
            DependencyEditorLocation::Archive(_) => Err(CompanionError::Unavailable(
                "Archive source has no local editor file".into(),
            )),
        }
    }

    pub(crate) fn dependency_editor_location(
        &self,
        request: &crate::companion::OpenSource,
    ) -> Result<DependencyEditorLocation, CompanionError> {
        let source = self
            .dependency(&request.snapshot)?
            .ok_or_else(|| CompanionError::Conflict("Dependency capture expired".into()))?;
        if source.doc().id().as_str() != request.file
            || source.doc().version().as_str() != request.version
            || request.line == 0
            || request.line > source.doc().captured().line_count()
        {
            return Err(CompanionError::Invalid(
                "Invalid dependency editor reference".into(),
            ));
        }
        source.verify().map_err(CompanionError::Conflict)?;
        let text = source
            .doc()
            .captured()
            .read_text(2 * 1024 * 1024)
            .map_err(|e| CompanionError::Unavailable(e.to_string()))?
            .ok_or_else(|| CompanionError::Invalid("Dependency source is not UTF-8".into()))?;
        let source_witness = crate::editor::SourceWitness::from_bytes(text.as_bytes());
        if let Some(archive) = &source.editor_archive {
            return Ok(DependencyEditorLocation::Archive(
                crate::editor::ArchiveEditorLocation {
                    capture: source.origin.capture.clone(),
                    source: crate::editor::EditorLocation {
                        path: source.input.root.clone(),
                        line: request.line,
                        source: source_witness,
                    },
                    binary: archive.binary.clone(),
                    uri: archive.uri.clone(),
                },
            ));
        }
        let path = source.path().ok_or_else(|| {
            CompanionError::Unavailable("Archive source has no local editor file".into())
        })?;
        Ok(DependencyEditorLocation::File(
            crate::editor::EditorLocation {
                path,
                line: request.line,
                source: source_witness,
            },
        ))
    }
    pub(crate) fn verify_editor_archive(&self, capture: &str) -> Result<(), CompanionError> {
        let source = self
            .dependency(capture)?
            .filter(|source| source.editor_archive.is_some())
            .ok_or_else(|| {
                CompanionError::Conflict("Editor archive capture is unavailable".into())
            })?;
        source.verify().map_err(CompanionError::Conflict)
    }
    /// Creates an idle projection adapter without source reads or parsing.
    pub fn new(host: Arc<Host>) -> Self {
        Self {
            reference_sets: Mutex::new(Vec::new()),
            host,
            cache: Mutex::new(None),
            definitions: None,
        }
    }
    pub(crate) fn modules_for_file(&self, snapshot: &Snapshot, path: &str) -> ModuleIndex {
        let cached = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .filter(|cached| cached.snapshot == snapshot.id().as_str())
            .map(|cached| cached.modules.for_file(path));
        cached.unwrap_or_else(|| {
            ModuleIndex::new(snapshot, &mut ModuleCache::default()).for_file(path)
        })
    }
    /// Runs one bounded operation, refusing stale captures before returning results.
    pub fn query(&self, request: &Request) -> Result<Value, CompanionError> {
        let snapshot = self
            .host
            .application()
            .snapshot()
            .ok_or_else(|| CompanionError::Unavailable("Wait for source indexing".into()))?;
        if snapshot.id().as_str() != request.snapshot {
            return Err(CompanionError::Conflict(
                "Source changed; choose the area again".into(),
            ));
        }
        let mut cache = match self.cache.try_lock() {
            Ok(g) => g,
            Err(TryLockError::Poisoned(e)) => e.into_inner(),
            Err(TryLockError::WouldBlock) => {
                return Err(CompanionError::Unavailable(
                    "Another structural query is running; retry".into(),
                ));
            }
        };
        if cache
            .as_ref()
            .is_none_or(|c| c.snapshot != request.snapshot)
        {
            let previous = cache.take();
            *cache = Some(Cached::rebuild(&snapshot, previous));
        }
        let cache = cache
            .as_mut()
            .ok_or_else(|| CompanionError::Unavailable("Structure is unavailable".into()))?;
        let source_doc = match &request.query {
            Operation::SourceActions { file, .. }
            | Operation::Function { file, .. }
            | Operation::SavedDeclaration { file, .. } => {
                Some(snapshot.file(file).ok_or_else(|| {
                    CompanionError::Invalid("File is absent from this observation".into())
                })?)
            }
            _ => None,
        };
        let scoped = match &request.query {
            Operation::Inventory {
                area,
                functions_only: true,
                ..
            } if !area.is_empty() => cache.for_file(&snapshot, area)?,
            Operation::Actions { symbol, .. } => {
                cache.for_symbol(&snapshot, symbol, request.path.as_deref())?
            }
            Operation::SourceActions { .. }
            | Operation::Function { .. }
            | Operation::SavedDeclaration { .. } => match source_doc {
                Some(doc) => cache.for_evidence(&snapshot, doc.path())?,
                None => None,
            },
            Operation::Uses { symbol, .. } if cache.index.resolve(None, symbol).is_some() => None,
            Operation::Calls { symbol, area, .. } | Operation::Uses { symbol, area, .. } => {
                cache.for_symbol(&snapshot, symbol, request.path.as_deref().or(Some(area)))?
            }
            _ => None,
        };
        let index = scoped.as_deref().unwrap_or(&cache.index);
        let foreign = match &request.query {
            Operation::Actions { symbol, .. }
            | Operation::Calls { symbol, .. }
            | Operation::Uses { symbol, .. }
                if index.resolve(None, symbol).is_none() =>
            {
                self.external(symbol)?
            }
            _ => None,
        };
        let index = foreign.as_ref().map_or(index, |s| &s.index);
        let mut value = match &request.query {
            Operation::Actions {
                symbol,
                line_start,
                line_count,
                offset,
            } => serde_json::to_value(
                index
                    .actions(symbol, *line_start, *line_count, *offset)
                    .map_err(|e| CompanionError::Invalid(e.to_string()))?,
            ),
            Operation::SourceActions {
                line_start,
                line_count,
                offset,
                ..
            } => {
                let doc = source_doc
                    .ok_or_else(|| CompanionError::Invalid("Captured file is required".into()))?;
                serde_json::to_value(
                    index
                        .source_actions(doc, *line_start, *line_count, *offset)
                        .map_err(|e| CompanionError::Invalid(e.to_string()))?,
                )
            }
            Operation::Function { file, symbol } => {
                let function = index
                    .resolve(None, symbol)
                    .filter(|symbol| symbol.kind == "function" && symbol.source.file == *file)
                    .ok_or_else(|| {
                        CompanionError::Invalid("Function is absent from this captured file".into())
                    })?;
                Ok(serde_json::json!({"snapshot":request.snapshot,"symbol":function}))
            }
            Operation::SavedDeclaration {
                file,
                version,
                line,
                declaration,
            } => {
                let symbol = index
                    .resolve(Some(declaration.as_str()), "")
                    .filter(|symbol| {
                        symbol.source.file == *file
                            && symbol.source.version == *version
                            && symbol.source.line == *line
                    })
                    .ok_or_else(|| {
                        CompanionError::Conflict(
                            "The saved declaration is unavailable in the current index".into(),
                        )
                    })?;
                Ok(serde_json::json!({"snapshot":request.snapshot,"symbol":symbol}))
            }
            Operation::Areas { search, offset } => serde_json::to_value(
                index
                    .areas(search, *offset)
                    .map_err(|e| CompanionError::Invalid(e.to_string()))?,
            ),
            Operation::Inventory {
                area,
                search,
                offset,
                functions_only,
            } => {
                let mut page = (if *functions_only {
                    index.functions(area, search, *offset)
                } else {
                    index.inventory(area, search, *offset)
                })
                .map_err(|e| CompanionError::Invalid(e.to_string()))?;
                if scoped.is_some() {
                    let scope = serde_json::to_value(&page.coverage)
                        .map_err(|e| CompanionError::Unavailable(e.to_string()))?;
                    page.coverage = cache.index.coverage().clone();
                    serde_json::to_value(page).map(|mut value| {
                        value["scope_coverage"] = scope;
                        value
                    })
                } else {
                    serde_json::to_value(page)
                }
            }
            Operation::Calls {
                symbol,
                area,
                offset,
            } if foreign.is_some() => serde_json::to_value(
                foreign
                    .as_ref()
                    .ok_or_else(|| CompanionError::Unavailable("Dependency source expired".into()))?
                    .calls(symbol, *offset)
                    .map_err(CompanionError::Unavailable)?,
            ),
            Operation::Calls {
                symbol,
                area,
                offset,
            } => serde_json::to_value(
                index
                    .outgoing_with_candidates(
                        symbol,
                        area,
                        *offset,
                        scoped.as_ref().map(|_| &cache.index),
                    )
                    .map_err(|e| CompanionError::Invalid(e.to_string()))?,
            ),
            Operation::Uses {
                symbol,
                area,
                offset,
            } => serde_json::to_value(
                index
                    .uses(symbol, area, *offset)
                    .map_err(|e| CompanionError::Invalid(e.to_string()))?,
            ),
        }
        .map_err(|e| CompanionError::Unavailable(e.to_string()))?;
        if foreign.is_some() {
            value["snapshot"] = request.snapshot.clone().into();
        }
        if self
            .host
            .application()
            .snapshot()
            .is_none_or(|current| current.id() != snapshot.id())
        {
            return Err(CompanionError::Conflict(
                "Source changed during exploration; choose the area again".into(),
            ));
        }
        Ok(value)
    }
}

/// Definition lookup accepts a native call identity, never a browser-supplied code position.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionRequest {
    /// Current captured observation.
    pub snapshot: String,
    /// Call identity from the structured call page.
    pub call: String,
    /// Exact captured path hint for a call outside the repository projection.
    #[serde(default)]
    pub path: Option<String>,
}
/// Definition result precision, including known destinations outside captured source.
#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionState {
    /// One complete captured destination.
    Unique,
    /// Several semantic destinations or omitted/outside destinations.
    Ambiguous,
    /// No semantic definition was returned.
    Unresolved,
    /// No captured destination represents the known definition.
    Outside,
    /// Provider is absent or could not analyse this call.
    Unavailable,
}
/// Version-bound semantic selection or an explicitly weaker fallback.
#[derive(serde::Serialize)]
pub struct DefinitionResult {
    /// Exact source observation.
    pub snapshot: String,
    /// Result precision.
    pub state: DefinitionState,
    /// Original witness with semantic candidates when available.
    pub invocation: vizzi_core::structure::Invocation,
    /// Optional reason for fallback or unavailable source.
    pub detail: Option<String>,
}
impl StructureHost {
    /// Configures a host definition provider; ordinary metadata queries remain independent.
    pub fn with_definitions(
        host: Arc<Host>,
        provider: Arc<dyn crate::definitions::Provider>,
    ) -> Self {
        Self {
            reference_sets: Mutex::new(Vec::new()),
            host,
            cache: Mutex::new(None),
            definitions: Some(provider),
        }
    }
    /// Whether type-aware navigation is explicitly configured.
    pub fn definitions_available(&self) -> bool {
        self.definitions.is_some()
    }
    pub(crate) fn supports_java_project_import(&self) -> bool {
        self.definitions
            .as_ref()
            .is_some_and(|provider| provider.supports_java_project_import())
    }
    pub(crate) fn java_project(&self) -> Option<serde_json::Value> {
        let snapshot = self.host.application().snapshot()?;
        let kind = self.definitions.as_ref()?.java_project_kind(&snapshot)?;
        Some(serde_json::json!({"snapshot":snapshot.id().as_str(), "kind":kind}))
    }
    pub(crate) fn import_java_project(&self, expected: &str) -> Result<(), CompanionError> {
        let snapshot = self
            .host
            .application()
            .snapshot()
            .ok_or_else(|| CompanionError::Unavailable("Wait for source indexing".into()))?;
        if snapshot.id().as_str() != expected {
            return Err(CompanionError::Conflict(
                "Source changed before Java project import".into(),
            ));
        }
        self.definitions
            .as_ref()
            .ok_or_else(|| {
                CompanionError::Unavailable("Java project import requires managed JDT LS".into())
            })?
            .import_java_project(snapshot, &|| self.host.application().snapshot())
            .map_err(CompanionError::Unavailable)?;
        self.reference_sets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|set| !set.uses_java);
        Ok(())
    }
    pub(crate) fn call_languages(&self) -> &[String] {
        self.host.call_languages()
    }
    /// Resolves only the captured call, rejecting source changes before returning an action.
    pub fn definition(
        &self,
        request: &DefinitionRequest,
    ) -> Result<DefinitionResult, CompanionError> {
        let snapshot = self
            .host
            .application()
            .snapshot()
            .ok_or_else(|| CompanionError::Unavailable("Wait for source indexing".into()))?;
        if snapshot.id().as_str() != request.snapshot {
            return Err(CompanionError::Conflict(
                "Source changed before definition lookup".into(),
            ));
        }
        if request.call.len() != 64 {
            return Err(CompanionError::Invalid(
                "Invalid captured call identity".into(),
            ));
        }
        let call = {
            let mut cache = self
                .cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if cache
                .as_ref()
                .is_none_or(|c| c.snapshot != request.snapshot)
            {
                let previous = cache.take();
                *cache = Some(Cached::rebuild(&snapshot, previous));
            }
            let cache = cache
                .as_mut()
                .ok_or_else(|| CompanionError::Unavailable("Structure is unavailable".into()))?;
            let local = cache.index.call(&request.call);
            let path = local
                .as_ref()
                .map(|call| call.invocation.source.path.as_str());
            let scoped = match path.or(request.path.as_deref()) {
                Some(path) => cache.for_evidence(&snapshot, path)?,
                None => cache.scoped.as_ref().map(|(_, index)| index.clone()),
            };
            let local = scoped
                .and_then(|index| index.call_with_candidates(&request.call, Some(&cache.index)))
                .or(local);
            let foreign = if local.is_none() {
                self.external(&request.call)?
            } else {
                None
            };
            local
                .or_else(|| {
                    foreign
                        .as_ref()
                        .and_then(|s| s.index.call(&request.call))
                        .map(|mut call| {
                            if let Some(s) = &foreign {
                                call.invocation.candidates = call
                                    .invocation
                                    .candidates
                                    .into_iter()
                                    .map(|c| s.symbol(c))
                                    .collect();
                            }
                            call
                        })
                })
                .ok_or_else(|| {
                    CompanionError::Invalid("Call is absent from this observation".into())
                })?
        };
        let mut invocation = call.invocation.clone();
        let mut state = DefinitionState::Unavailable;
        let mut detail = None;
        if invocation.binding.is_some() {
            state = DefinitionState::Unique;
        } else if let Some(provider) = &self.definitions {
            match provider.resolve(snapshot.clone(), &call) {
                Err(e) => detail = Some(crate::review_application::clip(&e, 384).0),
                Ok(answer) => {
                    if answer.locations.len().saturating_add(answer.outside) > 64 {
                        return Err(CompanionError::Unavailable(
                            "Definition result exceeds its allowance".into(),
                        ));
                    }
                    if answer.context.len() != 64
                        || !answer
                            .context
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    {
                        return Err(CompanionError::Unavailable(
                            "Invalid semantic context identity".into(),
                        ));
                    }
                    let mut candidates = vec![];
                    let mut outside = answer.outside;
                    let mut cache = self
                        .cache
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let cache = cache
                        .as_mut()
                        .filter(|c| c.snapshot == request.snapshot)
                        .ok_or_else(|| {
                            CompanionError::Conflict(
                                "Source changed during definition lookup".into(),
                            )
                        })?;
                    let mut remaining = SEMANTIC_FILE_PROJECTIONS;
                    for location in answer.locations {
                        let found = if let Some(capture) = &location.capture {
                            if let Some(source) = self.dependency(capture)? {
                                if source.doc().path() != location.path {
                                    None
                                } else {
                                    let text = source.doc().resident_source().ok_or_else(|| {
                                        CompanionError::Unavailable(
                                            "Dependency text unavailable".into(),
                                        )
                                    })?;
                                    let byte =
                                        crate::definitions::byte_offset(text, location.position)
                                            .map_err(CompanionError::Unavailable)?;
                                    source
                                        .index
                                        .definition(&location.path, byte)
                                        .map(|symbol| source.symbol(symbol))
                                }
                            } else {
                                None
                            }
                        } else if let Some(doc) = snapshot.file_by_path(&location.path) {
                            let text = doc
                                .captured()
                                .read_text(8 * 1024 * 1024)
                                .map_err(|e| {
                                    CompanionError::Unavailable(format!(
                                        "Cannot verify captured definition: {e}"
                                    ))
                                })?
                                .ok_or_else(|| {
                                    CompanionError::Unavailable(
                                        "Definition source is not UTF-8".into(),
                                    )
                                })?;
                            let byte = crate::definitions::byte_offset(&text, location.position)
                                .map_err(CompanionError::Unavailable)?;
                            cache
                                .at_location(&snapshot, doc, &mut remaining, |index| {
                                    index.definition(doc.path(), byte)
                                })?
                                .flatten()
                        } else {
                            None
                        };
                        if let Some(symbol) = found {
                            if !candidates
                                .iter()
                                .any(|s: &vizzi_core::structure::Symbol| s.id == symbol.id)
                            {
                                candidates.push(symbol);
                            }
                        } else {
                            outside += 1;
                        }
                    }
                    let total = candidates.len() + outside;
                    if total == 0 {
                        state = DefinitionState::Unresolved;
                        detail=Some("The language server could not establish a definition; these are name matches.".into());
                    } else {
                        state = if candidates.is_empty() {
                            DefinitionState::Outside
                        } else if total == 1 {
                            DefinitionState::Unique
                        } else {
                            DefinitionState::Ambiguous
                        };
                        invocation.binding =
                            Some(vizzi_core::calls::TargetBasis::LanguageServerDefinition);
                        invocation.semantic_context = Some(answer.context);
                        invocation.candidate_count = total;
                        invocation.candidates = candidates.into_iter().take(8).collect();
                        if outside > 0 {
                            detail = Some(format!(
                                "{outside} definition(s) could not be mapped to captured declarations."
                            ));
                        }
                    }
                }
            }
        } else {
            detail =
                Some("Type-aware navigation is not configured; these are name matches.".into());
        }
        if self
            .host
            .application()
            .snapshot()
            .is_none_or(|now| now.id() != snapshot.id())
        {
            return Err(CompanionError::Conflict(
                "Source changed during definition lookup".into(),
            ));
        }
        Ok(DefinitionResult {
            snapshot: request.snapshot.clone(),
            state,
            invocation,
            detail,
        })
    }
}
