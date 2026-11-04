//! Bounded raw-line edits and contextual hunks over an immutable comparison pair.

use crate::{
    comparison::{
        Comparison, EntryContent, EntryKind, EntryState, ReviewPath, Side, Unavailability,
    },
    model::{
        ComparisonId, Document, DocumentVersion, HunkId, SourceEncoding, SourceInput,
        SourceReadError,
    },
};
use std::ops::Range;

/// Per-file-pair admission and computation allowances; successful results are complete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffLimits {
    /// Combined captured bytes, checked before loading either document.
    pub max_bytes: usize,
    /// Combined physical lines, excluding the editor's final virtual empty row.
    pub max_lines: usize,
    /// Maximum complete contextual hunks.
    pub max_hunks: usize,
    /// Frontier, equality-byte, backtracking and output work allowance.
    pub max_work: usize,
}

impl Default for DiffLimits {
    fn default() -> Self {
        Self {
            max_bytes: 8 * 1024 * 1024,
            max_lines: 200_000,
            max_hunks: 10_000,
            max_work: 10_000_000,
        }
    }
}

/// A resource that prevented a complete textual diff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffLimit {
    /// Combined captured input size.
    Bytes,
    /// Combined physical line count.
    Lines,
    /// Number of contextual hunks.
    Hunks,
    /// Algorithm work allowance.
    Work,
}

/// Textual diff failure, distinct from a verified unchanged file.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DiffError {
    /// Neither observation contains the requested path.
    #[error("path is absent on both comparison sides")]
    UnknownPath,
    /// Presence or content could not be assessed.
    #[error("{side:?} source unavailable: {reason:?}")]
    Unavailable {
        /// Failed side.
        side: Side,
        /// Captured availability gap.
        reason: Unavailability,
    },
    /// A symlink, reference or special entry is not regular text.
    #[error("{0:?} entry is not a regular file")]
    UnsupportedKind(Side),
    /// Verified content identity exists, but no immutable document body is retained.
    #[error("{0:?} source body is not retained")]
    SourceNotRetained(Side),
    /// The captured source is not valid UTF-8.
    #[error("{0:?} source encoding is unsupported")]
    UnsupportedEncoding(Side),
    /// NUL-containing input is classified separately from its UTF-8 encoding.
    #[error("{0:?} source contains binary data")]
    Binary(Side),
    /// A verified immutable-source read failed.
    #[error("{side:?} source read failed: {source}")]
    Read {
        /// Failed side.
        side: Side,
        /// Captured-source IO, capacity or integrity failure.
        source: SourceReadError,
    },
    /// A complete result would exceed the configured allowance.
    #[error("text diff exceeds {0:?} allowance")]
    Limit(DiffLimit),
    /// Cooperative cancellation discarded all partial work.
    #[error("text diff cancelled")]
    Cancelled,
}

/// Corresponding zero-based, exclusive line and UTF-8 byte ranges on one side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffRange {
    /// Physical-line coordinates; empty ranges are insertion/deletion anchors.
    pub lines: Range<usize>,
    /// Exact source bytes, including original line terminators.
    pub bytes: Range<usize>,
}

/// One contiguous changed run, without context or copied source text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    /// Removed/replaced base bytes and lines.
    pub base: DiffRange,
    /// Inserted/replacement working bytes and lines.
    pub working: DiffRange,
}

/// A group of edits with up to three unchanged context lines on each side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextHunk {
    /// Comparison/path/policy-bound identity.
    pub id: HunkId,
    /// Base range including context.
    pub base: DiffRange,
    /// Working range including context.
    pub working: DiffRange,
    /// Exclusive indices into the result's ordered edit list.
    pub edits: Range<usize>,
}

/// Source metadata for interpreting physical-line ranges and missing sides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffSide {
    /// Retained document version; `None` means verified absence, even for empty input.
    pub version: Option<DocumentVersion>,
    /// Captured byte count.
    pub bytes: usize,
    /// Physical lines; an empty file has zero.
    pub lines: usize,
    /// Whether the last captured byte is LF; CRLF bytes are preserved in ranges.
    pub final_newline: bool,
}

