//! Syntax-only parser experiments; no name resolution or semantic indexing.

use serde::{Deserialize, Serialize};
use std::path::Path;
use tree_sitter::{InputEdit, Parser, Point, Tree};

/// The grammar is chosen explicitly for each experiment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceLanguage {
    /// Rust source.
    Rust,
    /// Python source.
    Python,
    /// C source and headers within a C experiment.
    C,
    /// C++ source and headers within a C++ experiment; excludes CUDA.
    Cpp,
}

/// A rejected edit or unavailable parser result.
#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    /// The replacement range is outside the source buffer.
    #[error("edit range must satisfy start <= end <= source length")]
    InvalidRange,
    /// Parsing did not produce a tree.
    #[error("parser did not produce a syntax tree")]
    NoTree,
    /// The grammar cannot be used with this parser version.
    #[error(transparent)]
    Language(#[from] tree_sitter::LanguageError),
}

impl SourceLanguage {
    /// Select file extensions within the explicitly chosen language scope.
    pub fn accepts(self, path: &Path) -> bool {
        let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
            return false;
        };
        match self {
            Self::Rust => extension == "rs",
            Self::Python => matches!(extension, "py" | "pyi"),
            Self::C => matches!(extension, "c" | "h"),
            Self::Cpp => matches!(
                extension,
                "cpp" | "cc" | "cxx" | "h" | "hpp" | "hh" | "hxx" | "inl" | "C"
            ),
        }
    }

    /// Create a reusable parser for this grammar.
    pub fn parser(self) -> Result<Parser, SyntaxError> {
        let language = match self {
            Self::Rust => tree_sitter_rust::LANGUAGE,
            Self::Python => tree_sitter_python::LANGUAGE,
            Self::C => tree_sitter_c::LANGUAGE,
            Self::Cpp => tree_sitter_cpp::LANGUAGE,
        };
        let mut parser = Parser::new();
        parser.set_language(&language.into())?;
        Ok(parser)
    }
}

/// Edited contents and the matching byte-oriented tree edit.
pub struct PreparedEdit {
    /// Complete new source buffer.
    pub source: Vec<u8>,
    /// Source positions required by the incremental parser.
    pub edit: InputEdit,
}

/// Prepare a replacement without changing the input buffer.
pub fn prepare_edit(
    source: &[u8],
    start: usize,
    end: usize,
    replacement: &[u8],
) -> Result<PreparedEdit, SyntaxError> {
    if start > end || end > source.len() {
        return Err(SyntaxError::InvalidRange);
    }
    let new_end_byte = start
        .checked_add(replacement.len())
        .ok_or(SyntaxError::InvalidRange)?;
    let size = (source.len() - end)
        .checked_add(new_end_byte)
        .ok_or(SyntaxError::InvalidRange)?;
    let start_position = advance(Point::new(0, 0), &source[..start]);
    let old_end_position = advance(start_position, &source[start..end]);
    let new_end_position = advance(start_position, replacement);
    let mut changed = Vec::with_capacity(size);
    changed.extend_from_slice(&source[..start]);
    changed.extend_from_slice(replacement);
    changed.extend_from_slice(&source[end..]);
    Ok(PreparedEdit {
        source: changed,
        edit: InputEdit {
            start_byte: start,
            old_end_byte: end,
            new_end_byte,
            start_position,
            old_end_position,
            new_end_position,
        },
    })
}

fn advance(mut point: Point, bytes: &[u8]) -> Point {
    for byte in bytes {
        if *byte == b'\n' {
            point.row += 1;
            point.column = 0;
        } else {
            point.column += 1;
        }
    }
    point
}

/// Reparse a cheaply cloned tree, preserving the previous tree for comparison.
pub fn reparse(
    parser: &mut Parser,
    previous: &Tree,
    prepared: &PreparedEdit,
) -> Result<Tree, SyntaxError> {
    let mut edited = previous.clone();
    edited.edit(&prepared.edit);
    parser
        .parse(&prepared.source, Some(&edited))
        .ok_or(SyntaxError::NoTree)
}

/// Syntax counts used to expose grammar errors in corpus measurements.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxCounts {
    /// All syntax nodes, including punctuation.
    pub nodes: usize,
    /// Nodes representing named grammar constructs.
    pub named: usize,
    /// Error nodes produced by the grammar.
    pub errors: usize,
    /// Missing nodes inserted by parser error recovery.
    pub missing: usize,
}

