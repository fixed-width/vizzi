//! Host definition providers with captured-source positions and explicit semantic context.
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vizzi_core::{
    model::Snapshot,
    structure::{CallLookup, Symbol},
};

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
    /// Independent dependency capture, absent for repository source.
    pub capture: Option<String>,
    /// Normalized workspace-relative source path.
    pub path: String,
    /// Exact provider position in the captured declaration.
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
    /// Finds semantic references to a captured declaration identifier.
    fn references(
        &self,
        _snapshot: Arc<Snapshot>,
        _symbol: &Symbol,
        _identifier: usize,
    ) -> Result<Answer, String> {
        Err("Reference analysis is unavailable for this language".into())
    }
    /// Captured dependency sources admitted by this language provider, if supported.
    fn dependencies(&self) -> Option<&crate::dependency_sources::Catalog> {
        None
    }
    /// Whether this provider offers managed Java project import.
    fn supports_java_project_import(&self) -> bool {
        false
    }
    /// Reports the captured Java build system available for explicit import.
    fn java_project_kind(&self, _snapshot: &Snapshot) -> Option<&'static str> {
        None
    }
    /// Loads captured Java project metadata following an explicit user action.
    fn import_java_project(
        &self,
        _snapshot: Arc<Snapshot>,
        _current: &dyn Fn() -> Option<Arc<Snapshot>>,
    ) -> Result<(), String> {
        Err("Java project import requires a managed JDT LS server".into())
    }
    /// Resolves the captured call; implementations must not substitute live source bytes.
    fn resolve(&self, snapshot: Arc<Snapshot>, call: &CallLookup) -> Result<Answer, String>;
}

fn line_starts(text: &str) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(0).chain(text.bytes().enumerate().filter_map(move |(byte, value)| {
        let next = byte + 1;
        match value {
            b'\n' => Some(next),
            b'\r' if text.as_bytes().get(next) != Some(&b'\n') => Some(next),
            _ => None,
        }
    }))
}

/// Converts a captured byte offset to the negotiated LSP position.
pub fn position(text: &str, byte: usize) -> Result<Position, String> {
    let prefix = text.get(..byte).ok_or("Invalid captured UTF-8 position")?;
    if prefix.ends_with('\r') && text.as_bytes().get(byte) == Some(&b'\n') {
        return Err("Position splits a line ending".into());
    }
    let (line, start) = line_starts(prefix).enumerate().last().unwrap_or_default();
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
    let start = line_starts(text)
        .nth(position.line as usize)
        .ok_or("Definition line is outside captured source")?;
    let line = text[start..].split(['\r', '\n']).next().unwrap_or_default();
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

pub(crate) fn reference_offset(language: Option<&str>, name: &str) -> usize {
    if !matches!(language, Some("c" | "cpp")) {
        return 0;
    }
    name.rfind("::operator")
        .map(|at| at + 2)
        .or_else(|| name.starts_with("operator").then_some(0))
        .unwrap_or_else(|| name.rfind("::").map_or(0, |at| at + 2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_round_trip_mixed_line_endings_and_empty_lines() {
        let text = "😀\rfirst\n\nsecond\r\n\rlast\r";
        for (byte, line, character) in [
            (0, 0, 0),
            (4, 0, 2),
            (5, 1, 0),
            (10, 1, 5),
            (11, 2, 0),
            (12, 3, 0),
            (18, 3, 6),
            (20, 4, 0),
            (21, 5, 0),
            (26, 6, 0),
        ] {
            let point = Position { line, character };
            assert_eq!(position(text, byte).unwrap(), point, "byte {byte}");
            assert_eq!(byte_offset(text, point).unwrap(), byte, "{point:?}");
        }
        assert!(position(text, 19).is_err());
        assert!(
            byte_offset(
                text,
                Position {
                    line: 2,
                    character: 1
                }
            )
            .is_err()
        );
        assert!(
            byte_offset(
                text,
                Position {
                    line: 7,
                    character: 0
                }
            )
            .is_err()
        );
    }

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
