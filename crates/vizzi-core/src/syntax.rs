//! Shared raw-syntax traversal and declaration identity construction.

use crate::{
    analysis::{AnalysisResult, Analyzer, SyntaxState},
    coloring::{Coloring, INDEX_BYTES, Style},
    model::{Analysis, ContentId, Context, Coverage, Fact, ModelError},
    syntax_state::edit_between,
};
use std::{collections::BTreeMap, ops::ControlFlow};
use tree_sitter::StreamingIterator;

struct Checkpoints<'a> {
    cancelled: &'a dyn Fn() -> bool,
    ticks: u8,
}
impl Checkpoints<'_> {
    fn poll(&mut self) -> bool {
        self.ticks = self.ticks.wrapping_add(1);
        self.ticks == 0 && (self.cancelled)()
    }
}

pub(crate) struct Declaration {
    pub kind: &'static str,
    pub name: String,
    pub span: std::ops::Range<usize>,
}
impl Declaration {
    pub(crate) fn at(node: tree_sitter::Node<'_>, kind: &'static str, name: String) -> Self {
        Self {
            kind,
            name,
            span: node.byte_range(),
        }
    }
}

type Collector = fn(tree_sitter::Node<'_>, &str, &mut Vec<Declaration>);
type Scope = fn(tree_sitter::Node<'_>, &str) -> Option<(&'static str, String)>;

pub(crate) struct SyntaxAnalyzer {
    parser: tree_sitter::Parser,
    highlights: tree_sitter::Query,
    context: Context,
    declarations: Collector,
    scope: Scope,
}
impl SyntaxAnalyzer {
    pub(crate) fn new(
        language: tree_sitter::Language,
        highlights: &str,
        context: Context,
        declarations: Collector,
        scope: Scope,
    ) -> Result<Self, ModelError> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language)
            .map_err(|error| ModelError(error.to_string()))?;
        Ok(Self {
            highlights: tree_sitter::Query::new(&language, highlights)
                .map_err(|e| ModelError(e.to_string()))?,
            parser,
            context,
            declarations,
            scope,
        })
    }
}
impl Analyzer for SyntaxAnalyzer {
    /// Extracts source-backed declarations and reports syntax recovery.
    fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError> {
        self.analyze_cancellable(bytes, &|| false)?
            .ok_or_else(|| ModelError("uncancelled analysis was interrupted".into()))
    }

