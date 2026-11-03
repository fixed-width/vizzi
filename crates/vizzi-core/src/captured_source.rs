//! Immutable owned source and line indexing, independent of paths and analysis.

use crate::model::{ContentId, SourceEncoding};
use serde::Serialize;
use std::{ops::Range, sync::Arc};

/// Owned source/index buffer accounting, excluding objects and allocator overhead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SourceStorage {
    /// Logical captured byte length.
    pub source_bytes: usize,
    /// Allocated byte/string capacity, including unused capacity.
    pub byte_capacity: usize,
    /// Stored line-index entries, including the initial zero for unsupported text.
    pub index_entries: usize,
    /// Allocated line-index entry capacity.
    pub index_capacity: usize,
    /// Line-index capacity multiplied by its element size.
    pub index_bytes: usize,
}

#[derive(Debug)]
enum Bytes {
    Text(String),
    Unsupported(Vec<u8>),
}

/// Verified source bytes and their line index, shareable across documents/contexts.
#[derive(Debug)]
pub struct CapturedSource {
    content: ContentId,
    bytes: Bytes,
    line_starts: Vec<usize>,
}
impl CapturedSource {
    /// Observes existing buffer capacities without copying, hashing or changing storage.
    pub fn storage(&self) -> SourceStorage {
        SourceStorage {
            source_bytes: self.bytes().len(),
            byte_capacity: match &self.bytes {
                Bytes::Text(text) => text.capacity(),
                Bytes::Unsupported(bytes) => bytes.capacity(),
            },
            index_entries: self.line_starts.len(),
            index_capacity: self.line_starts.capacity(),
            index_bytes: self.line_starts.capacity() * std::mem::size_of::<usize>(),
        }
    }
    /// Takes ownership of freshly read bytes; matching contents reuse prior storage.
    /// Every call hashes its input. Neither timestamps nor caller metadata prove reuse.
    pub fn capture(bytes: Vec<u8>, previous: Option<&Arc<Self>>) -> Arc<Self> {
        let content = ContentId::derive(&[&bytes]);
        if let Some(previous) = previous.filter(|previous| previous.content == content) {
            return previous.clone();
        }
        let bytes = match String::from_utf8(bytes) {
            Ok(text) => Bytes::Text(text),
            Err(error) => Bytes::Unsupported(error.into_bytes()),
        };
        let mut line_starts = vec![0];
        if let Bytes::Text(text) = &bytes {
            line_starts.extend(
                text.bytes()
                    .enumerate()
                    .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
            );
        }
        Arc::new(Self {
            content,
            bytes,
            line_starts,
        })
    }
    /// Returns the hash of exactly these captured bytes.
    pub fn content(&self) -> &ContentId {
        &self.content
    }
    /// Returns the original bytes, including unsupported encodings.
    pub fn bytes(&self) -> &[u8] {
        match &self.bytes {
            Bytes::Text(text) => text.as_bytes(),
            Bytes::Unsupported(bytes) => bytes,
        }
    }
    /// Returns UTF-8 source when decoding is supported.
    pub fn text(&self) -> Option<&str> {
        match &self.bytes {
            Bytes::Text(text) => Some(text),
            Bytes::Unsupported(_) => None,
        }
    }
    /// Describes text availability without conflating it with syntax coverage.
    pub fn encoding(&self) -> SourceEncoding {
        if self.text().is_some() {
            SourceEncoding::Utf8
        } else {
            SourceEncoding::Unsupported
        }
    }
    /// Counts display lines, including the final empty line after LF.
    pub fn line_count(&self) -> usize {
        if self.text().is_some() {
            self.line_starts.len()
        } else {
            0
        }
    }
    pub(crate) fn line_range(&self, number: usize) -> Option<Range<usize>> {
        let text = self.text()?;
        let start = *self.line_starts.get(number.checked_sub(1)?)?;
        let mut end = self.line_starts.get(number).copied().unwrap_or(text.len());
        if end > start && text.as_bytes()[end - 1] == b'\n' {
            end -= 1;
            if end > start && text.as_bytes()[end - 1] == b'\r' {
                end -= 1;
            }
        }
        Some(start..end)
    }
    pub(crate) fn lines(&self, span: &Range<usize>) -> Range<usize> {
        let start = self
            .line_starts
            .partition_point(|offset| *offset <= span.start);
        let end = self
            .line_starts
            .partition_point(|offset| *offset <= span.end.saturating_sub(1));
        start..end
    }
}
