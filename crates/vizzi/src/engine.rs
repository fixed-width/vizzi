//! Bounded parallel analysis with immutable reuse and disposable persistence.

use crate::{cache::Cache, source::Directory};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use vizzi_core::{
    model::{ContentId, Document, ModelError, Snapshot},
    rust::RustAnalyzer,
};

/// Native source/analysis failure; the host retains its previous snapshot.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
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
#[derive(Clone, Debug, Default, Serialize)]
pub struct Measurements {
    /// Captured Rust files.
    pub files: usize,
    /// Source bytes read and verified, including unchanged files.
    pub bytes: usize,
    /// Files requiring syntax parsing.
    pub parsed: usize,
    /// Files reusing immutable in-memory documents.
    pub reused: usize,
    /// Files reusing persisted analysis.
    pub cache_hits: usize,
    /// Nonfatal cache write failures.
    pub cache_warnings: usize,
    /// Full source discovery and read wall time.
    pub capture_ms: f64,
    /// Capture, analysis, cache, and snapshot-construction wall time.
    pub total_ms: f64,
}
/// A validated refresh ready for a publication-ticket check.
pub struct Refresh {
    /// Complete immutable observation.
    pub snapshot: Arc<Snapshot>,
    /// Directories for native observation.
    pub directories: Vec<PathBuf>,
    /// Capture and analysis measurements.
    pub measurements: Measurements,
}
/// The native analysis boundary; no host transport or mutable publication state.
pub struct Engine {
    directory: Directory,
    cache: Cache,
    workers: usize,
}
impl Engine {
    /// Configures 1–32 workers, each with its own parser.
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
        })
    }
    /// Returns the source adapter used by the watcher.
    pub fn directory(&self) -> &Directory {
        &self.directory
    }
    /// Captures source, reuses validated analysis, and checks cancellation between jobs.
    pub fn refresh(
        &self,
        previous: Option<&Snapshot>,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<Option<Refresh>, EngineError> {
        if cancelled() {
            return Ok(None);
        }
        let started = Instant::now();
        let capture = self.directory.capture()?;
        let context = RustAnalyzer::context();
        let mut measurements = Measurements {
            files: capture.files.len(),
            bytes: capture.files.iter().map(|file| file.bytes.len()).sum(),
            capture_ms: milliseconds(started.elapsed()),
            ..Default::default()
        };
        let previous: BTreeMap<_, _> = previous
            .into_iter()
            .flat_map(Snapshot::documents)
            .map(|file| (file.path(), file))
            .collect();
        let mut documents = Vec::with_capacity(capture.files.len());
        let mut jobs = Vec::new();
        for file in capture.files {
            if cancelled() {
                return Ok(None);
            }
            let content = ContentId::derive(&[&file.bytes]);
            if let Some(old) = previous.get(file.path.as_str()).filter(|old| {
                old.content() == &content
                    && old.analyses().len() == 1
                    && old.analyses()[0].context == context
            }) {
                documents.push((*old).clone());
                measurements.reused += 1;
            } else {
                jobs.push((file, content));
            }
        }
        jobs.sort_by_key(|job| std::cmp::Reverse(job.0.bytes.len()));
        let next = AtomicUsize::new(0);
        let batches = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..self.workers.min(jobs.len()))
                .map(|_| {
                    scope.spawn(|| -> Result<_, EngineError> {
                        let mut parser = RustAnalyzer::new()?;
                        let mut batch = Vec::new();
                        let mut counters = (0, 0, 0);
                        loop {
                            if cancelled() {
                                break;
                            }
                            let index = next.fetch_add(1, Ordering::Relaxed);
                            let Some((file, content)) = jobs.get(index) else {
                                break;
                            };
                            let cached =
                                self.cache.load(&context.id, content).and_then(|analysis| {
                                    Document::capture(
                                        self.directory.workspace().clone(),
                                        file.path.clone(),
                                        &file.bytes,
                                        vec![analysis],
                                    )
                                    .ok()
                                });
                            let document = if let Some(document) = cached {
                                counters.1 += 1;
                                document
                            } else {
                                let analysis = parser.analyze(&file.bytes)?;
                                counters.0 += 1;
                                if self.cache.store(&analysis).is_err() {
                                    counters.2 += 1;
                                }
                                Document::capture(
                                    self.directory.workspace().clone(),
                                    file.path.clone(),
                                    &file.bytes,
                                    vec![analysis],
                                )?
                            };
                            batch.push(Arc::new(document));
                        }
                        Ok((batch, counters))
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
        for (batch, (parsed, hits, warnings)) in batches {
            documents.extend(batch);
            measurements.parsed += parsed;
            measurements.cache_hits += hits;
            measurements.cache_warnings += warnings;
        }
        let snapshot = Arc::new(Snapshot::new(
            self.directory.workspace().clone(),
            documents,
        )?);
        measurements.total_ms = milliseconds(started.elapsed());
        Ok(Some(Refresh {
            snapshot,
            directories: capture.directories,
            measurements,
        }))
    }
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