/// Complete textual result; metadata does not own source bodies or publication leases.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextDiff {
    /// Ordered observation pair identity, not a publication token.
    pub comparison: ComparisonId,
    /// Lossless logical path.
    pub path: ReviewPath,
    /// Base presence/version and line metadata.
    pub base: DiffSide,
    /// Working presence/version and line metadata.
    pub working: DiffSide,
    /// Ordered, non-overlapping changed runs sufficient for byte reconstruction.
    pub edits: Vec<TextEdit>,
    /// Complete contextual hunks.
    pub hunks: Vec<TextHunk>,
    /// Work charged to this computation; not a duration or memory measurement.
    pub work: usize,
}

impl Comparison {
    /// Computes exact line edits from verified immutable regular UTF-8 documents.
    ///
    /// Missing sides are empty inputs. Unavailable/binary/nonregular source and exhausted
    /// limits are errors, never empty successful diffs. Cancellation is cooperative;
    /// an individual backing read is not preempted. No project tooling is invoked.
    pub fn text_diff(
        &self,
        path: &ReviewPath,
        limits: DiffLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<TextDiff, DiffError> {
        let mut budget = Budget {
            remaining: limits.max_work,
            cancelled,
        };
        budget.check()?;
        let entry = self.entry(path).ok_or(DiffError::UnknownPath)?;
        let base = document(self, path, Side::Base, entry.base)?;
        let working = document(self, path, Side::Working, entry.working)?;
        let bytes = base
            .map_or(0, Document::bytes)
            .checked_add(working.map_or(0, Document::bytes))
            .ok_or(DiffError::Limit(DiffLimit::Bytes))?;
        if bytes > limits.max_bytes {
            return Err(DiffError::Limit(DiffLimit::Bytes));
        }
        let base_input = base
            .map(|doc| load(doc, Side::Base, limits, &budget))
            .transpose()?;
        let working_input = working
            .map(|doc| load(doc, Side::Working, limits, &budget))
            .transpose()?;
        budget.check()?;
        let before = base_input.as_ref().map_or(Ok(""), |input| {
            input
                .text()
                .ok_or(DiffError::UnsupportedEncoding(Side::Base))
        })?;
        let after = working_input.as_ref().map_or(Ok(""), |input| {
            input
                .text()
                .ok_or(DiffError::UnsupportedEncoding(Side::Working))
        })?;
        if before.contains('\0') {
            return Err(DiffError::Binary(Side::Base));
        }
        if after.contains('\0') {
            return Err(DiffError::Binary(Side::Working));
        }
        let mut remaining_lines = limits.max_lines;
        let before = Lines::index(before, &mut remaining_lines, &budget)?;
        let after = Lines::index(after, &mut remaining_lines, &budget)?;
        let edits = edits(&before, &after, &mut budget)?;
        let hunks = hunks(
            self.id(),
            path,
            &before,
            &after,
            &edits,
            limits,
            &mut budget,
        )?;
        budget.check()?;
        Ok(TextDiff {
            comparison: self.id().clone(),
            path: path.clone(),
            base: before.metadata(base),
            working: after.metadata(working),
            edits,
            hunks,
            work: limits.max_work - budget.remaining,
        })
    }
}

fn load<'a>(
    doc: &'a Document,
    side: Side,
    limits: DiffLimits,
    budget: &Budget<'_>,
) -> Result<SourceInput<'a>, DiffError> {
    budget.check()?;
    doc.captured()
        .read_all(limits.max_bytes)
        .map_err(|source| DiffError::Read { side, source })
}

