//! Immutable, bounded syntax styles in captured UTF-8 byte coordinates.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use std::{ops::Range, sync::Arc};

/// Maximum packed styling data retained for one document.
pub const INDEX_BYTES: usize = 1024 * 1024;
/// Theme-independent lexical styling category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Style {
    /// Language syntax and control words.
    Keyword = 1,
    /// Type names and built-in types.
    Type,
    /// Functions, methods and macros.
    Function,
    /// String and character literals.
    String,
    /// Comments and documentation comments.
    Comment,
    /// Numeric literals.
    Number,
    /// Constants and literal booleans.
    Constant,
    /// Attributes and decorators.
    Attribute,
}
impl Style {
    pub(crate) fn capture(name: &str) -> Option<Self> {
        Some(match name.split('.').next()? {
            "keyword" => Self::Keyword,
            "type" | "constructor" => Self::Type,
            "function" => Self::Function,
            "string" | "character" => Self::String,
            "comment" => Self::Comment,
            "number" | "float" => Self::Number,
            "constant" | "boolean" => Self::Constant,
            "attribute" => Self::Attribute,
            _ => return None,
        })
    }
    fn tag(tag: u64) -> Option<Self> {
        Some(match tag {
            1 => Self::Keyword,
            2 => Self::Type,
            3 => Self::Function,
            4 => Self::String,
            5 => Self::Comment,
            6 => Self::Number,
            7 => Self::Constant,
            8 => Self::Attribute,
            _ => return None,
        })
    }
}
/// One non-overlapping style range relative to a returned UTF-8 fragment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyledSpan {
    /// Inclusive byte offset.
    pub start: usize,
    /// Exclusive byte offset.
    pub end: usize,
    /// Lexical styling category.
    pub kind: Style,
}
/// Compact immutable boundary index; plain text is represented by tag zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coloring {
    runs: Arc<[u64]>,
    limited: bool,
}
impl Coloring {
    /// Admits ordered captures, merging adjacent styles and preferring outer captures.
    /// Exceeding the byte allowance discards the entire index, never partial colours.
    pub fn from_spans(
        spans: impl IntoIterator<Item = (Range<usize>, Style)>,
        byte_limit: usize,
    ) -> Self {
        let mut runs = Vec::<u64>::new();
        let limit = byte_limit.min(INDEX_BYTES) / 8;
        let mut end = 0;
        for (range, kind) in spans {
            if range.start >= range.end || range.start < end {
                continue;
            }
            let Some(start) = u64::try_from(range.start)
                .ok()
                .and_then(|v| v.checked_mul(16))
            else {
                return Self::limited();
            };
            let Some(finish) = u64::try_from(range.end)
                .ok()
                .and_then(|v| v.checked_mul(16))
            else {
                return Self::limited();
            };
            if range.start == end && !runs.is_empty() {
                runs.pop();
            }
            if runs.last().is_none_or(|v| v & 15 != kind as u64) {
                runs.push(start | kind as u64);
            }
            runs.push(finish);
            if runs.len() > limit {
                return Self::limited();
            }
            end = range.end;
        }
        Self {
            runs: runs.into(),
            limited: false,
        }
    }
    pub(crate) fn limited() -> Self {
        Self {
            runs: Arc::from([]),
            limited: true,
        }
    }
    /// Whether the complete styling index was admitted.
    pub fn available(&self) -> bool {
        !self.limited
    }
    /// Checks that all boundaries fit within the captured document.
    pub fn valid_for(&self, bytes: usize) -> bool {
        self.runs.last().is_none_or(|v| v >> 4 <= bytes as u64)
    }
    pub(crate) fn valid_text(&self, text: &str) -> bool {
        self.runs
            .iter()
            .all(|v| text.is_char_boundary((v >> 4) as usize))
    }
    /// Clips styles using indexed lookup; `None` means the index or output exceeded its limit.
    pub fn spans(&self, range: Range<usize>, limit: usize) -> Option<Vec<StyledSpan>> {
        if self.limited {
            return None;
        }
        let mut spans = Vec::new();
        let start = self
            .runs
            .partition_point(|v| v >> 4 <= range.start as u64)
            .saturating_sub(1);
        for pair in self.runs[start..].windows(2) {
            let begin = (pair[0] >> 4) as usize;
            if begin >= range.end {
                break;
            }
            if let Some(kind) = Style::tag(pair[0] & 15) {
                let end = ((pair[1] >> 4) as usize).min(range.end);
                let begin = begin.max(range.start);
                if begin < end {
                    if spans.len() == limit {
                        return None;
                    }
                    spans.push(StyledSpan {
                        start: begin - range.start,
                        end: end - range.start,
                        kind,
                    });
                }
            }
        }
        Some(spans)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Encoded {
    data: String,
    limited: bool,
}
impl Serialize for Coloring {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let bytes: Vec<_> = self.runs.iter().flat_map(|v| v.to_le_bytes()).collect();
        Encoded {
            data: STANDARD.encode(bytes),
            limited: self.limited,
        }
        .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for Coloring {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let encoded = Encoded::deserialize(deserializer)?;
        if encoded.data.len() > INDEX_BYTES.div_ceil(3) * 4 {
            return Err(D::Error::custom("styling index exceeds byte limit"));
        }
        let bytes = STANDARD.decode(encoded.data).map_err(D::Error::custom)?;
        if bytes.len() > INDEX_BYTES || bytes.len() % 8 != 0 {
            return Err(D::Error::custom("invalid styling index length"));
        }
        let runs: Vec<_> = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|v| u64::from_le_bytes(*v))
            .collect();
        if (encoded.limited && !runs.is_empty())
            || runs.iter().any(|v| v & 15 > 8)
            || runs
                .windows(2)
                .any(|p| p[0] >> 4 >= p[1] >> 4 || p[0] & 15 == p[1] & 15)
            || runs.first().is_some_and(|v| v & 15 == 0)
            || runs.last().is_some_and(|v| v & 15 != 0)
        {
            return Err(D::Error::custom("invalid styling boundaries"));
        }
        Ok(Self {
            runs: runs.into(),
            limited: encoded.limited,
        })
    }
}
