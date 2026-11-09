//! Finite source/outline projections over an immutable captured document.

use crate::coloring::StyledSpan;
use crate::model::{Document, DocumentVersion, EntityId, FileId, SourceEncoding, SourceReadError};
use serde::{Deserialize, Serialize};

/// Limits count as well as unescaped UTF-8 text; JSON escaping has a finite multiplier.
pub const SOURCE_BYTES: usize = 32 * 1024;
/// Maximum source rows in a window.
pub const SOURCE_ROWS: usize = 96;
/// Maximum outline rows, excluding one separately returned cursor.
pub const OUTLINE_ROWS: usize = 200;
/// Maximum style spans across a complete source window.
pub const STYLE_SPANS: usize = 1024;

/// Source and outline ranges within one pinned document version.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct WindowQuery {
    /// Include bounded display-break hints and original line terminators.
    pub reflow: Option<bool>,
    /// One-based first source line.
    pub line_start: Option<usize>,
    /// Source row count, clamped to 1..96.
    pub line_count: Option<usize>,
    /// Line whose displayed fragment starts at `column`.
    pub column_line: Option<usize>,
    /// UTF-8 byte column, rounded down to a character boundary.
    pub column: Option<usize>,
    /// Zero-based first declaration ordinal.
    pub outline_start: Option<usize>,
    /// Declaration row count, clamped to 1..200.
    pub outline_count: Option<usize>,
    /// Current logical focus when no identity anchor is available.
    pub cursor_index: Option<usize>,
    /// Focus identity to reconcile across document changes.
    pub cursor: Option<String>,
    /// Top visible identity used for scroll anchoring.
    pub anchor: Option<String>,
    /// Anchor's offset from the requested outline window start.
    pub anchor_offset: Option<usize>,
}
/// A bounded declaration description and its logical position.
#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    /// Depth in the emitted lexical declaration forest.
    pub depth: usize,
    /// Nearest emitted enclosing scope label, at most 24 UTF-8 bytes including ellipsis.
    pub scope: Option<String>,
    /// Logical index across all configured outlines.
    pub index: usize,
    /// Analysis ordinal, preserving context membership without copying descriptors.
    pub analysis: usize,
    /// Opaque selectable identity.
    pub id: EntityId,
    /// UTF-8 display name, at most 128 bytes.
    pub name: String,
    /// Whether the complete name was longer than the display value.
    pub name_truncated: bool,
    /// Lexical kind, at most 32 UTF-8 bytes.
    pub kind: String,
    /// Whether the complete kind exceeded its display limit.
    pub kind_truncated: bool,
    /// Inclusive one-based starting source line.
    pub start_line: usize,
    /// Inclusive one-based ending source line.
    pub end_line: usize,
}
/// Source metadata without bulk text or declaration lists.
#[derive(Debug, Serialize)]
pub struct Summary {
    /// Logical source line count, zero for unsupported text.
    pub lines: usize,
    /// Logical declaration count across contexts.
    pub declarations: usize,
    /// Reconciled selected declaration, independently of loaded windows.
    pub active: Option<Entry>,
}
/// One bounded fragment, never a silently truncated whole line.
#[derive(Debug, Serialize)]
pub struct Line {
    /// Optional width-independent display hints for this captured fragment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflow: Option<crate::reflow::Hints>,
    /// Original line terminator, supplied with the display projection for exact copying.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending: Option<&'static str>,
    /// Styles in fragment-relative UTF-8 byte coordinates.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<StyledSpan>,
    /// One-based source line number.
    pub number: usize,
    /// First returned UTF-8 byte column.
    pub column: usize,
    /// Exclusive end byte column.
    pub end_column: usize,
    /// Complete line byte count, excluding CRLF/LF delimiters.
    pub line_bytes: usize,
    /// Literal captured text fragment.
    pub text: String,
    /// Earlier text exists in this line.
    pub left: bool,
    /// Later text exists in this line.
    pub right: bool,
}
/// Finite projection; the host adds the pinned snapshot identity.
#[derive(Debug, Serialize)]
pub struct Window {
    /// Whether complete styling is ready, limited, or unavailable.
    pub coloring: &'static str,
    /// Window protocol version.
    pub schema: u8,
    /// Captured file identity.
    pub file: FileId,
    /// Immutable document version.
    pub version: DocumentVersion,
    /// Logical source line count.
    pub line_total: usize,
    /// Actual first source line.
    pub line_start: usize,
    /// Bounded source fragments.
    pub lines: Vec<Line>,
    /// Logical outline count.
    pub outline_total: usize,
    /// Actual zero-based outline window start.
    pub outline_start: usize,
    /// Bounded outline entries.
    pub outline: Vec<Entry>,
    /// Focus row, including when outside the returned outline window.
    pub cursor: Option<Entry>,
}
fn floor(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}
fn label(text: &str, bytes: usize) -> String {
    text[..floor(text, bytes)].into()
}

