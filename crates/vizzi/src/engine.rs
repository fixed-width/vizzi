//! Bounded parallel analysis with immutable reuse and disposable persistence.

use crate::{
    admission::{Admission, Priority, Refusal},
    cache::Cache,
    source::{CaptureScope, Directory, MAX_PATH_HINTS},
    syntax_cache::{SyntaxCache, SyntaxKey},
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use vizzi_core::{
    analysis::{AnalysisResult, Analyzer},
    model::{Analysis, Coverage, Document, ModelError, Snapshot},
};

/// Native source/analysis failure; the host retains its previous snapshot.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// Creating a fresh notification stream identity failed.
    #[error("cannot initialise change stream: {0}")]
    Changes(#[from] getrandom::Error),
    /// Reading the source directory failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Source/fact validation failed.
    #[error(transparent)]
    Model(#[from] ModelError),
    /// Invalid configuration or a worker panic.
    #[error("{0}")]
    Configuration(String),
}
/// Measurements of an entire capture and analysis attempt.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Measurements {
    /// Indexed files whose analysis is deferred by resource admission.
    pub deferred: usize,
    /// Total indexed source files in the candidate snapshot.
    pub files: usize,
    /// Captured files containing analysis records, including unavailable analysis.
    pub analyzed: usize,
    /// Captured files with no analyzer assigned.
    pub source_only: usize,
    /// Source bytes read and verified, including unchanged files.
    pub bytes: usize,
    /// Source files reopened and verified during this attempt.
    pub read_files: usize,
    /// Reopened inputs sharing verified byte/index storage with the baseline.
    pub shared_sources: usize,
    /// Unread documents retained using the supplied hints, not metadata proof.
    pub retained: usize,
    /// Whether this attempt rediscovered and verified the complete source scope.
    pub reconciled: bool,
    /// Files requiring syntax parsing.
    pub parsed: usize,
    /// Parsed files supplied with a compatible retained syntax tree.
    pub tree_reuses: usize,
    /// Files reusing immutable in-memory documents.
    pub reused: usize,
    /// Files reusing persisted analysis.
    pub cache_hits: usize,
    /// Nonfatal cache write failures.
    pub cache_warnings: usize,
    /// Source discovery/candidate inspection and read wall time.
    pub capture_ms: f64,
    /// Capture, analysis, cache, and snapshot-construction wall time.
    pub total_ms: f64,
}
/// A validated refresh ready for a publication-ticket check.
#[derive(Clone)]
pub struct Refresh {
    /// Complete immutable observation.
    pub snapshot: Arc<Snapshot>,
    /// Directories for native observation.
    pub directories: Option<Vec<PathBuf>>,
    /// Capture and analysis measurements.
    pub measurements: Measurements,
}
/// The native analysis boundary; no host transport or mutable publication state.
pub struct Engine {
    directory: Directory,
    cache: Cache,
    workers: usize,
    syntax_cache: Option<Arc<SyntaxCache>>,
    admission: Option<Arc<Admission>>,
}
impl Engine {
    /// Configures 1–32 workers shared across languages, with worker-local parsers.
    pub fn new(directory: Directory, cache: Cache, workers: usize) -> Result<Self, EngineError> {
        if !(1..=32).contains(&workers) {
            return Err(EngineError::Configuration(
                "workers must be in 1..=32".into(),
            ));
        }
        Ok(Self {
            directory,
            cache,
            workers,
            syntax_cache: None,
            admission: None,
        })
    }
    /// Enables explicitly shared, bounded retained syntax independently of persistence.
    pub fn with_syntax_cache(mut self, cache: Arc<SyntaxCache>) -> Self {
        self.syntax_cache = Some(cache);
        self
    }
    /// Shares an opt-in active-input admission budget across attempts and languages.
    pub fn with_admission(mut self, admission: Arc<Admission>) -> Self {
        self.admission = Some(admission);
        self
    }
    /// Returns the source adapter used by the watcher.
    pub fn directory(&self) -> &Directory {
        &self.directory
    }
    pub(crate) fn workers(&self) -> usize {
        self.workers
    }
    /// Captures source, reuses validated analysis, and checks cancellation between jobs.
    pub fn refresh(
        &self,
        previous: Option<&Snapshot>,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<Option<Refresh>, EngineError> {
        self.refresh_scoped(previous, &CaptureScope::Reconcile, cancelled)
    }

    /// Verifies a path hint set or reconciles against an immutable baseline.
    /// A different source policy always forces complete discovery.
    pub fn refresh_scoped(
        &self,
        previous: Option<&Snapshot>,
        scope: &CaptureScope,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<Option<Refresh>, EngineError> {
        self.refresh_budget(
            previous,
            scope,
            cancelled,
            self.workers,
            Priority::Foreground,
        )
    }
    pub(crate) fn refresh_budget(
        &self,
        previous: Option<&Snapshot>,
        scope: &CaptureScope,
        cancelled: &(dyn Fn() -> bool + Sync),
        workers: usize,
        priority: Priority,
    ) -> Result<Option<Refresh>, EngineError> {
        let workers = workers.clamp(1, self.workers);
        if cancelled() {
            return Ok(None);
        }
        let started = Instant::now();
        let previous =
            previous.filter(|snapshot| snapshot.workspace() == self.directory.workspace());
        let capture = match scope {
            CaptureScope::Paths(paths)
                if previous.is_some_and(|snapshot| self.can_capture_paths(snapshot, paths)) =>
            {
                self.directory.capture_reusing(previous, scope, cancelled)?
            }
            _ => self
                .directory
                .capture_reusing(previous, &CaptureScope::Reconcile, cancelled)?,
        };
        let Some(capture) = capture else {
            return Ok(None);
        };
        let mut measurements = Measurements {
            read_files: capture.files.len(),
            reconciled: capture.directories.is_some(),
            bytes: capture
                .files
                .iter()
                .map(|file| file.source.bytes().len())
                .sum(),
            capture_ms: milliseconds(started.elapsed()),
            ..Default::default()
        };
        let mut documents = Vec::with_capacity(capture.files.len());
        if let Some(cache) = &self.syntax_cache {
            for path in &capture.removed {
                cache.invalidate(self.directory.workspace(), path);
            }
        }
        if !measurements.reconciled {
            let touched: BTreeSet<_> = capture
                .files
                .iter()
                .map(|file| file.path.as_str())
                .chain(capture.removed.iter().map(String::as_str))
                .collect();
            if let Some(previous) = previous {
                measurements.retained = previous.documents().len()
                    - touched
                        .iter()
                        .filter(|path| previous.file_by_path(path).is_some())
                        .count();
                measurements.reused = measurements.retained;
            }
        }
        let mut jobs = Vec::new();
        for file in capture.files {
            if cancelled() {
                return Ok(None);
            }
            let content = file.source.content();
            let language = self.directory.language(&file.classification);
            let old = previous.and_then(|snapshot| snapshot.file_by_path(&file.path));
            if old.is_some_and(|old| Arc::ptr_eq(old.captured(), &file.source)) {
                measurements.shared_sources += 1;
            }
            if let Some(old) = old.filter(|old| {
                old.content() == content
                    && old.classification() == &file.classification
                    && match language {
                        Some(language) => {
                            old.analyses().len() == 1
                                && old.analyses()[0].coverage != Coverage::Deferred
                                && old.analyses()[0].context == *language.context()
                        }
                        None => old.analyses().is_empty(),
                    }
            }) {
                documents.push((*old).clone());
                measurements.reused += 1;
            } else if let Some(language) = language {
                jobs.push((file, language));
            } else {
                documents.push(Arc::new(Document::capture_source(
                    self.directory.workspace().clone(),
                    file.path,
                    file.source,
                    vec![],
                    file.classification,
                )?));
            }
        }
        jobs.sort_by_key(|job| std::cmp::Reverse(job.0.source.bytes().len()));
        let next = AtomicUsize::new(0);
        let batches = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers.min(jobs.len()))
                .map(|_| {
                    scope.spawn(|| -> Result<_, EngineError> {
                        let mut parsers = BTreeMap::<_, Box<dyn Analyzer>>::new();
                        let mut batch = Vec::new();
                        let mut counters = (0, 0, 0, 0);
                        loop {
                            if cancelled() {
                                break;
                            }
                            let index = next.fetch_add(1, Ordering::Relaxed);
                            let Some((file, language)) = jobs.get(index) else {
                                break;
                            };
                            let context = language.context();
                            let content = file.source.content();
                            let _permit = match self.admission.as_ref().map(|budget| budget.acquire(file.source.bytes().len(), priority)) {
                                Some(Ok(permit)) => Some(permit),
                                Some(Err(reason)) => {
                                    let diagnostic = match reason {
                                        Refusal::Busy => "Analysis deferred: active input capacity is occupied or reserved for foreground work; source remains available".into(),
                                        Refusal::FileLimit => format!("Analysis deferred: {} input bytes exceed the per-file admission limit; source remains available", file.source.bytes().len()),
                                    };
                                    batch.push(Arc::new(Document::capture_source(
                                        self.directory.workspace().clone(), file.path.clone(), file.source.clone(),
                                        vec![Analysis { content: content.clone(), context: context.clone(), coverage: Coverage::Deferred, facts: vec![], diagnostics: vec![diagnostic] }],
                                        file.classification.clone(),
                                    )?));
                                    continue;
                                }
                                None => None,
                            };
                            let cached = self
                                .cache
                                .load(&context.id, content)
                                .filter(|analysis| analysis.context == *context)
                                .and_then(|analysis| {
                                    Document::capture_source(
                                        self.directory.workspace().clone(),
                                        file.path.clone(),
                                        file.source.clone(),
                                        vec![analysis],
                                        file.classification.clone(),
                                    )
                                    .ok()
                                });
                            let document = if let Some(document) = cached {
                                counters.1 += 1;
                                document
                            } else {
                                let parser = match parsers.entry(context.id.clone()) {
                                    std::collections::btree_map::Entry::Occupied(entry) => {
                                        entry.into_mut()
                                    }
                                    std::collections::btree_map::Entry::Vacant(entry) => {
                                        entry.insert(language.create()?)
                                    }
                                };
                                let mut lease = self.syntax_cache.as_ref().and_then(|cache| {
                                    cache.lease(
                                        SyntaxKey::new(
                                            self.directory.workspace(),
                                            &file.path,
                                            context,
                                        ),
                                        !measurements.reconciled,
                                    )
                                });
                                let result = if let Some(lease) = &mut lease {
                                    parser.analyze_with_state(
                                        file.source.bytes(),
                                        lease.take_previous(),
                                        lease.allowance(),
                                        cancelled,
                                    )?
                                } else {
                                    parser
                                        .analyze_cancellable(file.source.bytes(), cancelled)?
                                        .map(|analysis| AnalysisResult {
                                            analysis,
                                            state: None,
                                            tree_reused: false,
                                        })
                                };
                                let Some(result) = result else {
                                    return Ok(None);
                                };
                                let analysis = result.analysis;
                                if analysis.context != *context {
                                    return Err(EngineError::Configuration(
                                        "analyzer returned an unregistered context".into(),
                                    ));
                                }
                                let document = Document::capture_source(
                                    self.directory.workspace().clone(),
                                    file.path.clone(),
                                    file.source.clone(),
                                    vec![analysis],
                                    file.classification.clone(),
                                )?;
                                if let Some(lease) = lease {
                                    lease.finish(result.state, content);
                                }
                                counters.0 += 1;
                                counters.3 += usize::from(result.tree_reused);
                                if self.cache.store(&document.analyses()[0]).is_err() {
                                    counters.2 += 1;
                                }
                                document
                            };
                            batch.push(Arc::new(document));
                        }
                        Ok(Some((batch, counters)))
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    handle.join().map_err(|_| {
                        EngineError::Configuration("analysis worker panicked".into())
                    })?
                })
                .collect::<Result<Vec<_>, EngineError>>()
        })?;
        if cancelled() {
            return Ok(None);
        }
        for batch in batches {
            let Some((batch, (parsed, hits, warnings, tree_reuses))) = batch else {
                return Ok(None);
            };
            documents.extend(batch);
            measurements.parsed += parsed;
            measurements.cache_hits += hits;
            measurements.cache_warnings += warnings;
            measurements.tree_reuses += tree_reuses;
        }
        let snapshot = Arc::new(if measurements.reconciled {
            self.directory
                .plan()
                .snapshot(self.directory.workspace().clone(), documents)?
        } else {
            self.directory.plan().update(
                previous.ok_or_else(|| {
                    EngineError::Configuration("partial capture lost its baseline".into())
                })?,
                documents,
                &capture.removed,
            )?
        });
        if let Some(cache) = &self.syntax_cache {
            cache.reconcile(&snapshot);
        }
        measurements.files = snapshot.documents().len();
        measurements.deferred = snapshot.deferred_files();
        measurements.analyzed = snapshot.analyzed_files();
        measurements.source_only = measurements.files - measurements.analyzed;
        measurements.total_ms = milliseconds(started.elapsed());
        Ok(Some(Refresh {
            snapshot,
            directories: capture.directories,
            measurements,
        }))
    }

    fn can_capture_paths(&self, snapshot: &Snapshot, paths: &BTreeSet<PathBuf>) -> bool {
        if snapshot.capture_plan() != Some(self.directory.plan().id())
            || paths.is_empty()
            || paths.len() > MAX_PATH_HINTS
        {
            return false;
        }
        for path in paths {
            let key = match self.directory.relative_key(path) {
                Ok(Some(key)) => key,
                Ok(None) => continue,
                Err(_) => return false,
            };
            if has_descendant(snapshot, &key) {
                return false;
            }
            if let Some((parent, _)) = key.rsplit_once('/')
                && !has_descendant(snapshot, parent)
            {
                return false;
            }
        }
        true
    }
}

fn has_descendant(snapshot: &Snapshot, path: &str) -> bool {
    let prefix = format!("{path}/");
    let index = snapshot
        .documents()
        .partition_point(|file| file.path() < prefix.as_str());
    snapshot
        .documents()
        .get(index)
        .is_some_and(|file| file.path().starts_with(&prefix))
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mixed_languages_share_cache_and_incremental_source_policy() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("a.rs"), "fn rust() {}").unwrap();
        std::fs::write(source.path().join("b.py"), "def python(): pass\n").unwrap();
        std::fs::write(source.path().join("skip.h"), "ambiguous header").unwrap();
        std::fs::write(source.path().join("skip.lpc"), "LPC").unwrap();
        let engine = engine(source.path(), cache.path(), 2);
        let first = engine.refresh(None, &|| false).unwrap().unwrap();
        assert_eq!(first.measurements.files, 2);
        assert_eq!(first.measurements.parsed, 2);
        assert_eq!(
            first.snapshot.documents()[0].analyses()[0].context.language,
            "rust"
        );
        assert_eq!(
            first.snapshot.documents()[1].analyses()[0].context.language,
            "python"
        );
        let warm = engine.refresh(None, &|| false).unwrap().unwrap();
        assert_eq!(warm.measurements.cache_hits, 2);
        assert_eq!(warm.snapshot.id(), first.snapshot.id());
        std::fs::write(source.path().join("b.py"), "def edited(): pass\n").unwrap();
        let edited = engine
            .refresh_scoped(
                Some(&first.snapshot),
                &CaptureScope::Paths(BTreeSet::from(["b.py".into()])),
                &|| false,
            )
            .unwrap()
            .unwrap();
        assert!(!edited.measurements.reconciled);
        assert_eq!(edited.measurements.read_files, 1);
        assert_eq!(edited.measurements.parsed, 1);
        assert_eq!(edited.measurements.retained, 1);
        assert_eq!(
            edited.snapshot.documents()[1].analyses()[0].facts[0].name,
            "edited"
        );
        assert!(Arc::ptr_eq(
            &first.snapshot.documents()[0],
            &edited.snapshot.documents()[0]
        ));
        std::fs::rename(source.path().join("b.py"), source.path().join("b.txt")).unwrap();
        std::fs::write(source.path().join("c.py"), "class Added: pass\n").unwrap();
        let renamed = engine
            .refresh_scoped(
                Some(&edited.snapshot),
                &CaptureScope::Paths(BTreeSet::from([
                    "b.py".into(),
                    "b.txt".into(),
                    "c.py".into(),
                ])),
                &|| false,
            )
            .unwrap()
            .unwrap();
        assert_eq!(renamed.measurements.read_files, 1);
        assert_eq!(renamed.measurements.retained, 1);
        let clean = engine.refresh(None, &|| false).unwrap().unwrap();
        assert_eq!(renamed.snapshot.id(), clean.snapshot.id());
        assert_eq!(
            renamed
                .snapshot
                .documents()
                .iter()
                .map(|doc| doc.path())
                .collect::<Vec<_>>(),
            ["a.rs", "c.py"]
        );
    }

    fn engine(source: &std::path::Path, cache: &std::path::Path, workers: usize) -> Engine {
        Engine::new(
            Directory::open(source).unwrap(),
            Cache::new(cache.into()),
            workers,
        )
        .unwrap()
    }
    #[test]
    fn cold_warm_and_single_file_edit_reuse_exact_facts() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("a.rs"), "fn a() {}").unwrap();
        std::fs::write(source.path().join("b.rs"), "fn b() {}").unwrap();
        let first = engine(source.path(), cache.path(), 2)
            .refresh(None, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(first.measurements.parsed, 2);
        assert_eq!(first.measurements.files, 2);
        assert!(first.measurements.capture_ms > 0.0);
        assert!(first.measurements.total_ms >= first.measurements.capture_ms);
        assert_eq!(first.measurements.bytes, 18);
        let second = engine(source.path(), cache.path(), 1)
            .refresh(None, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(second.measurements.cache_hits, 2);
        assert_eq!(second.measurements.parsed, 0);
        assert_eq!(first.snapshot.id(), second.snapshot.id());
        let engine = engine(source.path(), cache.path(), 2);
        let unchanged = engine
            .refresh(Some(&second.snapshot), &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(unchanged.measurements.reused, 2);
        assert!(Arc::ptr_eq(
            &unchanged.snapshot.documents()[0],
            &second.snapshot.documents()[0]
        ));
        std::fs::write(source.path().join("a.rs"), "fn c() {}").unwrap();
        let edited = engine
            .refresh(Some(&second.snapshot), &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(edited.measurements.reused, 1);
        assert_eq!(edited.measurements.parsed, 1);
        assert_ne!(edited.snapshot.id(), second.snapshot.id());
        assert_eq!(second.snapshot.documents()[0].source(), Some("fn a() {}"));
        assert_eq!(
            edited.snapshot.documents()[0].analyses()[0].facts[0].name,
            "c"
        );
        std::fs::remove_file(source.path().join("a.rs")).unwrap();
        assert_eq!(
            engine
                .refresh(Some(&edited.snapshot), &|| false)
                .unwrap()
                .unwrap()
                .snapshot
                .documents()
                .len(),
            1
        );
        assert!(engine.refresh(None, &|| true).unwrap().is_none());
    }
    #[test]
    fn unusable_cache_is_nonfatal_and_worker_counts_do_not_change_results() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path().join("a.rs"), "fn a() {}").unwrap();
        let first = engine(source.path(), cache.path(), 1)
            .refresh(None, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(first.measurements.cache_warnings, 1);
        let second = engine(source.path(), cache.path(), 4)
            .refresh(None, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(first.snapshot.id(), second.snapshot.id());
    }
    #[test]
    fn measurement_units_preserve_submillisecond_precision() {
        assert_eq!(milliseconds(Duration::from_micros(1500)), 1.5);
        assert_eq!(milliseconds(Duration::from_secs(2)), 2000.0);
    }

    #[test]
    fn hinted_reads_detect_same_stamp_edits_and_reconciliation_recovers_silent_changes() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let a = source.path().join("a.rs");
        let b = source.path().join("b.rs");
        std::fs::write(&a, "fn a() {}").unwrap();
        std::fs::write(&b, "fn b() {}").unwrap();
        let engine = engine(source.path(), cache.path(), 2);
        let initial = engine.refresh(None, &|| false).unwrap().unwrap();
        let stamp = std::fs::metadata(&a).unwrap().modified().unwrap();
        std::fs::write(&a, "fn c() {}").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&a)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(stamp))
            .unwrap();
        std::fs::write(&b, "fn d() {}").unwrap();
        let hint = CaptureScope::Paths(["a.rs".into()].into());
        let incremental = engine
            .refresh_scoped(Some(&initial.snapshot), &hint, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(incremental.measurements.read_files, 1);
        assert_eq!(incremental.measurements.bytes, 9);
        assert_eq!(incremental.measurements.files, 2);
        assert_eq!(incremental.measurements.retained, 1);
        assert_eq!(incremental.measurements.reused, 1);
        assert_eq!(incremental.measurements.parsed, 1);
        assert!(!incremental.measurements.reconciled);
        assert!(incremental.directories.is_none());
        assert_eq!(
            incremental.snapshot.documents()[0].analyses()[0].facts[0].name,
            "c"
        );
        assert_eq!(
            incremental.snapshot.documents()[1].source(),
            Some("fn b() {}")
        );
        assert!(Arc::ptr_eq(
            &initial.snapshot.documents()[1],
            &incremental.snapshot.documents()[1]
        ));
        let complete = engine
            .refresh(Some(&incremental.snapshot), &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(complete.measurements.read_files, 2);
        assert!(complete.measurements.reconciled);
        assert_eq!(complete.snapshot.documents()[1].source(), Some("fn d() {}"));
        let both = engine
            .refresh_scoped(
                Some(&initial.snapshot),
                &CaptureScope::Paths(["a.rs".into(), "b.rs".into()].into()),
                &|| false,
            )
            .unwrap()
            .unwrap();
        assert_eq!(both.snapshot.id(), complete.snapshot.id());
    }

    #[test]
    fn hinted_renames_and_deletions_match_clean_capture() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("old.rs"), "fn a() {}").unwrap();
        let engine = engine(source.path(), cache.path(), 1);
        let initial = engine.refresh(None, &|| false).unwrap().unwrap();
        std::fs::rename(source.path().join("old.rs"), source.path().join("new.rs")).unwrap();
        let renamed = engine
            .refresh_scoped(
                Some(&initial.snapshot),
                &CaptureScope::Paths(["old.rs".into(), "new.rs".into()].into()),
                &|| false,
            )
            .unwrap()
            .unwrap();
        assert_eq!(renamed.measurements.read_files, 1);
        assert_eq!(renamed.snapshot.documents()[0].path(), "new.rs");
        assert_eq!(
            renamed.snapshot.id(),
            engine
                .refresh(None, &|| false)
                .unwrap()
                .unwrap()
                .snapshot
                .id()
        );
        std::fs::remove_file(source.path().join("new.rs")).unwrap();
        let removed = engine
            .refresh_scoped(
                Some(&renamed.snapshot),
                &CaptureScope::Paths(["new.rs".into()].into()),
                &|| false,
            )
            .unwrap()
            .unwrap();
        assert_eq!(removed.measurements.read_files, 0);
        assert!(removed.snapshot.documents().is_empty());
        assert_eq!(
            removed.snapshot.id(),
            engine
                .refresh(None, &|| false)
                .unwrap()
                .unwrap()
                .snapshot
                .id()
        );
    }

    #[test]
    fn new_parent_and_removed_subtree_require_complete_discovery() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let engine = engine(source.path(), cache.path(), 1);
        let initial = engine.refresh(None, &|| false).unwrap().unwrap();
        std::fs::create_dir(source.path().join("folder.rs")).unwrap();
        std::fs::write(source.path().join("folder.rs/a.rs"), "fn a() {}").unwrap();
        let created = engine
            .refresh_scoped(
                Some(&initial.snapshot),
                &CaptureScope::Paths(["folder.rs/a.rs".into()].into()),
                &|| false,
            )
            .unwrap()
            .unwrap();
        assert!(created.measurements.reconciled);
        assert!(created.directories.is_some());
        std::fs::rename(
            source.path().join("folder.rs"),
            source.path().join("renamed"),
        )
        .unwrap();
        let moved = engine
            .refresh_scoped(
                Some(&created.snapshot),
                &CaptureScope::Paths(["folder.rs".into()].into()),
                &|| false,
            )
            .unwrap()
            .unwrap();
        assert!(moved.measurements.reconciled);
        assert_eq!(moved.snapshot.documents()[0].path(), "renamed/a.rs");
    }

    #[test]
    fn incompatible_context_and_workspace_cannot_supply_unread_documents() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        for name in ["a", "b"] {
            std::fs::write(
                source.path().join(format!("{name}.rs")),
                format!("fn {name}() {{}}"),
            )
            .unwrap();
        }
        let engine = engine(source.path(), cache.path(), 1);
        let initial = engine.refresh(None, &|| false).unwrap().unwrap();
        let documents = initial
            .snapshot
            .documents()
            .iter()
            .map(|document| {
                let mut analysis = document.analyses()[0].clone();
                analysis.context.description = "old implementation".into();
                Arc::new(
                    Document::capture(
                        initial.snapshot.workspace().clone(),
                        document.path().into(),
                        document.source().unwrap().as_bytes(),
                        vec![analysis],
                        document.classification().clone(),
                    )
                    .unwrap(),
                )
            })
            .collect();
        let previous = Snapshot::new(
            initial.snapshot.workspace().clone(),
            documents,
            initial.snapshot.policy().clone(),
        )
        .unwrap();
        let fresh = engine
            .refresh_scoped(
                Some(&previous),
                &CaptureScope::Paths(["a.rs".into()].into()),
                &|| false,
            )
            .unwrap()
            .unwrap();
        assert!(fresh.measurements.reconciled);
        assert_eq!(fresh.measurements.read_files, 2);
        assert_eq!(fresh.measurements.retained, 0);
        assert_eq!(fresh.snapshot.id(), initial.snapshot.id());
        let unrelated = Snapshot::new(
            vizzi_core::model::WorkspaceId::derive(&[b"other"]),
            vec![],
            initial.snapshot.policy().clone(),
        )
        .unwrap();
        let fresh = engine
            .refresh_scoped(
                Some(&unrelated),
                &CaptureScope::Paths(["a.rs".into()].into()),
                &|| false,
            )
            .unwrap()
            .unwrap();
        assert!(fresh.measurements.reconciled);
        assert_eq!(fresh.snapshot.id(), initial.snapshot.id());
    }

    #[test]
    fn compatible_nested_hints_stay_partial_and_bad_baselines_and_limits_reconcile() {
        let source = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        std::fs::create_dir(source.path().join("src")).unwrap();
        std::fs::write(source.path().join("a.rs"), "fn first() {}").unwrap();
        std::fs::write(source.path().join("src/z.rs"), "fn nested() {}").unwrap();
        let engine = engine(source.path(), cache.path(), 1);
        let initial = engine.refresh(None, &|| false).unwrap().unwrap();
        let changed = engine
            .refresh_scoped(
                Some(&initial.snapshot),
                &CaptureScope::Paths(["src/z.rs".into()].into()),
                &|| false,
            )
            .unwrap()
            .unwrap();
        assert!(!changed.measurements.reconciled);
        assert_eq!(changed.measurements.read_files, 1);
        for (path, analyses) in [
            ("a.rs", vec![]),
            (
                "not-rust.txt",
                initial.snapshot.documents()[0].analyses().to_vec(),
            ),
        ] {
            let document = Document::capture(
                initial.snapshot.workspace().clone(),
                path.into(),
                b"fn first() {}",
                analyses,
                initial.snapshot.documents()[0].classification().clone(),
            )
            .unwrap();
            let previous = Snapshot::new(
                initial.snapshot.workspace().clone(),
                vec![Arc::new(document)],
                initial.snapshot.policy().clone(),
            )
            .unwrap();
            assert!(
                engine
                    .refresh_scoped(
                        Some(&previous),
                        &CaptureScope::Paths(["src/z.rs".into()].into()),
                        &|| false
                    )
                    .unwrap()
                    .unwrap()
                    .measurements
                    .reconciled
            );
        }
        let limit: BTreeSet<_> = (0..MAX_PATH_HINTS)
            .map(|i| format!("gone{i}.rs").into())
            .collect();
        let at_limit = engine
            .refresh_scoped(
                Some(&initial.snapshot),
                &CaptureScope::Paths(limit.clone()),
                &|| false,
            )
            .unwrap()
            .unwrap();
        assert!(!at_limit.measurements.reconciled);
        assert_eq!(at_limit.measurements.read_files, 0);
        for paths in [
            BTreeSet::new(),
            limit.into_iter().chain(["extra.rs".into()]).collect(),
        ] {
            assert!(
                engine
                    .refresh_scoped(
                        Some(&initial.snapshot),
                        &CaptureScope::Paths(paths),
                        &|| false
                    )
                    .unwrap()
                    .unwrap()
                    .measurements
                    .reconciled
            );
        }
    }
}
