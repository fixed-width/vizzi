//! Private append-only captured-source backing, with session-scoped disk capacity.

use serde::Serialize;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};
use vizzi_core::model::{CapturedSource, SourceReadError, SourceReader};
const CAPTURE_CHUNK: usize = 64 * 1024;
#[derive(Debug)]
struct State {
    file: File,
    written: usize,
    stores: usize,
    reads: usize,
    read_bytes: usize,
    staging_bytes: usize,
    peak_staging_bytes: usize,
    max_read_request: usize,
}
#[derive(Debug)]
struct Inner {
    state: Mutex<State>,
    capacity: usize,
    writer: Mutex<()>,
    stage: Arc<Mutex<File>>,
}
impl Inner {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn writer(&self) -> MutexGuard<'_, ()> {
        self.writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
/// Private committed backing and reusable staging; project paths are never backing.
#[derive(Debug)]
pub struct SourceSpool {
    directory: PathBuf,
    inner: Arc<Inner>,
}
/// Cumulative session disk/IO observations, not live bytes or resident memory.
#[derive(Clone, Debug, Serialize)]
pub struct SpoolStats {
    /// Maximum bytes appendable in this session.
    pub capacity_bytes: usize,
    /// Committed appended bytes, including older/cancelled capture versions.
    pub written_bytes: usize,
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
        let file = tempfile::tempfile_in(&directory)
            .map_err(|error| SourceReadError::Io(error.to_string()))?;
        let stage = tempfile::tempfile_in(&directory)
            .map_err(|error| SourceReadError::Io(error.to_string()))?;
        Ok(Self {
            directory,
            inner: Arc::new(Inner {
                state: Mutex::new(State {
                    file,
                    written: 0,
                    stores: 0,
                    reads: 0,
                    read_bytes: 0,
                    staging_bytes: 0,
                    peak_staging_bytes: 0,
                    max_read_request: 0,
                }),
                capacity,
                writer: Mutex::new(()),
                stage: Arc::new(Mutex::new(stage)),
            }),
        })
    }
    /// Directory used only to create anonymous backing, never exposed as source paths.
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    /// Persists a resident capture and moves its metadata onto private immutable backing.
    /// Already-backed captures are unchanged. This prototype reclaims disk at session end.
    pub fn persist(
        &self,
        source: Arc<CapturedSource>,
    ) -> Result<Arc<CapturedSource>, SourceReadError> {
        let Some(bytes) = source.resident_bytes() else {
            return Ok(source);
        };
        let len = bytes.len();
        let _writer = self.inner.writer();
        let start = self
            .append(
                len,
                |offset, target| {
                    target.copy_from_slice(&bytes[offset..offset + target.len()]);
                    Ok(())
                },
                &|| false,
            )?
            .ok_or(SourceReadError::Integrity)?;
        Ok(source.with_backing(Arc::new(Blob {
            inner: self.inner.clone(),
            start,
            len,
        })))
    }
    /// Streams actual input to private staging, verifies metadata, then reuses or commits it.
    /// Input/staging/copy requests are bounded; metadata still scales with input lines/blocks.
    pub fn capture(
        &self,
        input: &mut impl Read,
        previous: Option<&Arc<CapturedSource>>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Arc<CapturedSource>>, SourceReadError> {
        let _writer = self.inner.writer();
        let _reset = StageReset { inner: &self.inner };
        self.inner
            .stage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .set_len(0)
            .map_err(|error| SourceReadError::Io(error.to_string()))?;
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
        let staged = Arc::new(Staged {
            file: self.inner.stage.clone(),
            len,
        });
        let Some(source) = CapturedSource::capture_reader(len, staged.clone(), cancelled)? else {
            return Ok(None);
        };
        if let Some(old) = previous.filter(|old| old.content() == source.content()) {
            return Ok((!cancelled()).then(|| old.clone()));
        }
        let Some(start) = self.append(
            len,
            |offset, target| staged.read_at(offset, target),
            cancelled,
        )?
        else {
            return Ok(None);
        };
        Ok(Some(source.with_backing(Arc::new(Blob {
            inner: self.inner.clone(),
            start,
            len,
        }))))
    }
    fn append(
        &self,
        len: usize,
        mut read: impl FnMut(usize, &mut [u8]) -> Result<(), SourceReadError>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<usize>, SourceReadError> {
        let start = {
            let state = self.inner.lock();
            if len > self.inner.capacity - state.written {
                return Err(SourceReadError::Capacity);
            }
            state.written
        };
        let mut tail = AppendTail {
            inner: &self.inner,
            start,
            committed: false,
        };
        let mut buffer = vec![0u8; len.min(CAPTURE_CHUNK)];
        let mut at = 0;
        while at < len {
            if cancelled() {
                return Ok(None);
            }
            let count = (len - at).min(CAPTURE_CHUNK);
            read(at, &mut buffer[..count])?;
            if cancelled() {
                return Ok(None);
            }
            {
                let mut state = self.inner.lock();
                state
                    .file
                    .seek(SeekFrom::Start((start + at) as u64))
                    .map_err(|error| SourceReadError::Io(error.to_string()))?;
                state
                    .file
                    .write_all(&buffer[..count])
                    .map_err(|error| SourceReadError::Io(error.to_string()))?;
            }
            at += count;
        }
        if cancelled() {
            return Ok(None);
        }
        let mut state = self.inner.lock();
        state.written += len;
        state.stores += 1;
        tail.committed = true;
        Ok(Some(start))
    }
    /// Reads session accounting without loading captured source.
    pub fn stats(&self) -> SpoolStats {
        let state = self.inner.lock();
        SpoolStats {
            capacity_bytes: self.inner.capacity,
            written_bytes: state.written,
            stores: state.stores,
            reads: state.reads,
            read_bytes: state.read_bytes,
            staging_bytes: state.staging_bytes,
            peak_staging_bytes: state.peak_staging_bytes,
            max_read_request: state.max_read_request,
        }
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
struct AppendTail<'a> {
    inner: &'a Inner,
    start: usize,
    committed: bool,
}
impl Drop for AppendTail<'_> {
    fn drop(&mut self) {
        if !self.committed {
            let state = self.inner.lock();
            let _ = state.file.set_len(self.start as u64);
        }
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
    start: usize,
    len: usize,
}
impl SourceReader for Blob {
    fn read_at(&self, offset: usize, target: &mut [u8]) -> Result<(), SourceReadError> {
        let end = offset
            .checked_add(target.len())
            .ok_or(SourceReadError::InvalidRange)?;
        if end > self.len {
            return Err(SourceReadError::InvalidRange);
        }
        let at = self
            .start
            .checked_add(offset)
            .ok_or(SourceReadError::InvalidRange)?;
        let mut state = self.inner.lock();
        state
            .file
            .seek(SeekFrom::Start(at as u64))
            .map_err(|error| SourceReadError::Io(error.to_string()))?;
        state.file.read_exact(target).map_err(|error| {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                SourceReadError::ShortRead
            } else {
                SourceReadError::Io(error.to_string())
            }
        })?;
        state.reads += 1;
        state.read_bytes += target.len();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn immutable_blob_reads_reject_corruption_and_truncation() {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 100).unwrap();
        let source = spool
            .persist(CapturedSource::capture(b"correct bytes".to_vec(), None))
            .unwrap();
        {
            let mut state = spool.inner.lock();
            state.file.seek(SeekFrom::Start(1)).unwrap();
            state.file.write_all(b"X").unwrap();
        }
        assert_eq!(
            source.reader().read(0..1, 1).unwrap_err(),
            SourceReadError::Integrity
        );
        spool.inner.lock().file.set_len(0).unwrap();
        assert_eq!(
            source.read_all(100).unwrap_err(),
            SourceReadError::ShortRead
        );
    }
    #[test]
    fn cancelled_or_failed_append_rolls_back_only_the_uncommitted_tail() {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 200000).unwrap();
        let old = spool
            .capture(&mut std::io::Cursor::new(b"original"), None, &|| false)
            .unwrap()
            .unwrap();
        let cancelled = || {
            let state = spool.inner.lock();
            state.file.metadata().unwrap().len() > state.written as u64
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
        assert_eq!(spool.inner.lock().file.metadata().unwrap().len(), 8);
        assert_eq!(spool.stats().written_bytes, 8);
        assert_eq!(spool.stats().staging_bytes, 0);
        {
            let _writer = spool.inner.writer();
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
        }
        assert_eq!(spool.inner.lock().file.metadata().unwrap().len(), 8);
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
}
