//! Literal source-pack selection, independent of retrieval and evaluator prose.
use crate::{Context, Corpus, Error, excerpt, require};
use serde::{Deserialize, Serialize};

/// A half-open byte range in an identified captured source file.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRange {
    /// Repository-relative source path.
    pub path: String,
    /// Expected full-file BLAKE3 identity.
    pub content: String,
    /// Inclusive UTF-8 byte offset.
    pub start: usize,
    /// Exclusive UTF-8 byte offset.
    pub end: usize,
}

impl Corpus {
    /// Selects exact captured ranges; never expands, truncates or accepts supplied text.
    pub fn select_ranges(
        &self,
        snapshot: &str,
        ranges: &[SourceRange],
        budget: usize,
    ) -> Result<Context, Error> {
        require(
            snapshot == self.snapshot,
            "Selection belongs to a different source snapshot",
        )?;
        require(
            (1..=8).contains(&ranges.len()) && (1..=16000).contains(&budget),
            "Invalid source-pack allowance",
        )?;
        let mut evidence = Vec::with_capacity(ranges.len());
        let mut used = 0;
        for (index, selected) in ranges.iter().enumerate() {
            let file = self
                .files
                .get(&selected.path)
                .ok_or_else(|| Error("Selected source is unavailable".into()))?;
            require(
                selected.content == file.content.to_hex().as_str(),
                "Selected source content identity changed",
            )?;
            require(
                selected.start < selected.end && selected.end <= file.text.len(),
                "Invalid source-pack byte range",
            )?;
            require(
                file.text.is_char_boundary(selected.start)
                    && file.text.is_char_boundary(selected.end),
                "Source-pack range splits UTF-8",
            )?;
            require(
                !ranges[..index].iter().any(|other| {
                    other.path == selected.path
                        && other.start < selected.end
                        && selected.start < other.end
                }),
                "Overlapping source-pack ranges",
            )?;
            used += selected.end - selected.start;
            require(used <= budget, "Source pack exceeds allowance")?;
            evidence.push(excerpt(file, &(selected.start..selected.end), index + 1));
        }
        Ok(Context {
            snapshot: self.snapshot.clone(),
            roles: vec![],
            evidence,
        })
    }
}