    fn analyze_cancellable(
        &mut self,
        bytes: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Analysis>, ModelError> {
        Ok(self
            .run(bytes, None, 0, cancelled)?
            .map(|result| result.analysis))
    }
    fn analyze_with_state(
        &mut self,
        bytes: &[u8],
        previous: Option<SyntaxState>,
        retain_weight: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<AnalysisResult>, ModelError> {
        self.run(bytes, previous, retain_weight, cancelled)
    }
}
impl SyntaxAnalyzer {
    fn color(
        &self,
        tree: &tree_sitter::Tree,
        bytes: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Option<Coloring> {
        let mut cursor = tree_sitter::QueryCursor::new();
        let stopped = std::cell::Cell::new(false);
        let mut progress = |_: &tree_sitter::QueryCursorState| {
            if cancelled() {
                stopped.set(true);
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        };
        let mut captures = cursor.captures_with_options(
            &self.highlights,
            tree.root_node(),
            bytes,
            tree_sitter::QueryCursorOptions::new().progress_callback(&mut progress),
        );
        let mut checkpoints = Checkpoints {
            cancelled,
            ticks: 0,
        };
        let index = Coloring::from_spans(
            std::iter::from_fn(|| {
                while let Some((matched, position)) = captures.next() {
                    if checkpoints.poll() {
                        stopped.set(true);
                        return None;
                    }
                    let capture = matched.captures()[*position];
                    if let Some(mut kind) =
                        Style::capture(self.highlights.capture_names()[capture.index as usize])
                    {
                        if matches!(capture.node.kind(), "integer_literal" | "float_literal") {
                            kind = Style::Number;
                        }
                        return Some((capture.node.byte_range(), kind));
                    }
                }
                None
            }),
            INDEX_BYTES,
        );
        drop(captures);
        if stopped.get() || cancelled() {
            None
        } else {
            Some(if cursor.did_exceed_match_limit() {
                Coloring::limited()
            } else {
                index
            })
        }
    }
    fn run(
        &mut self,
        bytes: &[u8],
        previous: Option<SyntaxState>,
        retain_weight: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<AnalysisResult>, ModelError> {
        if cancelled() {
            return Ok(None);
        }
        let mut analysis = Analysis {
            coloring: None,
            content: ContentId::derive(&[bytes]),
            context: self.context.clone(),
            coverage: Coverage::Syntax,
            facts: Vec::new(),
            diagnostics: Vec::new(),
        };
        let Ok(text) = std::str::from_utf8(bytes) else {
            analysis.coverage = Coverage::Unavailable;
            analysis
                .diagnostics
                .push("Source is not UTF-8; syntax analysis is unavailable".into());
            return Ok((!cancelled()).then_some(AnalysisResult {
                analysis,
                state: None,
                tree_reused: false,
            }));
        };
        let mut stopped = false;
        let mut progress = |_: &tree_sitter::ParseState| {
            if cancelled() {
                stopped = true;
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        };
        let mut previous = previous.filter(|state| state.context() == &self.context);
        if let Some(state) = &mut previous {
            let Some(edit) = edit_between(&state.source, text, cancelled) else {
                return Ok(None);
            };
            state.tree.edit(&edit);
        }
        if cancelled() {
            return Ok(None);
        }
        let tree_reused = previous.is_some();
        let tree = self.parser.parse_with_options(
            &mut |offset, _| bytes.get(offset..).unwrap_or_default(),
            previous.as_ref().map(|state| &state.tree),
            Some(tree_sitter::ParseOptions::new().progress_callback(&mut progress)),
        );
        drop(previous);
        let Some(tree) = tree else {
            // Interrupted parsers otherwise resume the abandoned input on the next call.
            self.parser.reset();
            return if stopped {
                Ok(None)
            } else {
                Err(ModelError("parser did not return a tree".into()))
            };
        };
        if cancelled() {
            return Ok(None);
        }
        if tree.root_node().has_error() {
            analysis.coverage = Coverage::Partial;
            analysis.diagnostics.push(
                "Raw source required grammar recovery; declarations may be incomplete".into(),
            );
        }
        let mut cursor = tree.walk();
        let mut declarations = Vec::new();
        let mut scope_facts = BTreeMap::<usize, usize>::new();
        let mut checkpoints = Checkpoints {
            cancelled,
            ticks: 0,
        };
        loop {
            if checkpoints.poll() {
                return Ok(None);
            }
            let node = cursor.node();
            (self.declarations)(node, text, &mut declarations);
            if !declarations.is_empty() {
                let mut scope = Vec::new();
                let mut parent = node.parent();
                let mut parent_fact = None;
                while let Some(ancestor) = parent {
                    if checkpoints.poll() {
                        return Ok(None);
                    }
                    if parent_fact.is_none() {
                        parent_fact = scope_facts.get(&ancestor.id()).copied();
                    }
                    if let Some((kind, name)) = (self.scope)(ancestor, text) {
                        scope.push(format!("{kind}:{name}"));
                    }
                    parent = ancestor.parent();
                }
                scope.reverse();
                let own_scope = (self.scope)(node, text);
                for declaration in declarations.drain(..) {
                    if checkpoints.poll() {
                        return Ok(None);
                    }
                    scope.push(format!("{}:{}", declaration.kind, declaration.name));
                    let key = serde_json::to_string(&scope)
                        .map_err(|error| ModelError(error.to_string()))?;
                    scope.pop();
                    if own_scope.as_ref().is_some_and(|(kind, name)| {
                        *kind == declaration.kind && name == &declaration.name
                    }) {
                        scope_facts.insert(node.id(), analysis.facts.len());
                    }
                    analysis.facts.push(Fact {
                        parent: parent_fact,
                        key,
                        kind: declaration.kind.into(),
                        name: declaration.name,
                        span: declaration.span,
                        unique: true,
                    });
                }
            }
            if cursor.goto_first_child() {
                continue;
            }
            loop {
                if checkpoints.poll() {
                    return Ok(None);
                }
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    let mut counts = BTreeMap::<String, usize>::new();
                    for fact in &analysis.facts {
                        if checkpoints.poll() {
                            return Ok(None);
                        }
                        *counts.entry(fact.key.clone()).or_default() += 1;
                    }
                    let mut ordinals = BTreeMap::<String, usize>::new();
                    for fact in &mut analysis.facts {
                        if checkpoints.poll() {
                            return Ok(None);
                        }
                        fact.unique = counts[&fact.key] == 1;
                        let ordinal = ordinals.entry(fact.key.clone()).or_default();
                        let key = serde_json::to_string(&(&fact.key, *ordinal))
                            .map_err(|error| ModelError(error.to_string()))?;
                        *ordinal += 1;
                        fact.key = key;
                    }
                    if cancelled() {
                        return Ok(None);
                    }
                    drop(cursor);
                    analysis.coloring = self.color(&tree, bytes, cancelled);
                    if analysis.coloring.is_none() || cancelled() {
                        return Ok(None);
                    }
                    let state = SyntaxState::capture(
                        self.context.clone(),
                        analysis.content.clone(),
                        text,
                        tree,
                        retain_weight,
                    );
                    return Ok(Some(AnalysisResult {
                        analysis,
                        state,
                        tree_reused,
                    }));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! { static VISITED: Cell<usize> = const { Cell::new(0) }; }

    fn counting(_: tree_sitter::Node<'_>, _: &str, _: &mut Vec<Declaration>) {
        VISITED.set(VISITED.get() + 1);
    }
    fn no_scope(_: tree_sitter::Node<'_>, _: &str) -> Option<(&'static str, String)> {
        None
    }

    #[test]
    fn parser_progress_interrupts_before_declaration_collection() {
        let mut analyzer = SyntaxAnalyzer::new(
            tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            crate::rust::RustAnalyzer::context(),
            counting,
            no_scope,
        )
        .unwrap();
        VISITED.set(0);
        let calls = Cell::new(0);
        let source = "fn example() { let x = 12; }\n".repeat(10000);
        assert!(
            analyzer
                .analyze_cancellable(source.as_bytes(), &|| {
                    calls.set(calls.get() + 1);
                    calls.get() >= 20
                })
                .unwrap()
                .is_none()
        );
        assert_eq!(VISITED.get(), 0);
        assert!(
            calls.get() < 25,
            "the parser kept polling after cancellation"
        );
    }

    #[test]
    fn extraction_abandons_traversal_without_visiting_the_remaining_tree() {
        let mut analyzer = SyntaxAnalyzer::new(
            tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            crate::rust::RustAnalyzer::context(),
            counting,
            no_scope,
        )
        .unwrap();
        VISITED.set(0);
        let source = "fn example() { let x = 12; }\n".repeat(1000);
        assert!(
            analyzer
                .analyze_cancellable(source.as_bytes(), &|| VISITED.get() >= 100)
                .unwrap()
                .is_none()
        );
        assert!(
            (100..=356).contains(&VISITED.get()),
            "extraction continued after cancellation"
        );
        assert!(
            analyzer
                .analyze_cancellable(b"fn different() {}", &|| false)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn extraction_amortises_predicate_calls_across_many_node_visits() {
        let mut analyzer = SyntaxAnalyzer::new(
            tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            crate::rust::RustAnalyzer::context(),
            counting,
            no_scope,
        )
        .unwrap();
        VISITED.set(0);
        let polls = Cell::new(0);
        let source = "fn example() { let x = 12; }\n".repeat(1000);
        assert!(
            analyzer
                .analyze_cancellable(source.as_bytes(), &|| {
                    if VISITED.get() > 0 {
                        polls.set(polls.get() + 1);
                    }
                    false
                })
                .unwrap()
                .is_some()
        );
        assert!(VISITED.get() > 1000);
        assert!(
            polls.get() < VISITED.get() / 16,
            "cancellation polling grew with every node visit"
        );
    }
}
