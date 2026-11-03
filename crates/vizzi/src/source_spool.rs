//! Private append-only captured-source backing, with session-scoped disk capacity.

use serde::Serialize;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};
use vizzi_core::model::{CapturedSource, SourceReadError, SourceReader};
#[derive(Debug)]
struct State {
    file: File,
    written: usize,
    stores: usize,
    reads: usize,
    read_bytes: usize,
}
#[derive(Debug)]
struct Inner {
    state: Mutex<State>,
    capacity: usize,
}
impl Inner {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
/// A private anonymous file shared by captured descriptors; project paths are never backing.
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
    /// Resident captures persisted.
    pub stores: usize,
    /// Backing range calls.
    pub reads: usize,
    /// Bytes returned by backing range calls.
    pub read_bytes: usize,
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
        Ok(Self {
            directory,
            inner: Arc::new(Inner {
                state: Mutex::new(State {
                    file,
                    written: 0,
                    stores: 0,
                    reads: 0,
                    read_bytes: 0,
                }),
                capacity,
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
        let mut state = self.inner.lock();
        if len > self.inner.capacity - state.written {
            return Err(SourceReadError::Capacity);
        }
        let start = state.written;
        state
            .file
            .seek(SeekFrom::Start(start as u64))
            .map_err(|error| SourceReadError::Io(error.to_string()))?;
        state
            .file
            .write_all(bytes)
            .map_err(|error| SourceReadError::Io(error.to_string()))?;
        state.written += len;
        state.stores += 1;
        drop(state);
        Ok(source.with_backing(Arc::new(Blob {
            inner: self.inner.clone(),
            start,
            len,
        })))
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
        }
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
}
