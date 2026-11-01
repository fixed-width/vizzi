//! File-level parsing with a bounded native worker pool and deterministic results.

use crate::syntax::{SourceLanguage, SyntaxCounts, SyntaxError};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

/// A source identity and its syntax counts, independent of scheduling.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedFile {
    /// Relative source path.
    pub path: PathBuf,
    /// Full source content fingerprint.
    pub hash: blake3::Hash,
    /// Number of source bytes read.
    pub bytes: u64,
    /// Syntax counts for this input.
    pub counts: SyntaxCounts,
}

/// Observed work performed by one parser-owning thread.
#[derive(Debug, Default)]
pub struct WorkerStats {
    /// Number of files processed.
    pub files: usize,
    /// Number of source bytes read.
    pub bytes: u64,
    /// Sum of file-read wall durations in this worker.
    pub reads: Duration,
    /// Sum of parsing wall durations, including any scheduling delays.
    pub parsing: Duration,
    /// Sum of syntax-inspection wall durations.
    pub inspection: Duration,
}

impl WorkerStats {
    /// Accumulate the observed phases of one successfully parsed file.
    pub fn record(&mut self, bytes: u64, reads: Duration, parsing: Duration, inspection: Duration) {
        self.files += 1;
        self.bytes += bytes;
        self.reads += reads;
        self.parsing += parsing;
        self.inspection += inspection;
    }
}

/// Completed files sorted by path and measurements for each worker.
#[derive(Debug, Default)]
pub struct ParsedBatch {
    /// Complete, deterministic file results.
    pub files: Vec<ParsedFile>,
    /// Actual workers, clamped to the number of jobs.
    pub workers: Vec<WorkerStats>,
}

