//! Private captured-source backing with owned extents and deferred retirement.

mod extents;
mod pool;
use pool::{Allocation, Cut, Pool};

use serde::Serialize;
#[cfg(feature = "qualification")]
use std::time::Instant;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};
use vizzi_core::model::{CapturedSource, SourceReadError, SourceReader};
/// Qualification-only entry/readiness marks; default builds have no observer state.
#[cfg(feature = "qualification")]
pub(crate) type CaptureTrace = Mutex<Vec<(Instant, Instant)>>;
#[cfg(feature = "qualification")]
pub(crate) struct ReclaimSpan {
    pub started: Instant,
    pub finished: Instant,
    pub cut_bytes: u64,
}
const CAPTURE_CHUNK: usize = 64 * 1024;
#[derive(Debug)]
struct State {
    pool: Pool,
    written: usize,
    stores: usize,
    staging_bytes: usize,
    peak_staging_bytes: usize,
    max_read_request: usize,
}
#[derive(Debug)]
struct Backing {
    file: File,
    reads: usize,
    read_bytes: usize,
}
#[derive(Debug)]
struct Files {
    // Both handles share a cursor; this one is metadata/set_len only.
    maintenance: Mutex<File>,
    backing: Mutex<Backing>,
}
impl Files {
    fn new(directory: &Path) -> Result<Self, SourceReadError> {
        let file =
            tempfile::tempfile_in(directory).map_err(|e| SourceReadError::Io(e.to_string()))?;
        let cursor = file
            .try_clone()
            .map_err(|e| SourceReadError::Io(e.to_string()))?;
        Ok(Self {
            maintenance: Mutex::new(file),
            backing: Mutex::new(Backing {
                file: cursor,
                reads: 0,
                read_bytes: 0,
            }),
        })
    }
}
struct Cleaning<'a> {
    inner: &'a Inner,
    cut: Option<Cut>,
}
impl Cleaning<'_> {
    fn finish(mut self, success: bool) {
        if let Some(cut) = self.cut.take() {
            self.inner.lock().pool.finish_clean(cut, success);
        }
    }
}
impl Drop for Cleaning<'_> {
    fn drop(&mut self) {
        if let Some(cut) = self.cut.take() {
            self.inner.lock().pool.finish_clean(cut, false);
        }
    }
}
#[derive(Debug)]
struct Inner {
    state: Mutex<State>,
    files: [Files; 2],
    maintenance: Mutex<()>,
    capacity: usize,
    writer: Mutex<()>,
    stage: Arc<Mutex<File>>,
    retired: Mutex<Vec<Allocation>>,
    #[cfg(feature = "qualification")]
    trace: Mutex<Option<Arc<CaptureTrace>>>,
}
impl Inner {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn backing(&self, bank: usize) -> MutexGuard<'_, Backing> {
        self.files[bank]
            .backing
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn writer(&self) -> MutexGuard<'_, ()> {
        self.writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn retired(&self) -> MutexGuard<'_, Vec<Allocation>> {
        self.retired
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
/// Two private committed files and reusable staging; project paths are never backing.
#[derive(Debug)]
pub struct SourceSpool {
    directory: PathBuf,
    inner: Arc<Inner>,
}
/// Backing allocation and cumulative IO observations, separate from resident memory.
#[derive(Clone, Debug, Serialize)]
pub struct SpoolStats {
    /// Maximum aggregate charged span of both files; staging has a separate equal bound.
    pub capacity_bytes: usize,
    /// Cumulative committed bytes; may exceed capacity as ranges are reused.
    pub written_bytes: usize,
    /// Charged bytes, including unfinished writes and queued retirements.
    pub allocated_bytes: usize,
    /// Aggregate charged spans, including gaps and tails awaiting physical cleanup.
    pub extent_bytes: usize,
    /// Charged but unallocated bytes; cleaning-file space is temporarily unavailable.
    pub free_bytes: usize,
    /// Largest reserved address span observed.
    pub peak_extent_bytes: usize,
    /// Total actual length of both files, absent if metadata IO fails.
    pub file_bytes: Option<u64>,
    /// Descriptor retirements awaiting maintenance.
    pub pending_retirements: usize,
    /// Backing files currently unavailable for allocation during physical cleanup.
    pub cleaning_files: usize,
    /// Full charges retained by those cleaning files until IO succeeds.
    pub cleaning_bytes: usize,
    /// Captures committed to backing.
    pub stores: usize,
    /// Backing range calls.
    pub reads: usize,
    /// Bytes returned by backing range calls.
    pub read_bytes: usize,
    /// Current reusable staging-file length, separate from committed quota.
    pub staging_bytes: usize,
    /// Largest observed staging-file length.
    pub peak_staging_bytes: usize,
    /// Largest input read request made by streaming capture.
    pub max_read_request: usize,
}
impl SourceSpool {
    /// Creates private temporary backing in an existing or created external directory.
    pub fn new(directory: &Path, capacity: usize) -> Result<Self, SourceReadError> {
        if capacity == 0 {
            return Err(SourceReadError::Capacity);
        }
        std::fs::create_dir_all(directory)
            .map_err(|error| SourceReadError::Io(error.to_string()))?;
        let directory = directory
            .canonicalize()
            .map_err(|error| SourceReadError::Io(error.to_string()))?;
        let files = [Files::new(&directory)?, Files::new(&directory)?];
        let stage = tempfile::tempfile_in(&directory)
            .map_err(|error| SourceReadError::Io(error.to_string()))?;
        Ok(Self {
            directory,
            inner: Arc::new(Inner {
                state: Mutex::new(State {
                    pool: Pool::default(),
                    written: 0,
                    stores: 0,
                    staging_bytes: 0,
                    peak_staging_bytes: 0,
                    max_read_request: 0,
                }),
                files,
                maintenance: Mutex::new(()),
                capacity,
                writer: Mutex::new(()),
                stage: Arc::new(Mutex::new(stage)),
                retired: Mutex::new(Vec::new()),
                #[cfg(feature = "qualification")]
                trace: Mutex::new(None),
            }),
        })
    }
    /// Directory used only to create anonymous backing, never exposed as source paths.
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    /// Persists a resident capture and moves its metadata onto private immutable backing.
    /// Retirements update allocation indexes without performing physical cleanup.
    pub fn persist(
        &self,
        source: Arc<CapturedSource>,
    ) -> Result<Arc<CapturedSource>, SourceReadError> {
        let Some(bytes) = source.resident_bytes() else {
            return Ok(source);
        };
        let len = bytes.len();
        let _writer = self.inner.writer();
        let backing = self
            .append(
                len,
                |offset, target| {
                    target.copy_from_slice(&bytes[offset..offset + target.len()]);
                    Ok(())
                },
                &|| false,
            )?
            .ok_or(SourceReadError::Integrity)?;
        Ok(source.with_backing(backing))
    }
    /// Stages actual input, reuses verified previous content or builds and commits new metadata.
    /// Input/staging/copy requests are bounded; metadata still scales with input lines/blocks.
    pub fn capture(
        &self,
        input: &mut impl Read,
        previous: Option<&Arc<CapturedSource>>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Arc<CapturedSource>>, SourceReadError> {
        #[cfg(feature = "qualification")]
        let entered = Instant::now();
        #[cfg(feature = "qualification")]
        let trace = self
            .inner
            .trace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let _writer = self.inner.writer();
        let _reset = StageReset { inner: &self.inner };
        self.drain_retired();
        #[cfg(feature = "qualification")]
        if let Some(trace) = trace {
            let ready = Instant::now();
            let mut marks = trace
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if marks.len() < 64 {
                marks.push((entered, ready));
            }
        }
        self.inner
            .stage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .set_len(0)
            .map_err(|error| SourceReadError::Io(error.to_string()))?;
        let mut verification = previous.map(|old| (old, old.verifier()));
        let len = {
            let mut len = 0usize;
            let mut buffer = vec![0u8; CAPTURE_CHUNK];
            loop {
                if cancelled() {
                    return Ok(None);
                }
                self.inner.lock().max_read_request = CAPTURE_CHUNK;
                let count = match input.read(&mut buffer) {
                    Ok(count) => count,
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(SourceReadError::Io(error.to_string())),
                };
                if cancelled() {
                    return Ok(None);
                }
                if count == 0 {
                    break;
                }
                if count > self.inner.capacity - len {
                    return Err(SourceReadError::Capacity);
                }
                if let Some((_, verifier)) = &mut verification {
                    verifier.update(&buffer[..count]);
                }
                {
                    let mut stage = self
                        .inner
                        .stage
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    stage
                        .seek(SeekFrom::Start(len as u64))
                        .map_err(|error| SourceReadError::Io(error.to_string()))?;
                    stage
                        .write_all(&buffer[..count])
                        .map_err(|error| SourceReadError::Io(error.to_string()))?;
                }
                len += count;
                let mut state = self.inner.lock();
                state.staging_bytes = len;
                state.peak_staging_bytes = state.peak_staging_bytes.max(len);
            }
            len
        };
        if let Some((old, verifier)) = verification
            && verifier.matches()
        {
            return Ok((!cancelled()).then(|| old.clone()));
        }
        let staged = Arc::new(Staged {
            file: self.inner.stage.clone(),
            len,
        });
        let Some(source) = CapturedSource::capture_reader(len, staged.clone(), cancelled)? else {
            return Ok(None);
        };
        let Some(backing) = self.append(
            len,
            |offset, target| staged.read_at(offset, target),
            cancelled,
        )?
        else {
            return Ok(None);
        };
        Ok(Some(source.with_backing(backing)))
    }
    fn append(
        &self,
        len: usize,
        read: impl FnMut(usize, &mut [u8]) -> Result<(), SourceReadError>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Arc<Blob>>, SourceReadError> {
        self.drain_retired();
        let extent = self
            .inner
            .lock()
            .pool
            .reserve(len, self.inner.capacity)
            .ok_or(SourceReadError::Capacity)?;
        let start = extent.extent.start;
        let bank = extent.bank;
        let backing = Arc::new(Blob {
            inner: self.inner.clone(),
            extent: Some(extent),
        });
        match self.write_range(bank, start, len, read, cancelled) {
            Ok(true) => {
                let mut state = self.inner.lock();
                state.written = state.written.saturating_add(len);
                state.stores += 1;
                Ok(Some(backing))
            }
            result => {
                drop(backing);
                self.drain_retired();
                result?;
                Ok(None)
            }
        }
    }
    fn write_range(
        &self,
        bank: usize,
        start: usize,
        len: usize,
        mut read: impl FnMut(usize, &mut [u8]) -> Result<(), SourceReadError>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<bool, SourceReadError> {
        let mut buffer = vec![0u8; len.min(CAPTURE_CHUNK)];
        let mut at = 0;
        while at < len {
            if cancelled() {
                return Ok(false);
            }
            let count = (len - at).min(CAPTURE_CHUNK);
            read(at, &mut buffer[..count])?;
            if cancelled() {
                return Ok(false);
            }
            {
                let mut backing = self.inner.backing(bank);
                backing
                    .file
                    .seek(SeekFrom::Start((start + at) as u64))
                    .map_err(|error| SourceReadError::Io(error.to_string()))?;
                backing
                    .file
                    .write_all(&buffer[..count])
                    .map_err(|error| SourceReadError::Io(error.to_string()))?;
            }
            at += count;
        }
        Ok(!cancelled())
    }
    /// Cleans at most both files once, without holding allocation state during physical IO.
    /// Failure retains the old disk charge for retry; captures can use the other file.
    pub fn reclaim(&self) -> Result<(), SourceReadError> {
        self.reclaim_with(File::set_len)
    }
    #[cfg(feature = "qualification")]
    pub(crate) fn observe_reclaim(
        &self,
        on_cut: impl FnOnce(u64),
    ) -> Result<Option<ReclaimSpan>, SourceReadError> {
        self.observe_reclaim_minimum(0, on_cut)
    }
    #[cfg(feature = "qualification")]
    pub(crate) fn observe_reclaim_minimum(
        &self,
        minimum: u64,
        on_cut: impl FnOnce(u64),
    ) -> Result<Option<ReclaimSpan>, SourceReadError> {
        let mut span = None;
        let mut on_cut = Some(on_cut);
        self.reclaim_with(|file, end| {
            if span.is_some() || file.metadata()?.len().saturating_sub(end) < minimum {
                return file.set_len(end);
            }
            let started = Instant::now();
            if let Some(on_cut) = on_cut.take() {
                on_cut(end);
            }
            let result = file.set_len(end);
            span = Some(ReclaimSpan {
                started,
                finished: Instant::now(),
                cut_bytes: end,
            });
            result
        })?;
        Ok(span)
    }
    #[cfg(feature = "qualification")]
    pub(crate) fn trace_capture(&self, trace: Option<Arc<CaptureTrace>>) {
        *self
            .inner
            .trace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = trace;
    }
    fn drain_retired(&self) {
        let retired = std::mem::take(&mut *self.inner.retired());
        let mut state = self.inner.lock();
        for allocation in retired {
            state.pool.retire(allocation);
        }
    }
    fn reclaim_with(
        &self,
        mut truncate: impl FnMut(&File, u64) -> std::io::Result<()>,
    ) -> Result<(), SourceReadError> {
        let _maintenance = self
            .inner
            .maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.drain_retired();
        let active = self.inner.lock().pool.active;
        for bank in [active, 1 - active] {
            let Some(cut) = self.inner.lock().pool.begin_clean(bank) else {
                continue;
            };
            let end = cut.end as u64;
            let cleaning = Cleaning {
                inner: &self.inner,
                cut: Some(cut),
            };
            let result = (|| {
                let file = self.inner.files[bank]
                    .maintenance
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if file.metadata()?.len() > end {
                    truncate(&file, end)?;
                }
                Ok::<(), std::io::Error>(())
            })();
            cleaning.finish(result.is_ok());
            result.map_err(|error| SourceReadError::Io(error.to_string()))?;
        }
        Ok(())
    }
    /// Observes aggregate charges and IO; concurrent values are diagnostic snapshots.
    pub fn stats(&self) -> SpoolStats {
        let pending_retirements = self.inner.retired().len();
        let state = self.inner.lock();
        let charged = state.pool.charged();
        let allocated = state.pool.allocated();
        let mut stats = SpoolStats {
            capacity_bytes: self.inner.capacity,
            written_bytes: state.written,
            allocated_bytes: allocated,
            extent_bytes: charged,
            free_bytes: charged - allocated,
            peak_extent_bytes: state.pool.peak,
            file_bytes: None,
            pending_retirements,
            stores: state.stores,
            reads: 0,
            read_bytes: 0,
            staging_bytes: state.staging_bytes,
            peak_staging_bytes: state.peak_staging_bytes,
            max_read_request: state.max_read_request,
            cleaning_files: state.pool.banks.iter().filter(|b| b.cleaning).count(),
            cleaning_bytes: state
                .pool
                .banks
                .iter()
                .filter(|b| b.cleaning)
                .map(|b| b.charged)
                .sum(),
        };
        drop(state);
        stats.file_bytes = self.inner.files.iter().try_fold(0u64, |total, files| {
            let file = files
                .maintenance
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            total.checked_add(file.metadata().ok()?.len())
        });
        for bank in 0..2 {
            let backing = self.inner.backing(bank);
            stats.reads = stats.reads.saturating_add(backing.reads);
            stats.read_bytes = stats.read_bytes.saturating_add(backing.read_bytes);
        }
        stats
    }
}
impl crate::maintenance::SourceMaintenance for SourceSpool {
    fn pending(&self) -> bool {
        if !self.inner.retired().is_empty() {
            return true;
        }
        self.inner
            .lock()
            .pool
            .banks
            .iter()
            .any(|bank| !bank.cleaning && bank.charged > bank.extents.end)
    }
    fn run(&self) -> Result<(), String> {
        self.reclaim().map_err(|error| error.to_string())
    }
}
struct StageReset<'a> {
    inner: &'a Inner,
}
impl Drop for StageReset<'_> {
    fn drop(&mut self) {
        let stage = self
            .inner
            .stage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = stage.set_len(0);
        let remaining = stage
            .metadata()
            .map(|meta| meta.len() as usize)
            .unwrap_or(0);
        drop(stage);
        self.inner.lock().staging_bytes = remaining;
    }
}
#[derive(Debug)]
struct Staged {
    file: Arc<Mutex<File>>,
    len: usize,
}
impl SourceReader for Staged {
    fn read_at(&self, offset: usize, target: &mut [u8]) -> Result<(), SourceReadError> {
        if offset
            .checked_add(target.len())
            .is_none_or(|end| end > self.len)
        {
            return Err(SourceReadError::InvalidRange);
        }
        let mut file = self
            .file
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        file.seek(SeekFrom::Start(offset as u64))
            .map_err(|error| SourceReadError::Io(error.to_string()))?;
        file.read_exact(target).map_err(|error| {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                SourceReadError::ShortRead
            } else {
                SourceReadError::Io(error.to_string())
            }
        })
    }
}
#[derive(Debug)]
struct Blob {
    inner: Arc<Inner>,
    extent: Option<Allocation>,
}
impl Drop for Blob {
    fn drop(&mut self) {
        if let Some(extent) = self.extent.take()
            && extent.extent.len != 0
        {
            self.inner.retired().push(extent);
        }
    }
}
impl SourceReader for Blob {
    fn read_at(&self, offset: usize, target: &mut [u8]) -> Result<(), SourceReadError> {
        let extent = self.extent.as_ref().ok_or(SourceReadError::Integrity)?;
        let bank = extent.bank;
        let extent = &extent.extent;
        let end = offset
            .checked_add(target.len())
            .ok_or(SourceReadError::InvalidRange)?;
        if end > extent.len {
            return Err(SourceReadError::InvalidRange);
        }
        let at = extent
            .start
            .checked_add(offset)
            .ok_or(SourceReadError::InvalidRange)?;
        let mut backing = self.inner.backing(bank);
        backing
            .file
            .seek(SeekFrom::Start(at as u64))
            .map_err(|error| SourceReadError::Io(error.to_string()))?;
        backing.file.read_exact(target).map_err(|error| {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                SourceReadError::ShortRead
            } else {
                SourceReadError::Io(error.to_string())
            }
        })?;
        backing.reads += 1;
        backing.read_bytes += target.len();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maintenance_hint_is_metadata_only_and_clears_after_cleanup() {
        use crate::maintenance::SourceMaintenance;
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 8).unwrap();
        assert!(!spool.pending());
        let source = spool
            .persist(CapturedSource::capture(b"original".to_vec(), None))
            .unwrap();
        assert!(!spool.pending());
        drop(source);
        assert!(spool.pending());
        spool.drain_retired();
        assert!(spool.pending());
        spool.reclaim().unwrap();
        assert!(!spool.pending());
    }
    #[cfg(feature = "qualification")]
    #[test]
    fn observed_cleanup_filters_small_cuts_without_skipping_physical_work() {
        for (minimum, expected) in [(8, true), (9, false)] {
            let root = tempfile::tempdir().unwrap();
            let spool = SourceSpool::new(root.path(), 16).unwrap();
            let pin = spool
                .persist(CapturedSource::capture(b"original".to_vec(), None))
                .unwrap();
            let tail = spool
                .persist(CapturedSource::capture(b"old tail".to_vec(), None))
                .unwrap();
            drop(tail);
            let mut observed = false;
            let span = spool
                .observe_reclaim_minimum(minimum, |_| observed = true)
                .unwrap();
            assert_eq!(observed, expected);
            assert_eq!(span.is_some(), expected);
            assert_eq!(spool.stats().file_bytes, Some(8));
            assert_eq!(pin.read_text(8).unwrap().as_deref(), Some("original"));
        }
    }
    #[test]
    fn capture_finishes_before_a_different_backing_tail_is_unpaused() {
        use std::{sync::mpsc, time::Duration};
        let root = tempfile::tempdir().unwrap();
        let spool = Arc::new(SourceSpool::new(root.path(), 24).unwrap());
        let original = spool
            .persist(CapturedSource::capture(b"original".to_vec(), None))
            .unwrap();
        let tail = spool
            .persist(CapturedSource::capture(b"old tail".to_vec(), None))
            .unwrap();
        drop(tail);
        let (cut, at_cut) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        let cleaning = spool.clone();
        let worker = std::thread::spawn(move || {
            cleaning.reclaim_with(|file, end| {
                cut.send(end).unwrap();
                resume.recv_timeout(Duration::from_secs(5)).unwrap();
                file.set_len(end)
            })
        });
        assert_eq!(at_cut.recv_timeout(Duration::from_secs(2)).unwrap(), 8);
        let capture = spool.clone();
        let (done, completed) = mpsc::channel();
        let writer = std::thread::spawn(move || {
            let result = capture.capture(&mut std::io::Cursor::new(b"new tail"), None, &|| false);
            done.send(()).unwrap();
            result
        });
        let progressed = completed.recv_timeout(Duration::from_secs(2));
        release.send(()).unwrap();
        worker.join().unwrap().unwrap();
        let next = writer.join().unwrap().unwrap().unwrap();
        assert!(progressed.is_ok());
        assert_eq!(original.read_text(8).unwrap().as_deref(), Some("original"));
        assert_eq!(next.read_text(8).unwrap().as_deref(), Some("new tail"));
        assert_eq!(spool.stats().file_bytes, Some(16));
        assert_eq!(spool.stats().extent_bytes, 16);
        drop(original);
        drop(next);
        spool.reclaim().unwrap();
        assert_eq!(spool.stats().extent_bytes, 0);
        assert_eq!(spool.stats().file_bytes, Some(0));
    }
    #[cfg(feature = "qualification")]
    #[test]
    fn capture_trace_is_bounded_and_can_be_disabled() {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 1).unwrap();
        let trace = Arc::new(CaptureTrace::default());
        spool.trace_capture(Some(trace.clone()));
        for _ in 0..70 {
            spool
                .capture(&mut std::io::empty(), None, &|| false)
                .unwrap();
        }
        assert_eq!(trace.lock().unwrap().len(), 64);
        spool.trace_capture(None);
        trace.lock().unwrap().clear();
        spool
            .capture(&mut std::io::empty(), None, &|| false)
            .unwrap();
        assert!(trace.lock().unwrap().is_empty());
    }
    #[cfg(feature = "qualification")]
    #[test]
    fn capture_trace_finishes_while_another_file_has_a_paused_cut() {
        use std::{sync::mpsc, time::Duration};
        let root = tempfile::tempdir().unwrap();
        let spool = Arc::new(SourceSpool::new(root.path(), 24).unwrap());
        let old = spool
            .persist(CapturedSource::capture(b"original".to_vec(), None))
            .unwrap();
        let tail = spool
            .persist(CapturedSource::capture(b"old tail".to_vec(), None))
            .unwrap();
        drop(tail);
        let (cut, entered) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        let maintenance_spool = spool.clone();
        let maintenance = std::thread::spawn(move || {
            maintenance_spool.observe_reclaim(|end| {
                cut.send(end).unwrap();
                resume.recv_timeout(Duration::from_secs(5)).unwrap();
            })
        });
        assert_eq!(entered.recv_timeout(Duration::from_secs(2)).unwrap(), 8);
        let trace = Arc::new(CaptureTrace::default());
        spool.trace_capture(Some(trace.clone()));
        let capturing = spool.clone();
        let (done, completed) = mpsc::channel();
        let writer = std::thread::spawn(move || {
            let result = capturing.capture(&mut std::io::Cursor::new(b"new tail"), None, &|| false);
            done.send(()).unwrap();
            result
        });
        let progressed = completed.recv_timeout(Duration::from_secs(2));
        let released = Instant::now();
        release.send(()).unwrap();
        maintenance.join().unwrap().unwrap();
        let next = writer.join().unwrap().unwrap().unwrap();
        assert!(progressed.is_ok());
        let marks = trace.lock().unwrap();
        assert_eq!(marks.len(), 1);
        assert!(marks[0].0 < released && marks[0].1 < released);
        assert_eq!(old.read_text(8).unwrap().as_deref(), Some("original"));
        assert_eq!(next.read_text(8).unwrap().as_deref(), Some("new tail"));
        drop(marks);
        spool.trace_capture(None);
    }
    #[test]
    fn pinned_reads_continue_while_allocation_state_is_locked() {
        use std::{sync::mpsc, time::Duration};
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 8).unwrap();
        let source = spool
            .persist(CapturedSource::capture(b"original".to_vec(), None))
            .unwrap();
        let guard = spool.inner.lock();
        let (sent, received) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            sent.send(source.read_text(8).unwrap().unwrap().into_owned())
                .unwrap()
        });
        let read = received.recv_timeout(Duration::from_secs(2));
        drop(guard);
        reader.join().unwrap();
        assert_eq!(read.unwrap(), "original");
    }
    #[test]
    fn reads_and_captures_progress_during_successful_or_failed_cleanup() {
        use std::{sync::mpsc, time::Duration};
        for fail in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let spool = Arc::new(SourceSpool::new(root.path(), 24).unwrap());
            let source = spool
                .persist(CapturedSource::capture(b"original".to_vec(), None))
                .unwrap();
            let tail = spool
                .persist(CapturedSource::capture(b"old tail".to_vec(), None))
                .unwrap();
            drop(tail);
            let (entered, at_cut) = mpsc::channel();
            let (release, resume) = mpsc::channel();
            let maintenance_spool = spool.clone();
            let maintenance = std::thread::spawn(move || {
                maintenance_spool.reclaim_with(|file, end| {
                    entered.send(end).unwrap();
                    resume
                        .recv_timeout(Duration::from_secs(5))
                        .map_err(std::io::Error::other)?;
                    if fail {
                        Err(std::io::Error::other("truncate fixture"))
                    } else {
                        file.set_len(end)
                    }
                })
            });
            assert_eq!(at_cut.recv_timeout(Duration::from_secs(2)).unwrap(), 8);
            let pinned = source.clone();
            let (read_sent, read_received) = mpsc::channel();
            let reader = std::thread::spawn(move || {
                read_sent
                    .send(pinned.read_text(8).map(|s| s.unwrap().into_owned()))
                    .unwrap()
            });
            let read = read_received.recv_timeout(Duration::from_secs(2));

            let writer_spool = spool.clone();
            let (attempted, attempting) = mpsc::channel();
            let (written, finished) = mpsc::channel();
            let writer = std::thread::spawn(move || {
                attempted.send(()).unwrap();
                let result =
                    writer_spool.persist(CapturedSource::capture(b"new tail".to_vec(), None));
                written.send(()).unwrap();
                result
            });
            attempting.recv_timeout(Duration::from_secs(2)).unwrap();
            let premature = finished.recv_timeout(Duration::from_secs(2));
            release.send(()).unwrap();
            let maintained = maintenance.join().unwrap();
            reader.join().unwrap();
            let next = writer.join().unwrap().unwrap();
            assert_eq!(read.unwrap().unwrap(), "original");
            assert!(premature.is_ok());
            assert_eq!(maintained.is_err(), fail);
            assert_eq!(source.read_text(8).unwrap().as_deref(), Some("original"));
            assert_eq!(next.read_text(8).unwrap().as_deref(), Some("new tail"));
            assert_eq!(spool.stats().file_bytes, Some(if fail { 24 } else { 16 }));
            spool.reclaim().unwrap();
            assert_eq!(spool.stats().file_bytes, Some(16));
        }
    }
    #[test]
    fn immutable_blob_reads_reject_corruption_and_truncation() {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 100).unwrap();
        let source = spool
            .persist(CapturedSource::capture(b"correct bytes".to_vec(), None))
            .unwrap();
        {
            let mut backing = spool.inner.backing(0);
            backing.file.seek(SeekFrom::Start(1)).unwrap();
            backing.file.write_all(b"X").unwrap();
        }
        assert_eq!(
            source.reader().read(0..1, 1).unwrap_err(),
            SourceReadError::Integrity
        );
        spool.inner.files[0]
            .maintenance
            .lock()
            .unwrap()
            .set_len(0)
            .unwrap();
        assert_eq!(
            source.read_all(100).unwrap_err(),
            SourceReadError::ShortRead
        );
    }
    #[test]
    fn cancelled_or_failed_copy_returns_logical_space_before_physical_cleanup() {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 200000).unwrap();
        let old = spool
            .capture(&mut std::io::Cursor::new(b"original"), None, &|| false)
            .unwrap()
            .unwrap();
        let cancelled = || {
            spool.inner.files[0]
                .maintenance
                .lock()
                .unwrap()
                .metadata()
                .unwrap()
                .len()
                > 8
        };
        assert!(
            spool
                .capture(
                    &mut std::io::Cursor::new(vec![b'x'; 100000]),
                    None,
                    &cancelled
                )
                .unwrap()
                .is_none()
        );
        assert_eq!(spool.stats().allocated_bytes, 8);
        assert!(spool.stats().extent_bytes > 8);
        assert_eq!(spool.stats().written_bytes, 8);
        spool.reclaim().unwrap();
        assert_eq!(spool.stats().file_bytes, Some(8));
        let mut calls = 0;
        assert!(
            spool
                .append(
                    100000,
                    |_, target| {
                        calls += 1;
                        if calls == 2 {
                            return Err(SourceReadError::Io("fixture".into()));
                        }
                        target.fill(b'x');
                        Ok(())
                    },
                    &|| false
                )
                .is_err()
        );
        assert_eq!(spool.stats().allocated_bytes, 8);
        assert!(spool.stats().extent_bytes > 8);
        spool.reclaim().unwrap();
        assert_eq!(spool.stats().file_bytes, Some(8));
        assert_eq!(old.read_text(8).unwrap().as_deref(), Some("original"));
    }

    #[test]
    fn short_staging_reads_are_reported_as_short_reads() {
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(b"abc").unwrap();
        let staged = Staged {
            file: Arc::new(Mutex::new(file)),
            len: 10,
        };
        assert_eq!(
            staged.read_at(0, &mut [0u8; 10]),
            Err(SourceReadError::ShortRead)
        );
        assert_eq!(
            staged.read_at(9, &mut [0u8; 2]),
            Err(SourceReadError::InvalidRange)
        );
    }
    #[test]
    fn descriptor_retirement_never_waits_for_the_backing_io_lock() {
        use std::{sync::mpsc, time::Duration};
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 8).unwrap();
        let source = spool
            .persist(CapturedSource::capture(b"original".to_vec(), None))
            .unwrap();
        let guard = spool.inner.backing(0);
        let (sent, received) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            drop(source);
            sent.send(()).unwrap();
        });
        let finished = received.recv_timeout(Duration::from_secs(2));
        drop(guard);
        worker.join().unwrap();
        assert!(finished.is_ok());
        assert_eq!(spool.stats().pending_retirements, 1);
        spool.reclaim().unwrap();
        assert_eq!(spool.stats().file_bytes, Some(0));
    }
    #[test]
    fn maintenance_preserves_a_reserved_range_while_another_tail_retires() {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 200000).unwrap();
        let head = spool
            .persist(CapturedSource::capture(vec![b'a'; 100000], None))
            .unwrap();
        let mut tail = Some(
            spool
                .persist(CapturedSource::capture(vec![b'b'; 100000], None))
                .unwrap(),
        );
        drop(head);
        spool.reclaim().unwrap();
        let backing = spool
            .append(
                100000,
                |_, target| {
                    drop(tail.take());
                    spool.reclaim().unwrap();
                    assert_eq!(spool.stats().allocated_bytes, 100000);
                    target.fill(b'c');
                    Ok(())
                },
                &|| false,
            )
            .unwrap()
            .unwrap();
        let mut bytes = vec![0; 100000];
        backing.read_at(0, &mut bytes).unwrap();
        assert_eq!(bytes, vec![b'c'; 100000]);
        assert_eq!(spool.stats().file_bytes, Some(100000));
        drop(backing);
        spool.reclaim().unwrap();
        assert_eq!(spool.stats().file_bytes, Some(0));
    }
    #[test]
    fn failed_or_cancelled_copy_returns_a_reused_hole_without_damaging_live_neighbors() {
        use std::sync::atomic::{AtomicBool, Ordering};
        for fail in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let spool = SourceSpool::new(root.path(), 300000).unwrap();
            let a = spool
                .persist(CapturedSource::capture(vec![b'a'; 100000], None))
                .unwrap();
            let hole = spool
                .persist(CapturedSource::capture(vec![b'b'; 100000], None))
                .unwrap();
            let c = spool
                .persist(CapturedSource::capture(vec![b'c'; 100000], None))
                .unwrap();
            drop(hole);
            let mut reads = 0;
            let cancelled = AtomicBool::new(false);
            let result = spool.append(
                100000,
                |_, target| {
                    reads += 1;
                    if reads == 2 {
                        if fail {
                            return Err(SourceReadError::Io("copy fixture".into()));
                        }
                        cancelled.store(true, Ordering::SeqCst);
                    }
                    target.fill(b'd');
                    Ok(())
                },
                &|| cancelled.load(Ordering::SeqCst),
            );
            if fail {
                assert!(result.is_err());
            } else {
                assert!(result.unwrap().is_none());
            }
            assert_eq!(spool.stats().allocated_bytes, 200000);
            assert_eq!(spool.stats().free_bytes, 100000);
            assert_eq!(spool.stats().file_bytes, Some(300000));
            assert_eq!(a.read_all(100000).unwrap().bytes(), vec![b'a'; 100000]);
            assert_eq!(c.read_all(100000).unwrap().bytes(), vec![b'c'; 100000]);
            let replacement = spool
                .persist(CapturedSource::capture(vec![b'e'; 100000], None))
                .unwrap();
            assert_eq!(
                replacement.read_all(100000).unwrap().bytes(),
                vec![b'e'; 100000]
            );
        }
    }
    #[test]
    fn failed_tail_truncation_is_reported_and_retried_without_losing_retirement() {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 8).unwrap();
        let source = spool
            .persist(CapturedSource::capture(b"original".to_vec(), None))
            .unwrap();
        drop(source);
        let mut readonly = tempfile::NamedTempFile::new_in(root.path()).unwrap();
        readonly.write_all(b"original").unwrap();
        let original = std::mem::replace(
            &mut *spool.inner.files[0].maintenance.lock().unwrap(),
            File::open(readonly.path()).unwrap(),
        );
        assert!(matches!(spool.reclaim(), Err(SourceReadError::Io(_))));
        assert_eq!(spool.stats().allocated_bytes, 0);
        assert_eq!(spool.stats().extent_bytes, 8);
        assert_eq!(spool.stats().file_bytes, Some(8));
        *spool.inner.files[0].maintenance.lock().unwrap() = original;
        spool.reclaim().unwrap();
        assert_eq!(spool.stats().file_bytes, Some(0));
    }
    #[test]
    fn failed_write_releases_a_reservation_without_truncating_an_unchanged_readonly_prefix() {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 16).unwrap();
        let source = spool
            .persist(CapturedSource::capture(b"original".to_vec(), None))
            .unwrap();
        let mut readonly = tempfile::NamedTempFile::new_in(root.path()).unwrap();
        readonly.write_all(b"original").unwrap();
        let maintenance_file = std::mem::replace(
            &mut *spool.inner.files[0].maintenance.lock().unwrap(),
            File::open(readonly.path()).unwrap(),
        );
        let original = std::mem::replace(
            &mut spool.inner.backing(0).file,
            File::open(readonly.path()).unwrap(),
        );
        assert!(matches!(
            spool.persist(CapturedSource::capture(b"new data".to_vec(), None)),
            Err(SourceReadError::Io(_))
        ));
        spool.reclaim().unwrap();
        assert_eq!(spool.stats().allocated_bytes, 8);
        assert_eq!(source.read_text(8).unwrap().as_deref(), Some("original"));
        spool.inner.backing(0).file = original;
        *spool.inner.files[0].maintenance.lock().unwrap() = maintenance_file;
    }
    #[test]
    fn cleaning_space_is_not_credited_before_io_completes() {
        use std::{sync::mpsc, time::Duration};
        let root = tempfile::tempdir().unwrap();
        let spool = Arc::new(SourceSpool::new(root.path(), 16).unwrap());
        let pin = spool
            .persist(CapturedSource::capture(b"original".to_vec(), None))
            .unwrap();
        let tail = spool
            .persist(CapturedSource::capture(b"old tail".to_vec(), None))
            .unwrap();
        drop(tail);
        let (cut, at_cut) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        let maintenance = spool.clone();
        let cleaner = std::thread::spawn(move || {
            maintenance.reclaim_with(|file, end| {
                cut.send(()).unwrap();
                resume.recv_timeout(Duration::from_secs(5)).unwrap();
                file.set_len(end)
            })
        });
        at_cut.recv_timeout(Duration::from_secs(2)).unwrap();
        let capture = spool.clone();
        let (done, complete) = mpsc::channel();
        let writer = std::thread::spawn(move || {
            let result = capture.persist(CapturedSource::capture(b"new tail".to_vec(), None));
            done.send(()).unwrap();
            result
        });
        let prompt = complete.recv_timeout(Duration::from_secs(2));
        assert_eq!(spool.inner.lock().pool.charged(), 16);
        release.send(()).unwrap();
        cleaner.join().unwrap().unwrap();
        let result = writer.join().unwrap();
        assert!(prompt.is_ok());
        assert!(matches!(result, Err(SourceReadError::Capacity)));
        let next = spool
            .persist(CapturedSource::capture(b"new tail".to_vec(), None))
            .unwrap();
        assert_eq!(next.read_text(8).unwrap().as_deref(), Some("new tail"));
        assert_eq!(pin.read_text(8).unwrap().as_deref(), Some("original"));
        assert_eq!(spool.stats().extent_bytes, 16);
    }
    #[test]
    fn cleanup_unwind_restores_availability_and_keeps_charge_for_retry() {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 24).unwrap();
        let pin = spool
            .persist(CapturedSource::capture(b"original".to_vec(), None))
            .unwrap();
        let tail = spool
            .persist(CapturedSource::capture(b"old tail".to_vec(), None))
            .unwrap();
        drop(tail);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || spool.reclaim_with(|_, _| panic!("cleanup fixture"))
            ))
            .is_err()
        );
        assert_eq!(spool.stats().cleaning_files, 0);
        assert_eq!(spool.stats().extent_bytes, 16);
        let next = spool
            .persist(CapturedSource::capture(b"new tail".to_vec(), None))
            .unwrap();
        spool.reclaim().unwrap();
        assert_eq!(pin.read_text(8).unwrap().as_deref(), Some("original"));
        assert_eq!(next.read_text(8).unwrap().as_deref(), Some("new tail"));
        assert_eq!(spool.stats().file_bytes, Some(16));
    }
}
