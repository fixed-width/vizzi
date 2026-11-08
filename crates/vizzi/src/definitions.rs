//! Host definition providers with captured-source positions and explicit semantic context.
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vizzi_core::{model::Snapshot, structure::CallLookup};

/// LSP position measured in UTF-16 code units, with zero-based lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    /// Zero-based line.
    pub line: u32,
    /// UTF-16 units from the start of the line.
    pub character: u32,
}
/// A provider definition within the captured workspace.
#[derive(Clone, Debug)]
pub struct Location {
    /// Normalized workspace-relative source path.
    pub path: String,
    /// Exact definition identifier position.
    pub position: Position,
}
/// Semantic results, distinct from terminal-name matches.
#[derive(Clone, Debug)]
pub struct Answer {
    /// Captured-workspace definitions.
    pub locations: Vec<Location>,
    /// Additional known definitions outside the captured source.
    pub outside: usize,
    /// Provider/configuration/source context identity.
    pub context: String,
}
/// An editor-neutral definition seam over one immutable source observation.
pub trait Provider: Send + Sync {
    /// Resolves the captured call; implementations must not substitute live source bytes.
    fn resolve(&self, snapshot: Arc<Snapshot>, call: &CallLookup) -> Result<Answer, String>;
}
/// Converts a captured byte offset to the negotiated LSP position.
pub fn position(text: &str, byte: usize) -> Result<Position, String> {
    let prefix = text.get(..byte).ok_or("Invalid captured UTF-8 position")?;
    if prefix.ends_with('\r') {
        return Err("Position splits a line ending".into());
    }
    let line = prefix.bytes().filter(|b| *b == b'\n').count();
    let start = prefix.rfind('\n').map_or(0, |n| n + 1);
    Ok(Position {
        line: line.try_into().map_err(|_| "Line exceeds LSP bounds")?,
        character: prefix[start..]
            .encode_utf16()
            .count()
            .try_into()
            .map_err(|_| "Column exceeds LSP bounds")?,
    })
}
/// Converts an LSP definition position to an exact captured byte offset.
pub fn byte_offset(text: &str, position: Position) -> Result<usize, String> {
    let start = if position.line == 0 {
        0
    } else {
        text.match_indices('\n')
            .nth(position.line as usize - 1)
            .map(|(n, _)| n + 1)
            .ok_or("Definition line is outside captured source")?
    };
    let line = text[start..]
        .split('\n')
        .next()
        .unwrap_or_default()
        .trim_end_matches('\r');
    let mut units = 0;
    for (byte, ch) in line.char_indices() {
        if units == position.character as usize {
            return Ok(start + byte);
        }
        units += ch.len_utf16();
        if units > position.character as usize {
            return Err("Definition splits a UTF-16 surrogate".into());
        }
    }
    if units == position.character as usize {
        Ok(start + line.len())
    } else {
        Err("Definition column is outside captured source".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positions_round_trip_utf8_and_utf16_without_splitting_a_surrogate() {
        let text = "// 😀\r\nfn café() {}\n";
        let byte = text.find("café").unwrap();
        assert_eq!(
            position(text, byte).unwrap(),
            Position {
                line: 1,
                character: 3
            }
        );
        assert_eq!(
            byte_offset(
                text,
                Position {
                    line: 1,
                    character: 7
                }
            )
            .unwrap(),
            text.find('(').unwrap()
        );
        assert_eq!(
            position(text, 7).unwrap(),
            Position {
                line: 0,
                character: 5
            }
        );
        assert_eq!(
            byte_offset(
                text,
                Position {
                    line: 1,
                    character: 12
                }
            )
            .unwrap(),
            text.len() - 1
        );
        assert_eq!(
            byte_offset(
                text,
                Position {
                    line: 2,
                    character: 0
                }
            )
            .unwrap(),
            text.len()
        );
        assert!(
            byte_offset(
                text,
                Position {
                    line: 0,
                    character: 4
                }
            )
            .is_err()
        );
        assert!(position(text, 4).is_err());
        assert!(
            byte_offset(
                text,
                Position {
                    line: 99,
                    character: 0
                }
            )
            .is_err()
        );
        assert!(
            byte_offset(
                text,
                Position {
                    line: 1,
                    character: 99
                }
            )
            .is_err()
        );
    }
}