fn document<'a>(
    comparison: &'a Comparison,
    path: &ReviewPath,
    side: Side,
    state: Option<&EntryState>,
) -> Result<Option<&'a Document>, DiffError> {
    match state {
        None => Ok(None),
        Some(EntryState::Unobserved(reason)) => Err(DiffError::Unavailable {
            side,
            reason: *reason,
        }),
        Some(EntryState::Present { kind, content }) => {
            if !matches!(kind, EntryKind::Regular { .. }) {
                return Err(DiffError::UnsupportedKind(side));
            }
            if let EntryContent::Unavailable(reason) = content {
                return Err(DiffError::Unavailable {
                    side,
                    reason: *reason,
                });
            }
            let doc = comparison
                .document(side, path)
                .ok_or(DiffError::SourceNotRetained(side))?;
            if doc.encoding() != SourceEncoding::Utf8 {
                return Err(DiffError::UnsupportedEncoding(side));
            }
            Ok(Some(doc))
        }
    }
}

struct Budget<'a> {
    remaining: usize,
    cancelled: &'a dyn Fn() -> bool,
}

impl Budget<'_> {
    fn check(&self) -> Result<(), DiffError> {
        if (self.cancelled)() {
            Err(DiffError::Cancelled)
        } else {
            Ok(())
        }
    }
    fn spend(&mut self, amount: usize) -> Result<(), DiffError> {
        self.check()?;
        self.remaining = self
            .remaining
            .checked_sub(amount)
            .ok_or(DiffError::Limit(DiffLimit::Work))?;
        Ok(())
    }
    fn equal(&mut self, a: &str, b: &str) -> Result<bool, DiffError> {
        self.spend(1)?;
        if a.len() != b.len() {
            return Ok(false);
        }
        self.spend(a.len())?;
        Ok(a == b)
    }
}

struct Lines<'a> {
    text: &'a str,
    offsets: Vec<usize>,
}

impl<'a> Lines<'a> {
    fn index(text: &'a str, remaining: &mut usize, budget: &Budget<'_>) -> Result<Self, DiffError> {
        let mut offsets = vec![0];
        let mut at = 0;
        for line in text.split_inclusive('\n') {
            budget.check()?;
            *remaining = remaining
                .checked_sub(1)
                .ok_or(DiffError::Limit(DiffLimit::Lines))?;
            at += line.len();
            offsets.push(at);
        }
        Ok(Self { text, offsets })
    }
    fn len(&self) -> usize {
        self.offsets.len() - 1
    }
    fn line(&self, at: usize) -> &str {
        &self.text[self.offsets[at]..self.offsets[at + 1]]
    }
    fn range(&self, lines: Range<usize>) -> DiffRange {
        DiffRange {
            bytes: self.offsets[lines.start]..self.offsets[lines.end],
            lines,
        }
    }
    fn metadata(&self, document: Option<&Document>) -> DiffSide {
        DiffSide {
            version: document.map(|doc| doc.version().clone()),
            bytes: self.text.len(),
            lines: self.len(),
            final_newline: self.text.ends_with('\n'),
        }
    }
}

fn edits(
    base: &Lines<'_>,
    working: &Lines<'_>,
    budget: &mut Budget<'_>,
) -> Result<Vec<TextEdit>, DiffError> {
    if base.len() == 0 || working.len() == 0 {
        if base.len() == working.len() {
            return Ok(vec![]);
        }
        budget.spend(1)?;
        return Ok(vec![TextEdit {
            base: base.range(0..base.len()),
            working: working.range(0..working.len()),
        }]);
    }
    let trace = search(base, working, budget)?;
    backtrack(base, working, &trace, budget)
}

// Each row retains only reachable diagonal parity, with a prepaid cell allowance.
fn search(
    base: &Lines<'_>,
    working: &Lines<'_>,
    budget: &mut Budget<'_>,
) -> Result<Vec<Vec<usize>>, DiffError> {
    let mut trace: Vec<Vec<usize>> = vec![];
    for depth in 0..=base.len() + working.len() {
        budget.spend(depth + 1)?;
        let mut row = Vec::with_capacity(depth + 1);
        for index in 0..=depth {
            let mut x = if depth == 0 {
                0
            } else {
                let previous = &trace[depth - 1];
                if index == 0 || (index != depth && previous[index - 1] < previous[index]) {
                    previous[index]
                } else {
                    previous[index - 1] + 1
                }
            };
            let mut y = x + depth - 2 * index;
            while x < base.len()
                && y < working.len()
                && budget.equal(base.line(x), working.line(y))?
            {
                x += 1;
                y += 1;
            }
            row.push(x);
            if x >= base.len() && y >= working.len() {
                trace.push(row);
                return Ok(trace);
            }
        }
        trace.push(row);
    }
    Err(DiffError::Limit(DiffLimit::Work))
}