/// An invalid pool configuration or an incomplete parsing run.
#[derive(Debug, thiserror::Error)]
pub enum ParallelError {
    /// Worker counts are bounded independently of corpus size.
    #[error("worker count must be in 1..=128")]
    InvalidWorkers,
    /// Thread creation failed.
    #[error("cannot start parser worker: {0}")]
    Thread(#[source] io::Error),
    /// A worker did not exit normally.
    #[error("parser worker panicked")]
    WorkerPanicked,
    /// A source file could not be read.
    #[error("cannot read {path}: {source}")]
    File {
        /// Source path.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
    /// The selected grammar could not be configured or parsing was interrupted.
    #[error(transparent)]
    Syntax(#[from] SyntaxError),
}

/// Process distinct relative paths with one reusable parser per worker.
///
/// Jobs are claimed dynamically in input order. File I/O, hashing, parsing, inspection,
/// and tree disposal occur inside the pool; no syntax trees are shared between workers.
pub fn parse_files(
    root: &Path,
    language: SourceLanguage,
    paths: &[PathBuf],
    workers: usize,
) -> Result<ParsedBatch, ParallelError> {
    if workers == 0 || workers > 128 {
        return Err(ParallelError::InvalidWorkers);
    }
    let next = AtomicUsize::new(0);
    let mut batch = thread::scope(|scope| {
        let mut handles = Vec::new();
        for worker in 0..workers.min(paths.len()) {
            let next = &next;
            handles.push(
                thread::Builder::new()
                    .name(format!("syntax-{worker}"))
                    .spawn_scoped(scope, move || worker_files(root, language, paths, next))
                    .map_err(ParallelError::Thread)?,
            );
        }
        let mut batch = ParsedBatch {
            files: Vec::with_capacity(paths.len()),
            workers: Vec::new(),
        };
        for handle in handles {
            let (mut files, stats) = handle.join().map_err(|_| ParallelError::WorkerPanicked)??;
            batch.files.append(&mut files);
            batch.workers.push(stats);
        }
        Ok::<_, ParallelError>(batch)
    })?;
    batch.files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(batch)
}

fn worker_files(
    root: &Path,
    language: SourceLanguage,
    paths: &[PathBuf],
    next: &AtomicUsize,
) -> Result<(Vec<ParsedFile>, WorkerStats), ParallelError> {
    let mut parser = language.parser()?;
    let mut files = Vec::new();
    let mut stats = WorkerStats::default();
    loop {
        let index = next.fetch_add(1, Ordering::Relaxed);
        let Some(path) = paths.get(index) else {
            break;
        };
        let start = Instant::now();
        let bytes = std::fs::read(root.join(path)).map_err(|source| ParallelError::File {
            path: path.clone(),
            source,
        })?;
        let reads = start.elapsed();
        let start = Instant::now();
        let tree = parser.parse(&bytes, None).ok_or(SyntaxError::NoTree)?;
        let parsing = start.elapsed();
        let start = Instant::now();
        let counts = crate::syntax::inspect(&tree);
        let inspection = start.elapsed();
        stats.record(bytes.len() as u64, reads, parsing, inspection);
        files.push(ParsedFile {
            path: path.clone(),
            hash: blake3::hash(&bytes),
            bytes: bytes.len() as u64,
            counts,
        });
    }
    Ok((files, stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{source, syntax};
    use std::{collections::BTreeSet, fs};

    #[test]
    fn phase_totals_add_observations_without_assuming_clock_resolution() {
        let mut stats = WorkerStats::default();
        stats.record(
            3,
            Duration::from_millis(2),
            Duration::from_millis(3),
            Duration::from_millis(4),
        );
        stats.record(
            7,
            Duration::from_millis(11),
            Duration::from_millis(13),
            Duration::from_millis(17),
        );
        assert_eq!(
            (
                stats.files,
                stats.bytes,
                stats.reads,
                stats.parsing,
                stats.inspection
            ),
            (
                2,
                10,
                Duration::from_millis(13),
                Duration::from_millis(16),
                Duration::from_millis(21)
            )
        );
    }

    #[test]
    fn workers_produce_the_same_per_file_fingerprints_and_counts_as_sequential_parses() {
        let root = tempfile::tempdir().unwrap();
        for (name, text) in [
            ("z.rs", "fn z() {}"),
            ("a.rs", "pub fn a() { let x = 1; }"),
            ("broken.rs", "@"),
            ("empty.rs", ""),
        ] {
            fs::write(root.path().join(name), text).unwrap();
        }
        let paths = vec![
            "z.rs".into(),
            "broken.rs".into(),
            "empty.rs".into(),
            "a.rs".into(),
        ];
        let expected: Vec<_> = source::files(root.path())
            .unwrap()
            .into_iter()
            .map(|path| {
                let bytes = fs::read(root.path().join(&path)).unwrap();
                let tree = SourceLanguage::Rust
                    .parser()
                    .unwrap()
                    .parse(&bytes, None)
                    .unwrap();
                ParsedFile {
                    path,
                    hash: blake3::hash(&bytes),
                    bytes: bytes.len() as u64,
                    counts: syntax::inspect(&tree),
                }
            })
            .collect();
        for workers in [1, 2, 4, 8, 128] {
            let batch = parse_files(root.path(), SourceLanguage::Rust, &paths, workers).unwrap();
            assert_eq!(batch.files, expected);
            assert_eq!(batch.workers.len(), workers.min(paths.len()));
            assert_eq!(
                batch.workers.iter().map(|w| w.files).sum::<usize>(),
                paths.len()
            );
            assert_eq!(
                batch.workers.iter().map(|w| w.bytes).sum::<u64>(),
                expected.iter().map(|f| f.bytes).sum::<u64>()
            );
            assert_eq!(
                batch
                    .files
                    .iter()
                    .map(|f| &f.path)
                    .collect::<BTreeSet<_>>()
                    .len(),
                paths.len()
            );
        }
    }

    #[test]
    fn missing_files_fail_instead_of_silently_returning_partial_success() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("good.rs"), "fn f() {}").unwrap();
        let result = parse_files(
            root.path(),
            SourceLanguage::Rust,
            &["good.rs".into(), "missing.rs".into()],
            2,
        );
        assert!(
            matches!(result,Err(ParallelError::File {path,..}) if path==Path::new("missing.rs"))
        );
    }

    #[test]
    fn empty_corpora_use_no_workers_but_invalid_limits_still_fail() {
        let root = tempfile::tempdir().unwrap();
        let empty = parse_files(root.path(), SourceLanguage::Rust, &[], 4).unwrap();
        assert!(empty.files.is_empty() && empty.workers.is_empty());
        for workers in [0, 129, usize::MAX] {
            assert!(matches!(
                parse_files(root.path(), SourceLanguage::Rust, &[], workers),
                Err(ParallelError::InvalidWorkers)
            ));
        }
    }
}
