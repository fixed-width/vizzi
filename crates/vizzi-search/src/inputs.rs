//! Verified captured documents and standalone text share one indexing implementation.
use crate::{Corpus, Error, FILE_BYTES, SourceFile, Update};
use std::sync::Arc;
use vizzi_core::model::{Analysis, Coverage, Document};
pub(super) enum Input {
    Text(SourceFile),
    Captured(Arc<Document>),
}
impl Input {
    pub(super) fn path(&self) -> &str {
        match self {
            Self::Text(f) => &f.path,
            Self::Captured(d) => d.path(),
        }
    }
    pub(super) fn bytes(&self) -> usize {
        match self {
            Self::Text(f) => f.text.len(),
            Self::Captured(d) => d.bytes(),
        }
    }
    pub(super) fn binding(&self) -> String {
        match self {
            Self::Text(_) => "standalone".into(),
            Self::Captured(d) => format!("captured:{}", d.version().as_str()),
        }
    }
    pub(super) fn reusable(&self, content: blake3::Hash) -> bool {
        match self {
            Self::Text(f) => blake3::hash(f.text.as_bytes()) == content,
            Self::Captured(_) => true,
        }
    }
    pub(super) fn text(&self) -> Result<String, Error> {
        match self {
            Self::Text(f) => Ok(f.text.clone()),
            Self::Captured(d) => d
                .captured()
                .read_text(FILE_BYTES)
                .map_err(|e| Error(e.to_string()))?
                .map(|t| t.into_owned())
                .ok_or_else(|| Error("Captured source is not UTF-8".into())),
        }
    }
    pub(super) fn analysis(&self) -> Option<&Analysis> {
        match self {
            Self::Text(_) => None,
            Self::Captured(d) => d
                .analyses()
                .iter()
                .find(|a| matches!(a.coverage, Coverage::Syntax | Coverage::Partial)),
        }
    }
}
impl Corpus {
    /// Reuses immutable document versions and their validated captured analysis.
    /// Source-only/deferred documents are indexed as text without another parser run.
    /// Read/encoding/capacity failures leave the previous corpus unchanged.
    pub fn replace_documents(&mut self, documents: &[Arc<Document>]) -> Result<Update, Error> {
        self.replace_inputs(documents.iter().cloned().map(Input::Captured).collect())
    }
}