impl Document {
    /// Counts declarations without constructing their transport descriptions.
    pub fn declaration_count(&self) -> usize {
        self.analyses().iter().map(|a| a.facts.len()).sum()
    }
    /// Projects one declaration by ordinal without projecting its neighbours.
    pub fn entry(&self, index: usize) -> Option<Entry> {
        let mut offset = index;
        for (ordinal, analysis) in self.analyses().iter().enumerate() {
            if let Some(fact) = analysis.facts.get(offset) {
                let lines = self.lines(&fact.span);
                return Some(Entry {
                    depth: self.outline_depths[index],
                    scope: fact.parent.map(|parent| {
                        let name = &analysis.facts[parent].name;
                        if name.len() <= 24 {
                            name.clone()
                        } else {
                            format!("{}…", label(name, 21))
                        }
                    }),
                    index,
                    analysis: ordinal,
                    id: self.entity_id(analysis, fact),
                    name: label(&fact.name, 128),
                    name_truncated: fact.name.len() > 128,
                    kind: label(&fact.kind, 32),
                    kind_truncated: fact.kind.len() > 32,
                    start_line: lines.start,
                    end_line: lines.end,
                });
            }
            offset -= analysis.facts.len();
        }
        None
    }
    pub(crate) fn entity_position(&self, id: &str) -> Option<usize> {
        self.outline_index
            .get_or_init(|| {
                let mut index = std::collections::BTreeMap::new();
                let mut ordinal = 0;
                for analysis in self.analyses() {
                    for fact in &analysis.facts {
                        index.insert(self.entity_id(analysis, fact).as_str().to_owned(), ordinal);
                        ordinal += 1;
                    }
                }
                index
            })
            .get(id)
            .copied()
    }
    fn recover(&self, id: &str, previous: Option<&Document>) -> Option<usize> {
        if self.declaration_count() == 0 {
            return None;
        }
        if let Some(position) = self.entity_position(id) {
            return Some(position);
        }
        let old = previous.filter(|old| old.id() == self.id())?;
        let position = old.entity_position(id)?;
        (position..old.declaration_count())
            .skip(1)
            .chain((0..position).rev())
            .find_map(|index| {
                old.entry(index)
                    .and_then(|row| self.entity_position(row.id.as_str()))
            })
    }
    /// Returns bounded text/outline rows; old source is used only for identity recovery.
    pub fn window(
        &self,
        query: &WindowQuery,
        previous: Option<&Document>,
    ) -> Result<Window, SourceReadError> {
        let line_total = self.captured().line_count();
        let line_start = query.line_start.unwrap_or(1).clamp(1, line_total.max(1));
        let count = query
            .line_count
            .unwrap_or(SOURCE_ROWS)
            .clamp(1, SOURCE_ROWS)
            .min(line_total.saturating_sub(line_start - 1));
        let allowance = SOURCE_BYTES / count.max(1);
        let mut lines = Vec::new();
        let colored = self.analyses().iter().find(|a| a.coloring.is_some());
        let coloring = colored.and_then(|a| a.coloring.as_ref());
        let mut coloring_status = coloring.map_or("unavailable", |c| {
            if c.available() { "ready" } else { "limited" }
        });
        let mut style_count = 0;
        let mut reader = self.captured().reader();
        if self.encoding() == SourceEncoding::Utf8 {
            for number in line_start..line_start.saturating_add(count) {
                if let Some(range) = self.captured().line_range(number) {
                    let column = reader.floor(
                        &range,
                        if query.column_line == Some(number) {
                            query.column.unwrap_or(0)
                        } else {
                            0
                        },
                    )?;
                    let end = reader.floor(&range, column.saturating_add(allowance))?;
                    let text = String::from_utf8(
                        reader
                            .read(range.start + column..range.start + end, allowance)?
                            .into_owned(),
                    )
                    .map_err(|_| SourceReadError::Integrity)?;
                    let styles = if coloring_status == "ready" {
                        match coloring.and_then(|c| {
                            c.spans(
                                range.start + column..range.start + end,
                                STYLE_SPANS - style_count,
                            )
                        }) {
                            Some(styles)
                                if styles.iter().all(|s| text.get(s.start..s.end).is_some()) =>
                            {
                                style_count += styles.len();
                                styles
                            }
                            _ => {
                                coloring_status = "limited";
                                Vec::new()
                            }
                        }
                    } else {
                        Vec::new()
                    };
                    lines.push(Line {
                        reflow: None,
                        ending: (query.reflow == Some(true)).then(|| {
                            let next = self
                                .captured()
                                .line_range(number + 1)
                                .map_or(self.captured().len(), |line| line.start);
                            match next - range.end {
                                2 => "\r\n",
                                1 => "\n",
                                _ => "",
                            }
                        }),
                        styles,
                        number,
                        column,
                        end_column: end,
                        line_bytes: range.len(),
                        text,
                        left: column > 0,
                        right: end < range.len(),
                    });
                }
            }
        }
        let total = self.declaration_count();
        if coloring_status != "ready" {
            for line in &mut lines {
                line.styles.clear();
            }
        }
        if query.reflow == Some(true) && coloring_status == "ready" {
            let mut allowance = crate::reflow::WINDOW_POINTS;
            for line in &mut lines {
                if !line.left {
                    line.reflow = colored.and_then(|a| {
                        crate::reflow::hints(
                            &a.context.language,
                            &line.text,
                            &line.styles,
                            allowance,
                        )
                    });
                    allowance -= line.reflow.as_ref().map_or(0, |h| h.points.len());
                }
            }
        }
        let cursor = query
            .cursor
            .as_deref()
            .and_then(|id| self.recover(id, previous))
            .unwrap_or(query.cursor_index.unwrap_or(0))
            .min(total.saturating_sub(1));
        let start = query
            .anchor
            .as_deref()
            .and_then(|id| self.entity_position(id))
            .map(|index| index.saturating_sub(query.anchor_offset.unwrap_or(0)))
            .unwrap_or(query.outline_start.unwrap_or(0))
            .min(total.saturating_sub(1));
        let end = start
            .saturating_add(
                query
                    .outline_count
                    .unwrap_or(OUTLINE_ROWS)
                    .clamp(1, OUTLINE_ROWS),
            )
            .min(total);
        Ok(Window {
            coloring: coloring_status,
            schema: 1,
            file: self.id().clone(),
            version: self.version().clone(),
            line_total,
            line_start,
            lines,
            outline_total: total,
            outline_start: start,
            outline: (start..end).filter_map(|index| self.entry(index)).collect(),
            cursor: self.entry(cursor),
        })
    }
}
