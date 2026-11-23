//! Reader-confirmed correspondence across one immutable Git edge.
use super::*;
use std::collections::BTreeMap;
use vizzi_core::model::{Analysis, Fact};

const CHANGED_FILES: usize = 16;
const SEARCH_BYTES: usize = 8 * 1024 * 1024;
const TRANSITIONS: usize = 4;

/// Inspects or accepts an adjacent history correspondence.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LineageRequest {
    /// Find a candidate at the immediate parent without changing the timeline.
    Preview {
        /// Exact accepted version on the current side.
        source: HistoryBinding,
    },
    /// Record the exact candidate the reader selected in the preview.
    Accept {
        /// Exact accepted version on the current side.
        source: HistoryBinding,
        /// Parent file observation shown in the preview.
        snapshot: String,
        /// Exact declaration shown in that observation.
        target: String,
    },
}

#[derive(Serialize)]
pub(super) struct Edge {
    at: usize,
    newer: Pin,
    older: Pin,
}

#[derive(Serialize)]
struct Pin {
    commit: String,
    snapshot: String,
    file: String,
    tracking: String,
}

impl Pin {
    fn new(commit: &str, capture: &Capture) -> Self {
        Self {
            commit: commit.into(),
            snapshot: capture.file.snapshot.id().as_str().into(),
            file: capture.symbol.source.file.clone(),
            tracking: capture.symbol.tracking.clone(),
        }
    }
}

struct Match {
    declaration: Declaration,
    location: Location,
    name: String,
    unique: bool,
    digest: blake3::Hash,
}

type Snapshots = BTreeMap<String, Arc<Snapshot>>;

impl FunctionHistory {
    /// Verifies source evidence and changes history only on an explicit, exact acceptance.
    pub fn lineage(&self, request: &LineageRequest) -> Result<Value, CompanionError> {
        let source = match request {
            LineageRequest::Preview { source } | LineageRequest::Accept { source, .. } => source,
        };
        let mut sessions = self.lock()?;
        let (queue, limit) = if sessions
            .retained
            .iter()
            .any(|session| session.token == source.token)
        {
            (&mut sessions.retained, TOTAL_BYTES)
        } else {
            (&mut sessions.prepared, PREPARED_BYTES)
        };
        let unused = limit.saturating_sub(queue.iter().map(size).sum());
        let session = queue
            .iter_mut()
            .find(|session| session.token == source.token)
            .ok_or_else(|| conflict("Function history expired; return to live"))?;
        if source.index.checked_add(1) != Some(session.versions.len()) {
            return Err(conflict(
                "Choose the oldest accepted version before finding an earlier declaration",
            ));
        }
        if session.lineage.len() >= TRANSITIONS || session.versions.len() >= HISTORY_REVISIONS {
            return Err(invalid("Function history correspondence limit reached"));
        }
        let current = self.restore(session, source.index)?;
        if !current
            .file
            .matches(&source.snapshot, &source.file, &source.version)
        {
            return Err(conflict(
                "Historical source differs from the requested declaration",
            ));
        }
        let version = &session.versions[source.index];
        let commit = &version.commit;
        let explored = session.explored
            + session
                .commits
                .iter()
                .position(|old| old == commit)
                .ok_or_else(|| invalid("Choose the commit that changed the declaration's file"))?
            + 1;
        let remaining = HISTORY_REVISIONS.saturating_sub(explored);
        if remaining == 0 {
            return Err(invalid(
                "Function history limit reached (64 file revisions)",
            ));
        }
        let parent = session
            .repository
            .sole_parent(commit)
            .map_err(unavailable)?
            .ok_or_else(|| {
                invalid("This commit has no single available parent for correspondence")
            })?;
        let Match {
            declaration,
            location,
            digest,
            ..
        } = self.earlier_match(session, &current, commit, &parent)?;
        match request {
            LineageRequest::Preview { .. } => {
                let earlier = declaration
                    .bind(Arc::new(ModuleIndex::empty(session.repository.workspace())))?;
                Ok(
                    json!({"commit":parent,"snapshot":earlier.file.snapshot.id().as_str(),"symbol":earlier.symbol}),
                )
            }
            LineageRequest::Accept {
                snapshot, target, ..
            } => {
                if declaration.snapshot.id().as_str() != snapshot {
                    return Err(conflict("Earlier declaration differs from the preview"));
                }
                let (older_segment, boundary) =
                    segment_history(&session.repository, &parent, &location.path, remaining)?;
                let (version, capture) =
                    self.history_capture(session, location.clone(), parent, declaration)?;
                if capture.symbol.id != *target {
                    return Err(conflict("Earlier declaration differs from the preview"));
                }
                let replaceable = session
                    .selected
                    .as_ref()
                    .map_or(0, |old| old.file.document().bytes());
                if capture.file.document().bytes() > unused + replaceable {
                    return Err(conflict(
                        "Earlier declaration exceeds the retained source allowance",
                    ));
                }
                session.lineage.push(Edge {
                    at: source.index + 1,
                    newer: Pin::new(commit, &current),
                    older: Pin::new(&version.commit, &capture),
                });
                session.location = location;
                session.commits = older_segment;
                session.explored = explored;
                session.cursor = 0;
                session.boundary = boundary;
                session.stopped = false;
                session.push(version, capture, digest);
                frame(session)
            }
        }
    }

