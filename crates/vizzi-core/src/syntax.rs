//! Shared raw-syntax traversal and declaration identity construction.

use crate::{
    analysis::{AnalysisResult, Analyzer, SyntaxState},
    calls::{CallExpression, CallFacts},
    coloring::{Coloring, INDEX_BYTES, Style},
    model::{Analysis, ContentId, Context, Coverage, Fact, ModelError},
    syntax_state::edit_between,
};
use std::{collections::BTreeMap, ops::ControlFlow};
use tree_sitter::StreamingIterator;

pub(crate) struct Checkpoints<'a> {
    cancelled: &'a dyn Fn() -> bool,
    ticks: u8,
}
impl<'a> Checkpoints<'a> {
    pub(crate) fn new(cancelled: &'a dyn Fn() -> bool) -> Self {
        Self {
            cancelled,
            ticks: 0,
        }
    }
    pub(crate) fn poll(&mut self) -> bool {
        self.ticks = self.ticks.wrapping_add(1);
        self.ticks == 0 && (self.cancelled)()
    }
}

pub(crate) struct Declaration {
    pub declaration_span: Option<std::ops::Range<usize>>,
    pub identity: Option<String>,
    pub name_span: Option<std::ops::Range<usize>>,
    pub definition_anchor: Option<usize>,
    pub kind: &'static str,
    pub name: String,
    pub span: std::ops::Range<usize>,
    pub owns_calls: bool,
}
impl Declaration {
    pub(crate) fn at(node: tree_sitter::Node<'_>, kind: &'static str, name: String) -> Self {
        Self {
            declaration_span: (kind == "function").then(|| node.byte_range()),
            identity: None,
            definition_anchor: None,
            name_span: node
                .child_by_field_name("name")
                .map(|name| name.byte_range()),
            kind,
            name,
            span: node.byte_range(),
            owns_calls: true,
        }
    }
}

pub(crate) struct ScopeFact {
    pub ordinal: usize,
    pub owns_calls: bool,
    identity: Option<String>,
}

pub(crate) type ScopeFacts = BTreeMap<usize, ScopeFact>;