/// Count every syntax node without recursion.
pub fn inspect(tree: &Tree) -> SyntaxCounts {
    let mut counts = SyntaxCounts::default();
    visit(tree, |cursor| {
        let node = cursor.node();
        counts.nodes += 1;
        counts.named += usize::from(node.is_named());
        counts.errors += usize::from(node.is_error());
        counts.missing += usize::from(node.is_missing());
    });
    counts
}

/// One node in a canonical preorder tree description.
#[derive(Debug, PartialEq, Eq)]
pub struct NodeShape {
    /// Node kind from the selected grammar.
    pub kind: String,
    /// Byte range occupied by this node.
    pub range: std::ops::Range<usize>,
    /// Depth relative to the root node.
    pub depth: u32,
    /// Grammar field identifying this node's role within its parent.
    pub field: Option<String>,
    /// Whether the grammar names this node.
    pub named: bool,
    /// Whether this is an error node.
    pub error: bool,
    /// Whether error recovery inserted this node.
    pub missing: bool,
}

/// Describe all nodes and ranges for comparison within the same grammar.
pub fn shape(tree: &Tree) -> Vec<NodeShape> {
    let mut nodes = Vec::new();
    visit(tree, |cursor| {
        let node = cursor.node();
        nodes.push(NodeShape {
            kind: node.kind().into(),
            range: node.byte_range(),
            depth: cursor.depth(),
            field: cursor.field_name().map(str::to_owned),
            named: node.is_named(),
            error: node.is_error(),
            missing: node.is_missing(),
        });
    });
    nodes
}

