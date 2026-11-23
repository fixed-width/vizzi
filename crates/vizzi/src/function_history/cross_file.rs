//! Bounded file-scoped name candidates from one owned Git commit.
use super::*;
use captured_calls::{destination_frame, invocation};

const ADDITIONAL_FILES: usize = 8;
const ADDITIONAL_BYTES: usize = 4 * 1024 * 1024;
const FILE_PAGE: usize = 24;
const CANDIDATE_PAGE: usize = 8;

struct ScopedFile {
    owner: CapturedOwner,
    file: FileCapture,
}

#[derive(Default)]
pub(super) struct AdditionalFiles {
    retained: Vec<ScopedFile>,
    prepared: Option<ScopedFile>,
}

impl AdditionalFiles {
    fn all(&self) -> impl Iterator<Item = &ScopedFile> {
        self.retained.iter().chain(self.prepared.iter())
    }

    pub(super) fn bytes(&self) -> usize {
        self.all().map(|item| item.file.document().bytes()).sum()
    }

    pub(super) fn find(
        &self,
        owner: Option<&CapturedOwner>,
        snapshot: &str,
    ) -> Option<&FileCapture> {
        self.all()
            .find(|item| {
                owner.is_none_or(|owner| item.owner == *owner)
                    && item.file.snapshot.id().as_str() == snapshot
            })
            .map(|item| &item.file)
    }

    fn stage(
        &mut self,
        owner: &CapturedOwner,
        file: FileCapture,
        allowance: usize,
    ) -> Result<bool, CompanionError> {
        let snapshot = file.snapshot.id();
        if self
            .retained
            .iter()
            .any(|item| item.owner == *owner && item.file.snapshot.id() == snapshot)
        {
            return Ok(false);
        }
        let bytes = file.document().bytes();
        let occupied: usize = self
            .retained
            .iter()
            .map(|item| item.file.document().bytes())
            .sum();
        let replaceable = self
            .prepared
            .as_ref()
            .map_or(0, |item| item.file.document().bytes());
        if self.retained.len() >= ADDITIONAL_FILES
            || occupied + bytes > ADDITIONAL_BYTES
            || bytes > allowance + replaceable
        {
            return Err(conflict(
                "Historical files exceed the retained source allowance",
            ));
        }
        self.prepared = Some(ScopedFile {
            owner: owner.clone(),
            file,
        });
        Ok(true)
    }

    fn confirm(&mut self, source: &FrozenSource) {
        if self.prepared.as_ref().is_some_and(|item| {
            item.owner == source.owner
                && item
                    .file
                    .matches(&source.snapshot, &source.file, &source.version)
        }) {
            self.retained.extend(self.prepared.take());
        }
    }
}

pub(super) struct RevisionFiles<'a> {
    pub(super) primary: FileCapture,
    pub(super) commit: &'a str,
    pub(super) files: &'a mut AdditionalFiles,
    pub(super) allowance: usize,
}

impl RevisionFiles<'_> {
    fn source(&self, request: &FrozenSource) -> Result<&FileCapture, CompanionError> {
        let file = self
            .files
            .find(Some(&request.owner), &request.snapshot)
            .unwrap_or(&self.primary);
        if !file.matches(&request.snapshot, &request.file, &request.version) {
            return Err(conflict("Captured source differs from this revision"));
        }
        Ok(file)
    }

    fn by_path(&self, owner: &CapturedOwner, path: &str) -> Option<&FileCapture> {
        if self.primary.document().path() == path {
            return Some(&self.primary);
        }
        self.files
            .all()
            .find(|item| item.owner == *owner && item.file.document().path() == path)
            .map(|item| &item.file)
    }
}