    fn earlier_match(
        &self,
        session: &Session,
        current: &Capture,
        commit: &str,
        parent: &str,
    ) -> Result<Match, CompanionError> {
        let current_doc = current.file.document();
        let analysis = complete_analysis(current_doc, &session.language)?;
        let current_fact = analysis
            .facts
            .iter()
            .find(|fact| {
                fact.unique
                    && fact.kind == "function"
                    && current_doc.entity_id(analysis, fact).as_str() == current.symbol.tracking
            })
            .ok_or_else(|| invalid("This declaration lacks an unambiguous lexical binding"))?;
        let scope = lexical_scope(current_fact)?;
        let (shape, _) = declaration_shape(current_doc, current_fact)?.ok_or_else(|| {
            invalid("This function lacks complete declaration source for correspondence")
        })?;
        let (before, after) = self.edge_sources(session, current, commit, parent)?;
        if !after.contains_key(current_doc.path())
            || contains_name(
                &before,
                current_doc.path(),
                &scope,
                &current_fact.name,
                &session.language,
            )?
        {
            return Err(invalid(
                "The declaration continues at this path. Use Previous change.",
            ));
        }
        let older = unique_match(&before, &session.language, &scope, shape)?;
        let newer = unique_match(&after, &session.language, &scope, shape)?;
        if newer.declaration.tracking.as_str() != current.symbol.tracking
            || contains_name(
                &after,
                &older.location.path,
                &scope,
                &older.name,
                &session.language,
            )?
        {
            return Err(invalid(
                "No unambiguous rename or move at this history boundary",
            ));
        }
        Ok(older)
    }

    fn edge_sources(
        &self,
        session: &Session,
        current: &Capture,
        commit: &str,
        parent: &str,
    ) -> Result<(Snapshots, Snapshots), CompanionError> {
        let paths = session
            .repository
            .changed_paths(parent, commit)
            .map_err(unavailable)?;
        let mut relevant = Vec::new();
        for path in &paths {
            let name = path
                .document_path()
                .ok_or_else(|| invalid("A changed path cannot be analysed for correspondence"))?;
            if let Some(classification) = self
                .host
                .source_policy()
                .classify(name)
                .map_err(unavailable)?
                && classification.assigned_language() == Some(session.language.as_str())
            {
                relevant.push((name, classification));
                if relevant.len() > CHANGED_FILES {
                    return Err(invalid("Too many changed files to verify a rename or move"));
                }
            }
        }
        let mut bytes = 0;
        let mut collect = |revision: &str| -> Result<Snapshots, CompanionError> {
            let mut observations = BTreeMap::new();
            for (path, classification) in &relevant {
                let capture = if revision == commit && *path == current.file.document().path() {
                    Some(current.file.snapshot.clone())
                } else {
                    self.analyzed_revision_file(
                        &session.repository,
                        revision,
                        path,
                        classification.clone(),
                        FILE_BYTES.min(SEARCH_BYTES - bytes),
                    )?
                };
                if let Some(capture) = capture {
                    bytes += capture.documents()[0].bytes();
                    if bytes > SEARCH_BYTES {
                        return Err(invalid(
                            "Changed source exceeds the correspondence byte allowance",
                        ));
                    }
                    observations.insert((*path).into(), capture);
                }
            }
            Ok(observations)
        };
        Ok((collect(parent)?, collect(commit)?))
    }
}

