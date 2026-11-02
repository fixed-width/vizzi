//! Owned, transferable syntax state; no parser or published document is retained.

use crate::model::{Analysis, ContentId, Context};
use tree_sitter::{InputEdit, Point, Tree};

/// Complete analysis with optional disposable syntax state.
pub struct AnalysisResult {
    /// Facts for exactly the new captured source and registered context.
    pub analysis: Analysis,
    /// Owned state for a later call; absent when unsupported or over its allowance.
    pub state: Option<SyntaxState>,
    /// Whether a compatible old tree was supplied to the parser.
    pub tree_reused: bool,
}

/// Opaque, nonserialisable tree paired with its exact source and full context.
pub struct SyntaxState {
    pub(crate) tree: Tree,
    pub(crate) source: Box<str>,
    context: Context,
    content: ContentId,
    nodes: usize,
    weight: usize,
}
impl SyntaxState {
    pub(crate) fn capture(
        context: Context,
        content: ContentId,
        source: &str,
        tree: Tree,
        limit: usize,
    ) -> Option<Self> {
        let nodes = tree.root_node().descendant_count();
        let weight = source
            .len()
            .checked_add(nodes.checked_mul(128)?)?
            .checked_add(4096)?;
        if weight > limit {
            return None;
        }
        Some(Self {
            tree,
            source: source.into(),
            context,
            content,
            nodes,
            weight,
        })
    }
    /// Admission weight, not a measurement of native heap allocation.
    pub fn weight(&self) -> usize {
        self.weight
    }
    /// Exact retained source byte count.
    pub fn source_bytes(&self) -> usize {
        self.source.len()
    }
    /// Syntax descendant count, including the root and anonymous nodes.
    pub fn nodes(&self) -> usize {
        self.nodes
    }
    /// Complete context of the retained parse.
    pub fn context(&self) -> &Context {
        &self.context
    }
    /// Hash of the exact retained source.
    pub fn content(&self) -> &ContentId {
        &self.content
    }
}

pub(crate) fn edit_between(
    old: &str,
    new: &str,
    cancelled: &dyn Fn() -> bool,
) -> Option<InputEdit> {
    if cancelled() {
        return None;
    }
    let old_bytes = old.as_bytes();
    let new_bytes = new.as_bytes();
    let mut start = 0;
    let common = old.len().min(new.len());
    while start < common {
        if cancelled() {
            return None;
        }
        let end = (start + 65536).min(common);
        let same = old_bytes[start..end]
            .iter()
            .zip(&new_bytes[start..end])
            .take_while(|(a, b)| a == b)
            .count();
        start += same;
        if start < end {
            break;
        }
    }
    // A shared UTF-8 prefix/suffix has matching character boundaries.
    while !old.is_char_boundary(start) {
        start -= 1;
    }
    let mut suffix = 0;
    let remaining = common - start;
    while suffix < remaining {
        if cancelled() {
            return None;
        }
        let count = (remaining - suffix).min(65536);
        let same = old_bytes[old.len() - suffix - count..old.len() - suffix]
            .iter()
            .rev()
            .zip(
                new_bytes[new.len() - suffix - count..new.len() - suffix]
                    .iter()
                    .rev(),
            )
            .take_while(|(a, b)| a == b)
            .count();
        suffix += same;
        if same < count {
            break;
        }
    }
    let mut old_end = old.len() - suffix;
    let mut new_end = new.len() - suffix;
    while !old.is_char_boundary(old_end) {
        old_end += 1;
        new_end += 1;
    }
    let start_position = advance(Point::new(0, 0), &old_bytes[..start], cancelled)?;
    Some(InputEdit {
        start_byte: start,
        old_end_byte: old_end,
        new_end_byte: new_end,
        start_position,
        old_end_position: advance(start_position, &old_bytes[start..old_end], cancelled)?,
        new_end_position: advance(start_position, &new_bytes[start..new_end], cancelled)?,
    })
}
fn advance(mut point: Point, bytes: &[u8], cancelled: &dyn Fn() -> bool) -> Option<Point> {
    for chunk in bytes.chunks(65536) {
        if cancelled() {
            return None;
        }
        for byte in chunk {
            if *byte == b'\n' {
                point.row += 1;
                point.column = 0;
            } else {
                point.column += 1;
            }
        }
    }
    Some(point)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edit_coordinates_expand_multibyte_boundaries_and_count_byte_columns() {
        let edit = edit_between("α\n🙂old\n", "α\n🙃new\n", &|| false).unwrap();
        assert_eq!(
            (edit.start_byte, edit.old_end_byte, edit.new_end_byte),
            (3, 10, 10)
        );
        assert_eq!(edit.start_position, Point::new(1, 0));
        assert_eq!(edit.old_end_position, Point::new(1, 7));
        assert_eq!(edit.new_end_position, Point::new(1, 7));
        assert!(edit_between("abc", "def", &|| true).is_none());
        let edit = edit_between("à", "Ϡ", &|| false).unwrap();
        assert_eq!(
            (edit.start_byte, edit.old_end_byte, edit.new_end_byte),
            (0, 2, 2)
        );
        let edit = edit_between("abcabc", "abc", &|| false).unwrap();
        assert_eq!(
            (edit.start_byte, edit.old_end_byte, edit.new_end_byte),
            (3, 6, 3)
        );
        let edit = edit_between("abc", "abcabc", &|| false).unwrap();
        assert_eq!(
            (edit.start_byte, edit.old_end_byte, edit.new_end_byte),
            (3, 3, 6)
        );
        let repeated = "x".repeat(131073);
        let edit = edit_between(&repeated, &format!("z{repeated}"), &|| false).unwrap();
        assert_eq!(
            (edit.start_byte, edit.old_end_byte, edit.new_end_byte),
            (0, 0, 1)
        );
        let edit = edit_between("abc", "abc", &|| false).unwrap();
        assert_eq!(
            (edit.start_byte, edit.old_end_byte, edit.new_end_byte),
            (3, 3, 3)
        );
        let prefix = "x".repeat(131072);
        let suffix = "y".repeat(131072);
        let old = format!("{prefix}α🙂oldβ{suffix}");
        let new = format!("{prefix}αρnewβ{suffix}");
        let edit = edit_between(&old, &new, &|| false).unwrap();
        assert_eq!(
            (edit.start_byte, edit.old_end_byte, edit.new_end_byte),
            (131074, 131081, 131079)
        );
        assert_eq!(edit.old_end_position, Point::new(0, 131081));
        assert_eq!(edit.new_end_position, Point::new(0, 131079));
        let edit = edit_between("a\r\nb", "a\r\nb\nlast", &|| false).unwrap();
        assert_eq!(edit.start_position, Point::new(1, 1));
        assert_eq!(edit.new_end_position, Point::new(2, 4));
    }
}