fn visit(tree: &Tree, mut action: impl FnMut(&tree_sitter::TreeCursor<'_>)) {
    let mut cursor = tree.walk();
    loop {
        action(&cursor);
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes_extensions_without_treating_cuda_as_cpp_or_lpc_as_rust() {
        assert!(SourceLanguage::Rust.accepts(Path::new("main.rs")));
        assert!(!SourceLanguage::Rust.accepts(Path::new("fixture.c")));
        assert!(SourceLanguage::Python.accepts(Path::new("main.py")));
        assert!(!SourceLanguage::Python.accepts(Path::new("main.rs")));
        assert!(SourceLanguage::C.accepts(Path::new("include/api.h")));
        assert!(!SourceLanguage::C.accepts(Path::new("api.cpp")));
        assert!(SourceLanguage::Cpp.accepts(Path::new("api.cpp")));
        assert!(SourceLanguage::Cpp.accepts(Path::new("api.hpp")));
        assert!(!SourceLanguage::Cpp.accepts(Path::new("kernel.cu")));
        assert!(!SourceLanguage::Cpp.accepts(Path::new("README")));
    }

    #[test]
    fn byte_positions_handle_multibyte_characters_and_newlines() {
        let source = "ab\né\nx".as_bytes();
        let prepared = prepare_edit(source, 3, 5, b"z\n").unwrap();
        assert_eq!(prepared.source, b"ab\nz\n\nx");
        assert_eq!(
            (
                prepared.edit.start_byte,
                prepared.edit.old_end_byte,
                prepared.edit.new_end_byte
            ),
            (3, 5, 5)
        );
        assert_eq!(
            (
                prepared.edit.start_position,
                prepared.edit.old_end_position,
                prepared.edit.new_end_position
            ),
            (Point::new(1, 0), Point::new(1, 2), Point::new(2, 0))
        );
    }

    #[test]
    fn invalid_ranges_are_rejected() {
        assert!(prepare_edit(b"abc", 2, 1, b"").is_err());
        assert!(prepare_edit(b"abc", 0, 4, b"").is_err());
    }

    #[test]
    fn empty_buffer_insert_and_same_line_replacement_have_correct_positions() {
        let inserted = prepare_edit(b"", 0, 0, b"abc").unwrap();
        assert_eq!(
            (inserted.source, inserted.edit.new_end_position),
            (b"abc".to_vec(), Point::new(0, 3))
        );
        let edited = prepare_edit(b"abc", 1, 2, b"xyz").unwrap();
        assert_eq!(
            (
                edited.source,
                edited.edit.start_position,
                edited.edit.new_end_position
            ),
            (b"axyzc".to_vec(), Point::new(0, 1), Point::new(0, 4))
        );
    }

    #[test]
    fn simple_rust_tree_exposes_counts_ranges_and_fields() {
        let mut parser = SourceLanguage::Rust.parser().unwrap();
        let tree = parser.parse("fn f() {}", None).unwrap();
        assert_eq!(
            inspect(&tree),
            SyntaxCounts {
                nodes: 10,
                named: 5,
                errors: 0,
                missing: 0
            }
        );
        let nodes = shape(&tree);
        assert_eq!(
            nodes[0],
            NodeShape {
                kind: "source_file".into(),
                range: 0..9,
                depth: 0,
                field: None,
                named: true,
                error: false,
                missing: false
            }
        );
        assert_eq!(
            nodes[3],
            NodeShape {
                kind: "identifier".into(),
                range: 3..4,
                depth: 2,
                field: Some("name".into()),
                named: true,
                error: false,
                missing: false
            }
        );
    }

    #[test]
    fn incremental_edits_match_fresh_parses_and_preserve_previous_trees() {
        for (language, source, prefix) in [
            (SourceLanguage::Rust, "fn f() {}", "// note\n"),
            (
                SourceLanguage::Python,
                "def f():\n    return 1\n",
                "# note\n",
            ),
            (SourceLanguage::C, "int f(void) { return 1; }", "// note\n"),
            (SourceLanguage::Cpp, "int f() { return 1; }", "// note\n"),
        ] {
            let mut parser = language.parser().unwrap();
            let tree = parser.parse(source, None).unwrap();
            let before = shape(&tree);
            let prepared = prepare_edit(source.as_bytes(), 0, 0, prefix.as_bytes()).unwrap();
            let updated = reparse(&mut parser, &tree, &prepared).unwrap();
            let fresh = parser.parse(&prepared.source, None).unwrap();
            assert_eq!(shape(&updated), shape(&fresh));
            assert_eq!(shape(&tree), before);
            assert!(!inspect(&updated).nodes.eq(&0));
        }
    }

    #[test]
    fn error_recovery_is_reported_instead_of_hidden() {
        let mut parser = SourceLanguage::Rust.parser().unwrap();
        let tree = parser.parse("fn f() {", None).unwrap();
        assert!(inspect(&tree).missing > 0 || inspect(&tree).errors > 0);
        assert!(shape(&tree).iter().any(|node| node.missing || node.error));
    }

    #[test]
    fn grammars_parse_their_own_language_constructs() {
        for (language, source, root_kind) in [
            (SourceLanguage::Rust, "fn f() {}", "source_file"),
            (SourceLanguage::Python, "def f():\n    return 1\n", "module"),
            (
                SourceLanguage::C,
                "int f(int x) { return _Generic(x, int: 1, default: 0); }",
                "translation_unit",
            ),
            (
                SourceLanguage::Cpp,
                "namespace n { int f() { return 1; } }",
                "translation_unit",
            ),
        ] {
            let tree = language.parser().unwrap().parse(source, None).unwrap();
            assert_eq!(tree.root_node().kind(), root_kind);
            assert!(
                !tree.root_node().has_error(),
                "{}",
                tree.root_node().to_sexp()
            );
        }
    }

    #[test]
    fn error_nodes_are_counted_separately_from_missing_nodes() {
        let tree = SourceLanguage::Rust
            .parser()
            .unwrap()
            .parse("@", None)
            .unwrap();
        assert_eq!(inspect(&tree).errors, 1);
        assert!(shape(&tree).iter().any(|node| node.error));
    }

    #[test]
    fn deleting_across_a_newline_preserves_the_prefix_and_end_position() {
        let prepared = prepare_edit(b"a\nb", 1, 3, b"").unwrap();
        assert_eq!(
            (
                prepared.source,
                prepared.edit.old_end_position,
                prepared.edit.new_end_position
            ),
            (b"a".to_vec(), Point::new(1, 1), Point::new(0, 1))
        );
    }

    #[test]
    fn inserted_missing_tokens_are_counted_and_described() {
        let tree = SourceLanguage::C
            .parser()
            .unwrap()
            .parse("int x", None)
            .unwrap();
        assert_eq!(inspect(&tree).missing, 1, "{}", tree.root_node().to_sexp());
        assert!(shape(&tree).iter().any(|node| node.missing));
    }
}