fn complete_analysis<'a>(
    doc: &'a Document,
    language: &str,
) -> Result<&'a Analysis, CompanionError> {
    let mut analyses = doc
        .analyses()
        .iter()
        .filter(|analysis| analysis.context.language == language);
    let analysis = analyses
        .next()
        .filter(|analysis| analysis.coverage == Coverage::Syntax)
        .ok_or_else(|| invalid("Changed source lacks complete syntax for correspondence"))?;
    if analyses.next().is_some() {
        return Err(invalid("Changed source has ambiguous analysis"));
    }
    Ok(analysis)
}

fn lexical_scope(fact: &Fact) -> Result<Vec<String>, CompanionError> {
    let unsupported = || invalid("A function lacks a usable parsed lexical scope");
    let (key, _ordinal): (String, usize) =
        serde_json::from_str(&fact.key).map_err(|_| unsupported())?;
    let mut scope: Vec<String> = serde_json::from_str(&key).map_err(|_| unsupported())?;
    let leaf = scope.pop().ok_or_else(unsupported)?;
    let owner = leaf
        .strip_prefix("function:")
        .and_then(|identity| identity.strip_suffix(&fact.name))
        .ok_or_else(unsupported)?;
    scope.push(owner.into());
    scope.push(
        fact.name
            .rsplit_once("::")
            .map_or("", |(qualifier, _)| qualifier)
            .into(),
    );
    Ok(scope)
}

fn declaration_shape(
    doc: &Document,
    fact: &Fact,
) -> Result<Option<(blake3::Hash, blake3::Hash)>, CompanionError> {
    let (Some(complete), Some(name)) = (&fact.declaration_span, &fact.name_span) else {
        return Ok(None);
    };
    if name.start < fact.span.start || name.end > fact.span.end {
        return Ok(None);
    }
    let bytes = doc
        .captured()
        .reader()
        .read(complete.clone(), FILE_BYTES)
        .map_err(unavailable)?;
    let range = name.start - complete.start..name.end - complete.start;
    if bytes.get(range.clone()) != Some(fact.name.as_bytes()) {
        return Ok(None);
    }
    let prefix = &bytes[..range.start];
    let mut shape = blake3::Hasher::new();
    shape.update(&(prefix.len() as u64).to_le_bytes());
    shape.update(prefix);
    shape.update(&bytes[range.end..]);
    let displayed = fact.span.start - complete.start..fact.span.end - complete.start;
    Ok(Some((shape.finalize(), blake3::hash(&bytes[displayed]))))
}

fn unique_match(
    snapshots: &Snapshots,
    language: &str,
    scope: &[String],
    shape: blake3::Hash,
) -> Result<Match, CompanionError> {
    let mut result = None;
    for (path, snapshot) in snapshots {
        let doc = &snapshot.documents()[0];
        let analysis = complete_analysis(doc, language)?;
        for fact in analysis.facts.iter().filter(|fact| fact.kind == "function") {
            if lexical_scope(fact)?.as_slice() != scope {
                continue;
            }
            let (fingerprint, digest) = declaration_shape(doc, fact)?.ok_or_else(|| {
                invalid(
                    "A function in this scope lacks complete declaration source for correspondence",
                )
            })?;
            if fingerprint != shape {
                continue;
            }
            if result.is_some() {
                return Err(invalid(
                    "Several declarations match the same source evidence",
                ));
            }
            result = Some(Match {
                declaration: Declaration {
                    snapshot: snapshot.clone(),
                    tracking: doc.entity_id(analysis, fact),
                    span: fact.span.clone(),
                },
                location: Location {
                    path: path.clone(),
                    key: fact.key.clone(),
                    classification: doc.classification().clone(),
                },
                name: fact.name.clone(),
                unique: fact.unique,
                digest,
            });
        }
    }
    result
        .filter(|candidate| candidate.unique)
        .ok_or_else(|| invalid("No unambiguous earlier declaration matches the source evidence"))
}

fn contains_name(
    snapshots: &Snapshots,
    path: &str,
    scope: &[String],
    name: &str,
    language: &str,
) -> Result<bool, CompanionError> {
    let Some(snapshot) = snapshots.get(path) else {
        return Ok(false);
    };
    let analysis = complete_analysis(&snapshot.documents()[0], language)?;
    for fact in analysis
        .facts
        .iter()
        .filter(|fact| fact.kind == "function" && fact.name == name)
    {
        if lexical_scope(fact)?.as_slice() == scope {
            return Ok(true);
        }
    }
    Ok(false)
}