fn backtrack(
    base: &Lines<'_>,
    working: &Lines<'_>,
    trace: &[Vec<usize>],
    budget: &mut Budget<'_>,
) -> Result<Vec<TextEdit>, DiffError> {
    let mut edits: Vec<TextEdit> = vec![];
    let (mut x, mut y) = (base.len(), working.len());
    for depth in (1..trace.len()).rev() {
        budget.spend(1)?;
        let index = (x + depth - y) / 2;
        let previous = &trace[depth - 1];
        let previous_index =
            if index == 0 || (index != depth && previous[index - 1] < previous[index]) {
                index
            } else {
                index - 1
            };
        let before_x = previous[previous_index];
        let before_y = before_x + depth - 1 - 2 * previous_index;
        while x > before_x && y > before_y {
            budget.spend(1)?;
            x -= 1;
            y -= 1;
        }
        let edit = TextEdit {
            base: base.range(before_x..x),
            working: working.range(before_y..y),
        };
        // Equal lines between edits advance both coordinates together.
        if let Some(last) = edits.last_mut().filter(|last| last.base.lines.start == x) {
            last.base.lines.start = edit.base.lines.start;
            last.base.bytes.start = edit.base.bytes.start;
            last.working.lines.start = edit.working.lines.start;
            last.working.bytes.start = edit.working.bytes.start;
        } else {
            edits.push(edit);
        }
        (x, y) = (before_x, before_y);
    }
    edits.reverse();
    Ok(edits)
}

fn hunks(
    comparison: &ComparisonId,
    path: &ReviewPath,
    base: &Lines<'_>,
    working: &Lines<'_>,
    edits: &[TextEdit],
    limits: DiffLimits,
    budget: &mut Budget<'_>,
) -> Result<Vec<TextHunk>, DiffError> {
    let policy: Vec<_> = [
        limits.max_bytes,
        limits.max_lines,
        limits.max_hunks,
        limits.max_work,
    ]
    .into_iter()
    .flat_map(|value| (value as u64).to_le_bytes())
    .collect();
    let mut hunks = vec![];
    let mut first = 0;
    while first < edits.len() {
        budget.spend(1)?;
        if hunks.len() >= limits.max_hunks {
            return Err(DiffError::Limit(DiffLimit::Hunks));
        }
        let before_start = edits[first].base.lines.start.saturating_sub(3);
        let after_start = edits[first].working.lines.start.saturating_sub(3);
        let mut before_end = edits[first]
            .base
            .lines
            .end
            .saturating_add(3)
            .min(base.len());
        let mut after_end = edits[first]
            .working
            .lines
            .end
            .saturating_add(3)
            .min(working.len());
        let mut end = first + 1;
        while end < edits.len() {
            budget.spend(1)?;
            if edits[end].base.lines.start.saturating_sub(3) > before_end {
                break;
            }
            before_end = edits[end].base.lines.end.saturating_add(3).min(base.len());
            after_end = edits[end]
                .working
                .lines
                .end
                .saturating_add(3)
                .min(working.len());
            end += 1;
        }
        let coordinates: Vec<_> = [before_start, before_end, after_start, after_end, first, end]
            .into_iter()
            .flat_map(|value| (value as u64).to_le_bytes())
            .collect();
        let id = HunkId::derive(&[
            b"raw-line-myers-v1",
            comparison.as_str().as_bytes(),
            path.bytes(),
            &policy,
            &coordinates,
        ]);
        hunks.push(TextHunk {
            id,
            base: base.range(before_start..before_end),
            working: working.range(after_start..after_end),
            edits: first..end,
        });
        first = end;
    }
    Ok(hunks)
}