type Collector = fn(tree_sitter::Node<'_>, &str, &mut Vec<Declaration>);
type Scope = fn(tree_sitter::Node<'_>, &str) -> Option<(&'static str, String)>;
type EntryPoint = fn(tree_sitter::Node<'_>, &str) -> bool;

pub(crate) fn unparenthesized(mut node: tree_sitter::Node<'_>) -> tree_sitter::Node<'_> {
    while matches!(
        node.kind(),
        "parenthesized_expression" | "parenthesized_statements" | "parenthesized_declarator"
    ) {
        let mut cursor = node.walk();
        let mut contents = node
            .named_children(&mut cursor)
            .filter(|child| !child.is_extra());
        let Some(inner) = contents.next() else {
            break;
        };
        if contents.next().is_some() {
            break;
        }
        node = inner;
    }
    node
}
type FinishCalls = fn(
    tree_sitter::Node<'_>,
    &str,
    &Coverage,
    &ScopeFacts,
    &mut CallFacts,
    &dyn Fn() -> bool,
) -> bool;

#[derive(Clone, Copy, Default)]
pub(crate) struct CallRules {
    pub expressions: &'static [CallExpression],
    pub finish: Option<FinishCalls>,
}

impl CallRules {
    pub(crate) const fn supported(&self) -> bool {
        !self.expressions.is_empty() || self.finish.is_some()
    }
}

pub(crate) struct SyntaxAnalyzer {
    parser: tree_sitter::Parser,
    highlights: tree_sitter::Query,
    context: Context,
    declarations: Collector,
    scope: Scope,
    calls: CallRules,
    entry_point: Option<EntryPoint>,
}
impl SyntaxAnalyzer {
    pub(crate) fn new(
        language: tree_sitter::Language,
        highlights: &str,
        context: Context,
        declarations: Collector,
        scope: Scope,
        calls: CallRules,
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
            calls,
            entry_point: None,
        })
    }

    pub(crate) fn with_entry_point(mut self, entry_point: EntryPoint) -> Self {
        self.entry_point = Some(entry_point);
        self
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
            calls: None,
            coloring: None,
            content: ContentId::derive(&[bytes]),
            context: self.context.clone(),
            coverage: Coverage::Syntax,
            facts: Vec::new(),
            diagnostics: Vec::new(),
            has_entry_point: false,
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
        let mut calls = (!self.calls.expressions.is_empty()).then(CallFacts::default);
        let mut scope_facts = ScopeFacts::new();
        let mut checkpoints = Checkpoints {
            cancelled,
            ticks: 0,
        };
        loop {
            if checkpoints.poll() {
                return Ok(None);
            }
            let node = cursor.node();
            if !analysis.has_entry_point
                && self
                    .entry_point
                    .is_some_and(|recognize| recognize(node, text))
                && !node.has_error()
            {
                analysis.has_entry_point = true;
            }
            (self.declarations)(node, text, &mut declarations);
            if !declarations.is_empty() {
                let mut scope = Vec::new();
                let mut parent = node.parent();
                let mut parent_fact = None;
                while let Some(ancestor) = parent {
                    if checkpoints.poll() {
                        return Ok(None);
                    }
                    if let Some(fact) = scope_facts.get(&ancestor.id()) {
                        parent_fact.get_or_insert(fact.ordinal);
                        let declared = &analysis.facts[fact.ordinal];
                        let name = fact.identity.as_ref().unwrap_or(&declared.name);
                        scope.push(format!("{}:{name}", declared.kind));
                    } else if let Some((kind, name)) = (self.scope)(ancestor, text) {
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
                    let identity = declaration.identity.as_ref().unwrap_or(&declaration.name);
                    scope.push(format!("{}:{identity}", declaration.kind));
                    let key = serde_json::to_string(&scope)
                        .map_err(|error| ModelError(error.to_string()))?;
                    scope.pop();
                    if own_scope
                        .as_ref()
                        .is_some_and(|(kind, name)| *kind == declaration.kind && name == identity)
                    {
                        scope_facts.insert(
                            node.id(),
                            ScopeFact {
                                ordinal: analysis.facts.len(),
                                owns_calls: declaration.owns_calls,
                                identity: declaration.identity,
                            },
                        );
                    }
                    analysis.facts.push(Fact {
                        declaration_span: declaration.declaration_span,
                        name_span: declaration.name_span.filter(|range| {
                            text.get(range.clone()) == Some(declaration.name.as_str())
                        }),
                        definition_anchor: declaration.definition_anchor,
                        parent: parent_fact,
                        key,
                        kind: declaration.kind.into(),
                        name: declaration.name,
                        span: declaration.span,
                        unique: true,
                    });
                }
            }
            if let Some(facts) = &mut calls {
                for expression in self.calls.expressions {
                    expression.collect(node, text, &scope_facts, facts);
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
                    analysis.calls = calls;
                    if let Some(finish) = self.calls.finish
                        && !finish(
                            tree.root_node(),
                            text,
                            &analysis.coverage,
                            &scope_facts,
                            analysis.calls.get_or_insert_with(CallFacts::default),
                            cancelled,
                        )
                    {
                        return Ok(None);
                    }
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
    fn registered_syntax_can_extract_written_calls_under_a_new_language_name() {
        use crate::analysis::{LanguageAdapter, Languages};
        use std::sync::Arc;

        const CALLS: CallRules = CallRules {
            expressions: &[CallExpression {
                kind: "call",
                callee_field: "function",
                receiver_field: None,
                unwrap_callee: None,
                name_kinds: &["identifier"],
                name_fields: &[("attribute", "attribute")],
            }],
            finish: None,
        };

        struct FixtureAdapter {
            name: &'static str,
            calls: CallRules,
        }

        impl LanguageAdapter for FixtureAdapter {
            fn context(&self) -> Context {
                Context {
                    language: self.name.into(),
                    description: "Test syntax adapter".into(),
                    id: crate::model::ContextId::derive(&[self.name.as_bytes()]),
                }
            }

            fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
                Ok(Box::new(SyntaxAnalyzer::new(
                    tree_sitter_python::LANGUAGE.into(),
                    tree_sitter_python::HIGHLIGHTS_QUERY,
                    self.context(),
                    |_, _, _| {},
                    no_scope,
                    self.calls,
                )?))
            }

            fn supports_written_calls(&self) -> bool {
                self.calls.supported()
            }
        }

        let languages = Languages::new(vec![
            Arc::new(FixtureAdapter {
                name: "outline",
                calls: CallRules::default(),
            }),
            Arc::new(FixtureAdapter {
                name: "fixture",
                calls: CALLS,
            }),
        ])
        .unwrap();
        assert_eq!(languages.call_languages(), ["fixture"]);
        let analyze = |language| {
            languages
                .select(language)
                .unwrap()
                .create()
                .unwrap()
                .analyze(b"receiver.run()")
                .unwrap()
        };
        assert!(analyze("outline").calls.is_none());
        let calls = analyze("fixture")
            .calls
            .expect("fixture supports written calls");
        assert_eq!(calls.sites.len(), 1);
        assert_eq!(calls.sites[0].name.as_deref(), Some("run"));
    }

    #[test]
    fn an_expression_missing_the_adapters_callee_field_counts_as_omitted() {
        use crate::analysis::LanguageAdapter;

        let mut analyzer = SyntaxAnalyzer::new(
            tree_sitter_ruby::LANGUAGE.into(),
            include_str!("ruby-highlights.scm"),
            crate::ruby::RubyAdapter.context(),
            |_, _, _| {},
            no_scope,
            CallRules {
                expressions: &[CallExpression {
                    kind: "call",
                    callee_field: "method",
                    receiver_field: None,
                    unwrap_callee: None,
                    name_kinds: &["identifier"],
                    name_fields: &[],
                }],
                finish: None,
            },
        )
        .unwrap();
        let written_calls = analyzer.analyze(b"receiver.()").unwrap().calls.unwrap();
        assert_eq!(
            written_calls,
            CallFacts {
                sites: Vec::new(),
                omitted: 1,
            }
        );
    }

    #[test]
    fn parser_progress_interrupts_before_declaration_collection() {
        let mut analyzer = SyntaxAnalyzer::new(
            tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            crate::rust::RustAnalyzer::context(),
            counting,
            no_scope,
            CallRules::default(),
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
            CallRules::default(),
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
            CallRules::default(),
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