impl FunctionHistory {
    fn with_revision_files<T>(
        &self,
        owner: &CapturedOwner,
        operation: impl FnOnce(RevisionFiles<'_>) -> Result<T, CompanionError>,
    ) -> Result<T, CompanionError> {
        match owner {
            CapturedOwner::Comparison { token, side } => {
                self.with_comparison_revision(token, side, operation)
            }
            CapturedOwner::History { token, index } => {
                let mut sessions = self.lock()?;
                let (queue, limit) = if sessions
                    .retained
                    .iter()
                    .any(|session| session.token == *token)
                {
                    (&mut sessions.retained, TOTAL_BYTES)
                } else {
                    (&mut sessions.prepared, PREPARED_BYTES)
                };
                let allowance = limit.saturating_sub(queue.iter().map(size).sum());
                let session = queue
                    .iter_mut()
                    .find(|session| session.token == *token)
                    .ok_or_else(|| conflict("Captured history expired; return to live"))?;
                let version = session
                    .versions
                    .get(*index)
                    .ok_or_else(|| conflict("Captured history version is unavailable"))?;
                let primary = self.restore(session, *index)?.file;
                operation(RevisionFiles {
                    primary,
                    commit: &version.commit,
                    files: &mut session.files,
                    allowance,
                })
            }
        }
    }

    pub(super) fn cross_file_query(
        &self,
        request: &CapturedRequest,
    ) -> Result<Value, CompanionError> {
        let source = &request.source;
        self.with_revision_files(&source.owner, |context| {
            let caller = context.source(source)?;
            let index = caller.structure();
            if index.resolve(None, &source.symbol).is_none() {
                return Err(conflict("Captured declaration differs from the requested file"));
            }
            if matches!(request.query, CapturedQuery::ConfirmFile) {
                context.files.confirm(source);
                return Ok(Value::Null);
            }
            let call = match &request.query {
                CapturedQuery::Files { call, .. } | CapturedQuery::Candidates { call, .. } | CapturedQuery::OpenFile { call, .. } => call,
                _ => return Err(invalid("Expected a cross-file query")),
            };
            let mut site = invocation(&index, call, &source.symbol)?;
            let name = site.name.as_deref().ok_or_else(|| invalid("This call has no captured name"))?;
            match &request.query {
                CapturedQuery::Files { directory, offset, .. } => self.revision_files(context.commit, directory, *offset),
                CapturedQuery::Candidates { path, offset, .. } => {
                    let file = self.additional_file(&context, source, path)?;
                    let index = file.structure();
                    let matching = index.named_callees(name).count();
                    let items: Vec<_> = index.named_callees(name).skip(*offset).take(CANDIDATE_PAGE).collect();
                    let next = continuation(*offset, items.len(), matching);
                    Ok(json!({"commit":context.commit,"snapshot":file.snapshot.id().as_str(),"path":path,"items":items,"matching":matching,"next":next,"coverage":index.coverage()}))
                }
                CapturedQuery::OpenFile { path, snapshot, target, .. } => {
                    let file = self.additional_file(&context, source, path)?;
                    if file.snapshot.id().as_str() != snapshot {
                        return Err(conflict("Candidate file differs from the requested observation"));
                    }
                    let index = file.structure();
                    let destination = index.named_callees(name).find(|symbol| symbol.id == *target)
                        .ok_or_else(|| conflict("Destination is not a candidate of the captured call in this file"))?;
                    let mut frame = destination_frame(&file, destination, &source.owner, context.commit)?;
                    frame["cross_file"] = json!(true);
                    site.binding = None;
                    site.candidates = vec![destination.clone()];
                    site.candidate_count = index.named_callees(name).count();
                    let prepared = if file.snapshot.id() == context.primary.snapshot.id() {
                        false
                    } else {
                        context.files.stage(&source.owner, file, context.allowance)?
                    };
                    Ok(json!({"invocation":site,"frame":frame,"prepared":prepared}))
                }
                _ => Err(invalid("Expected a cross-file query")),
            }
        })
    }

    fn revision_files(
        &self,
        commit: &str,
        directory: &str,
        offset: usize,
    ) -> Result<Value, CompanionError> {
        if !directory.is_empty() {
            validate_path(directory)?;
        }
        let repository = GitRepository::open(self.host.root()).map_err(unavailable)?;
        let entries = repository
            .revision_directory(commit, directory)
            .map_err(|e| {
                unavailable(format!(
                    "Directory listing unavailable. Enter the exact file path. {e}"
                ))
            })?;
        let entries: Vec<_> = entries
            .into_iter()
            .filter(|(path, _)| {
                self.host
                    .source_policy()
                    .excludes(path)
                    .is_ok_and(|excluded| !excluded)
            })
            .collect();
        let items: Vec<_> = entries
            .iter()
            .skip(offset)
            .take(FILE_PAGE)
            .map(|(path, directory)| json!({"path":path,"directory":directory}))
            .collect();
        let next = continuation(offset, items.len(), entries.len());
        Ok(json!({"commit":commit,"directory":directory,"items":items,"next":next}))
    }

    fn additional_file(
        &self,
        context: &RevisionFiles<'_>,
        source: &FrozenSource,
        path: &str,
    ) -> Result<FileCapture, CompanionError> {
        validate_path(path)?;
        let caller = context.source(source)?;
        if caller.document().path() == path {
            return Err(invalid(
                "Choose a different file or use the current file's call choices",
            ));
        }
        if let Some(file) = context.by_path(&source.owner, path) {
            return Ok(file.clone());
        }
        let classification = self
            .host
            .source_policy()
            .classify(path)
            .map_err(unavailable)?
            .ok_or_else(|| invalid("File is outside the configured source scope"))?;
        let repository = GitRepository::open(self.host.root()).map_err(unavailable)?;
        let snapshot = self
            .analyzed_revision_file(&repository, context.commit, path, classification)?
            .ok_or_else(|| invalid("File is missing or is not a regular file at this commit"))?;
        if !snapshot.documents()[0]
            .analyses()
            .iter()
            .any(|analysis| matches!(analysis.coverage, Coverage::Syntax | Coverage::Partial))
        {
            return Err(invalid("Declaration analysis is unavailable for this file"));
        }
        let modules = Arc::new(ModuleIndex::empty(snapshot.workspace()));
        Ok(FileCapture { snapshot, modules })
    }
}

fn validate_path(path: &str) -> Result<(), CompanionError> {
    if path.len() > 4096 || path.chars().any(|ch| ch == '\\' || ch.is_control()) {
        return Err(invalid(
            "Enter a repository-relative path of at most 4,096 bytes",
        ));
    }
    ReviewPath::parse(path.as_bytes().to_vec()).map_err(|e| invalid(e.to_string()))?;
    Ok(())
}

fn continuation(offset: usize, shown: usize, matching: usize) -> Option<usize> {
    let end = offset.saturating_add(shown);
    (end < matching).then_some(end)
}
