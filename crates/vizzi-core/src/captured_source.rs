//! Immutable source metadata with bounded verified reads over resident or backed bytes.

use crate::model::{ContentId, SourceEncoding};
use serde::Serialize;
use std::{borrow::Cow, collections::VecDeque, ops::Range, sync::Arc};

/// Integrity blocks bound backing IO and per-read scratch independently of file size.
pub const SOURCE_CHUNK: usize = 64 * 1024;
/// Explicit bulk inspection limit; bounded windows remain available beyond it.
pub const BULK_SOURCE_LIMIT: usize = 32 * 1024 * 1024;
/// A source read failed without changing the captured encoding or identity.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceReadError {
    /// The requested range is outside captured bytes.
    #[error("invalid captured-source range")]
    InvalidRange,
    /// The operation exceeds its explicit loading/output allowance.
    #[error("captured-source read exceeds capacity")]
    Capacity,
    /// The backing could not return the complete requested bytes.
    #[error("captured-source backing returned a short read")]
    ShortRead,
    /// Returned bytes do not match the captured identity/integrity blocks.
    #[error("captured-source integrity check failed")]
    Integrity,
    /// Native/storage failure, distinct from unsupported text encoding.
    #[error("captured-source IO failed: {0}")]
    Io(String),
}
/// Immutable random-access backing. Implementations must fill the entire supplied slice.
pub trait SourceReader: std::fmt::Debug + Send + Sync {
    /// Reads exactly `target.len()` bytes at the captured-relative offset or returns an error.
    fn read_at(&self, offset: usize, target: &mut [u8]) -> Result<(), SourceReadError>;
}
/// Owned source/index buffer accounting, excluding objects and allocator overhead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SourceStorage {
    /// Logical captured byte length.
    pub source_bytes: usize,
    /// Resident byte capacity; zero for backed source.
    pub byte_capacity: usize,
    /// Stored line-start entries, including the initial zero for unsupported input.
    pub index_entries: usize,
    /// Allocated line-start entry capacity.
    pub index_capacity: usize,
    /// Line-start capacity in bytes, plus compact CRLF terminator metadata.
    pub index_bytes: usize,
    /// Compact CRLF metadata capacity, also included in index bytes.
    pub terminator_bytes: usize,
    /// Integrity-block digest capacity in bytes.
    pub integrity_bytes: usize,
}
#[derive(Debug)]
enum Data {
    Resident(Vec<u8>),
    Backed(Arc<dyn SourceReader>),
}
/// Verified bytes and metadata, independent of path, language, filesystem and source control.
#[derive(Debug)]
pub struct CapturedSource {
    content: ContentId,
    len: usize,
    encoding: SourceEncoding,
    data: Data,
    line_starts: Vec<usize>,
    crlf: Vec<u8>,
    chunks: Vec<[u8; 32]>,
}
/// A verified whole input, borrowing resident bytes or owning one backing load.
#[derive(Debug)]
pub struct SourceInput<'a> {
    content: &'a ContentId,
    data: Cow<'a, [u8]>,
}
impl<'a> SourceInput<'a> {
    /// Exact verified input bytes, including unsupported encoding.
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }
    /// Captured identity verified by this load.
    pub fn content(&self) -> &ContentId {
        self.content
    }
    /// UTF-8 text, absent only for unsupported input.
    pub fn text(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }
    fn into_text(self) -> Option<Cow<'a, str>> {
        match self.data {
            Cow::Borrowed(bytes) => std::str::from_utf8(bytes).ok().map(Cow::Borrowed),
            Cow::Owned(bytes) => String::from_utf8(bytes).ok().map(Cow::Owned),
        }
    }
}
impl CapturedSource {
    /// Hashes actual supplied bytes on every call; equal content may reuse prior storage.
    pub fn capture(bytes: Vec<u8>, previous: Option<&Arc<Self>>) -> Arc<Self> {
        let content = ContentId::derive(&[&bytes]);
        if let Some(previous) = previous.filter(|old| old.content == content) {
            return previous.clone();
        }
        let encoding = if std::str::from_utf8(&bytes).is_ok() {
            SourceEncoding::Utf8
        } else {
            SourceEncoding::Unsupported
        };
        let mut line_starts = vec![0];
        let mut crlf = Vec::new();
        if encoding == SourceEncoding::Utf8 {
            for (index, byte) in bytes.iter().enumerate() {
                if *byte == b'\n' {
                    if index > 0 && bytes[index - 1] == b'\r' {
                        let line = line_starts.len() - 1;
                        crlf.resize((line / 8 + 1).max(crlf.len()), 0);
                        crlf[line / 8] |= 1 << (line % 8);
                    }
                    line_starts.push(index + 1);
                }
            }
        }
        let chunks = bytes
            .chunks(SOURCE_CHUNK)
            .map(|chunk| *blake3::hash(chunk).as_bytes())
            .collect();
        Arc::new(Self {
            content,
            len: bytes.len(),
            encoding,
            data: Data::Resident(bytes),
            line_starts,
            crlf,
            chunks,
        })
    }
    /// Replaces resident bytes with a backing bound to this captured metadata.
    /// Reads verify captured integrity blocks; callers must keep backing immutable.
    pub fn with_backing(self: Arc<Self>, reader: Arc<dyn SourceReader>) -> Arc<Self> {
        let mut source = match Arc::try_unwrap(self) {
            Ok(source) => source,
            Err(source) => Self {
                content: source.content.clone(),
                len: source.len,
                encoding: source.encoding,
                data: Data::Backed(reader.clone()),
                line_starts: source.line_starts.clone(),
                crlf: source.crlf.clone(),
                chunks: source.chunks.clone(),
            },
        };
        source.data = Data::Backed(reader);
        Arc::new(source)
    }
    /// Captured byte identity, independent of backing representation.
    pub fn content(&self) -> &ContentId {
        &self.content
    }
    /// Captured byte length without reading backing.
    pub fn len(&self) -> usize {
        self.len
    }
    /// Whether the captured source is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Resident bytes for diagnostics/fixtures; absence says nothing about encoding.
    pub fn resident_bytes(&self) -> Option<&[u8]> {
        match &self.data {
            Data::Resident(bytes) => Some(bytes),
            Data::Backed(_) => None,
        }
    }
    /// Resident UTF-8 text, without loading backing. Use `read_text` for inspection.
    pub fn resident_text(&self) -> Option<&str> {
        self.resident_bytes()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
    }
    /// Captured encoding without reading backing.
    pub fn encoding(&self) -> SourceEncoding {
        self.encoding
    }
    /// Display line count, including final empty LF line; zero for unsupported text.
    pub fn line_count(&self) -> usize {
        if self.encoding == SourceEncoding::Utf8 {
            self.line_starts.len()
        } else {
            0
        }
    }
    /// A bounded read session with at most two verified integrity blocks of scratch.
    pub fn reader(&self) -> SourceSession<'_> {
        SourceSession {
            source: self,
            cache: VecDeque::new(),
        }
    }
    /// Loads an exact verified whole input within an explicit allowance.
    pub fn read_all(&self, limit: usize) -> Result<SourceInput<'_>, SourceReadError> {
        if self.len > limit {
            return Err(SourceReadError::Capacity);
        }
        let data = self.reader().read(0..self.len, limit)?;
        if matches!(data, Cow::Owned(_)) && ContentId::derive(&[&data]) != self.content {
            return Err(SourceReadError::Integrity);
        }
        Ok(SourceInput {
            content: &self.content,
            data,
        })
    }
    /// Reads UTF-8 source within a bulk allowance; errors never become absent encoding.
    pub fn read_text(&self, limit: usize) -> Result<Option<Cow<'_, str>>, SourceReadError> {
        if self.encoding == SourceEncoding::Unsupported {
            return Ok(None);
        }
        self.read_all(limit)?
            .into_text()
            .map(Some)
            .ok_or(SourceReadError::Integrity)
    }
    /// Observes owned capacities without loading, hashing or mutating source.
    pub fn storage(&self) -> SourceStorage {
        SourceStorage {
            source_bytes: self.len,
            byte_capacity: match &self.data {
                Data::Resident(bytes) => bytes.capacity(),
                Data::Backed(_) => 0,
            },
            index_entries: self.line_starts.len(),
            index_capacity: self.line_starts.capacity(),
            index_bytes: self.line_starts.capacity() * std::mem::size_of::<usize>()
                + self.crlf.capacity(),
            terminator_bytes: self.crlf.capacity(),
            integrity_bytes: self.chunks.capacity() * 32,
        }
    }
    pub(crate) fn line_range(&self, number: usize) -> Option<Range<usize>> {
        if self.encoding != SourceEncoding::Utf8 {
            return None;
        }
        let line = number.checked_sub(1)?;
        let start = *self.line_starts.get(line)?;
        let mut end = self
            .line_starts
            .get(number)
            .map_or(self.len, |offset| offset - 1);
        if self
            .crlf
            .get(line / 8)
            .is_some_and(|bits| bits & (1 << (line % 8)) != 0)
        {
            end -= 1;
        }
        Some(start..end)
    }
    pub(crate) fn lines(&self, span: &Range<usize>) -> Range<usize> {
        self.line_starts
            .partition_point(|offset| *offset <= span.start)
            ..self
                .line_starts
                .partition_point(|offset| *offset <= span.end.saturating_sub(1))
    }
}
/// Bounded reads with a two-block cache; integrity is checked before exposing backing bytes.
pub struct SourceSession<'a> {
    source: &'a CapturedSource,
    cache: VecDeque<(usize, Vec<u8>)>,
}
impl<'a> SourceSession<'a> {
    /// Reads a captured-relative byte range; backing scratch is at most two 64 KiB blocks.
    pub fn read(
        &mut self,
        range: Range<usize>,
        limit: usize,
    ) -> Result<Cow<'a, [u8]>, SourceReadError> {
        if range.start > range.end || range.end > self.source.len {
            return Err(SourceReadError::InvalidRange);
        }
        if range.len() > limit {
            return Err(SourceReadError::Capacity);
        }
        if let Data::Resident(bytes) = &self.source.data {
            return Ok(Cow::Borrowed(&bytes[range]));
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(range.len())
            .map_err(|_| SourceReadError::Capacity)?;
        let mut at = range.start;
        while at < range.end {
            let index = at / SOURCE_CHUNK;
            let within = at % SOURCE_CHUNK;
            let end = range.end.min((index + 1) * SOURCE_CHUNK);
            let chunk = self.chunk(index)?;
            output.extend_from_slice(&chunk[within..within + (end - at)]);
            at = end;
        }
        Ok(Cow::Owned(output))
    }
    fn chunk(&mut self, index: usize) -> Result<&[u8], SourceReadError> {
        if let Some(position) = self.cache.iter().position(|(key, _)| *key == index) {
            return Ok(&self.cache[position].1);
        }
        let start = index
            .checked_mul(SOURCE_CHUNK)
            .ok_or(SourceReadError::InvalidRange)?;
        let len = self
            .source
            .len
            .checked_sub(start)
            .ok_or(SourceReadError::InvalidRange)?
            .min(SOURCE_CHUNK);
        if self.cache.len() == 2 {
            self.cache.pop_front();
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(len)
            .map_err(|_| SourceReadError::Capacity)?;
        bytes.resize(len, 0);
        if let Data::Backed(reader) = &self.source.data {
            reader.read_at(start, &mut bytes)?;
        }
        if self.source.chunks.get(index) != Some(blake3::hash(&bytes).as_bytes()) {
            return Err(SourceReadError::Integrity);
        }
        self.cache.push_back((index, bytes));
        Ok(&self.cache[self.cache.len() - 1].1)
    }
    pub(crate) fn floor(
        &mut self,
        range: &Range<usize>,
        offset: usize,
    ) -> Result<usize, SourceReadError> {
        let mut offset = offset.min(range.len());
        if offset == 0 || offset == range.len() {
            return Ok(offset);
        }
        let start = offset.saturating_sub(3);
        let bytes = self.read(range.start + start..range.start + offset + 1, 4)?;
        // Verified UTF-8 and three-byte lookbehind guarantee a preceding boundary.
        while bytes[offset - start] & 0xc0 == 0x80 {
            offset -= 1;
        }
        Ok(offset)
    }
}
