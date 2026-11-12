//! Language-aware display breaks over bounded, already-styled captured lines.
use crate::coloring::{Style, StyledSpan};
use serde::Serialize;

/// Maximum retained break hints for one physical line.
pub const LINE_POINTS: usize = 256;
/// Maximum retained break hints across a source window.
pub const WINDOW_POINTS: usize = 1024;
/// A display break in original UTF-8 byte coordinates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Point {
    /// First byte of the continuation, after any original whitespace.
    pub offset: usize,
    /// Preferred alignment byte after an opening delimiter, when present.
    pub anchor: Option<usize>,
    /// Lower values prefer structural breaks; eight is a whitespace fallback.
    pub priority: u8,
}
/// Width-independent display hints, never a replacement source string.
#[derive(Debug, Serialize)]
pub struct Hints {
    /// End of the original leading spaces/tabs, in UTF-8 bytes.
    pub leading: usize,
    /// Unique ordered break positions, admitted atomically for this line.
    pub points: Vec<Point>,
}
fn space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}
fn word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 128
}
fn after_space(bytes: &[u8], mut at: usize) -> usize {
    while at < bytes.len() && space(bytes[at]) {
        at += 1;
    }
    at
}
fn point(
    points: &mut Vec<Point>,
    bytes: &[u8],
    leading: usize,
    at: usize,
    anchor: Option<usize>,
    priority: u8,
) {
    let offset = after_space(bytes, at);
    if offset > leading && offset < bytes.len() {
        points.push(Point {
            offset,
            anchor,
            priority,
        });
    }
}
/// Requires complete, non-overlapping parser-derived styles for these UTF-8 bytes.
/// Unknown languages and exceeded allowances return no hints, rather than partial ones.
pub(crate) fn hints(
    language: &str,
    text: &str,
    styles: &[StyledSpan],
    allowance: usize,
) -> Option<Hints> {
    let python = match language {
        "python" | "ruby" => true,
        "rust" | "c" | "cpp" => false,
        _ => return None,
    };
    let bytes = text.as_bytes();
    let leading = after_space(bytes, 0);
    let mut at = leading;
    let mut style = 0;
    let mut stack = Vec::new();
    let mut points = Vec::new();
    while at < bytes.len() {
        while style < styles.len() && styles[style].end <= at {
            style += 1;
        }
        if let Some(span) = styles.get(style).filter(|s| s.start <= at) {
            if !matches!(span.kind, Style::Keyword | Style::Comment) {
                at = span.end;
                continue;
            }
            if span.kind == Style::Comment {
                while at < span.end {
                    if space(bytes[at]) {
                        point(&mut points, bytes, leading, at, None, 4);
                        at = after_space(bytes, at);
                    } else {
                        at += 1;
                    }
                }
                continue;
            }
        }
        let anchor = stack.last().map(|(_, at)| *at);
        if space(bytes[at]) {
            point(&mut points, bytes, leading, at, anchor, 8);
            at = after_space(bytes, at);
            continue;
        }
        if word(bytes[at]) {
            let start = at;
            while at < bytes.len() && word(bytes[at]) {
                at += 1;
            }
            if python && matches!(&text[start..at], "and" | "or") {
                point(&mut points, bytes, leading, start, anchor, 1);
            }
            continue;
        }
        match bytes[at] {
            b'(' | b'[' | b'{' => {
                stack.push((bytes[at], at + 1));
                let close = match bytes[at] {
                    b'(' => b')',
                    b'[' => b']',
                    _ => b'}',
                };
                if bytes.get(after_space(bytes, at + 1)) != Some(&close) {
                    point(&mut points, bytes, leading, at + 1, Some(at + 1), 1);
                }
                at += 1;
            }
            b')' | b']' | b'}' => {
                let open = match bytes[at] {
                    b')' => b'(',
                    b']' => b'[',
                    _ => b'{',
                };
                if stack.last().is_some_and(|(b, _)| *b == open) {
                    stack.pop();
                } else {
                    stack.clear();
                }
                at += 1;
            }
            b',' | b';' => {
                point(&mut points, bytes, leading, at + 1, anchor, 0);
                at += 1;
            }
            b':' if bytes.get(at + 1) == Some(&b':') => {
                at += 2;
            }
            _ => {
                let rest = &text[at..];
                if let Some(op) = [
                    "<<=", ">>=", "**=", "//=", "->*", "<=>", "...", "..=", "&&", "||", "==", "!=",
                    "<=", ">=", "=>", "->", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "**",
                    "//", ":=", "<<", ">>", "++", "--", ".*", "..", ".", "=", "+", "-", "*", "/",
                    "%", "<", ">", "&", "|", "^", "!", "~",
                ]
                .into_iter()
                .find(|op| !op.is_empty() && rest.starts_with(op))
                {
                    point(
                        &mut points,
                        bytes,
                        leading,
                        at,
                        if op == "." { None } else { anchor },
                        if matches!(op, "&&" | "||") {
                            1
                        } else if op == "." {
                            2
                        } else {
                            3
                        },
                    );
                    at += op.len();
                } else {
                    at += 1;
                }
            }
        }
    }
    points.sort_by_key(|p| (p.offset, p.priority));
    points.dedup_by_key(|p| p.offset);
    (points.len() <= allowance.min(LINE_POINTS)).then_some(Hints { leading, points })
}
