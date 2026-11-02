//! Worker-owned analysis adapters, independent of source selection and filesystem access.

pub use crate::syntax_state::{AnalysisResult, SyntaxState};
use crate::{
    c_family::{CAdapter, CppAdapter},
    model::{Analysis, Context, ModelError},
    python::PythonAdapter,
    rust::RustAdapter,
};
use std::{collections::BTreeMap, sync::Arc};

/// A worker-owned analyzer of captured bytes. It must not execute project tooling.
pub trait Analyzer {
    /// Returns facts for exactly these bytes and the adapter's registered context.
    fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError>;

    /// Returns complete facts, or `None` when this attempt is cancelled.
    /// The predicate must be cheap, nonblocking, nonpanicking, and remain true once set.
    /// The default checks before/after analysis; adapters may also interrupt their work.
    fn analyze_cancellable(
        &mut self,
        bytes: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Analysis>, ModelError> {
        if cancelled() {
            return Ok(None);
        }
        let analysis = self.analyze(bytes)?;
        Ok((!cancelled()).then_some(analysis))
    }

    /// Consumes optional previous syntax state and returns complete new facts.
    /// `retain_weight` limits returned state; zero permits reuse without retention.
    /// Existing adapters use cancellable analysis and return no retained state.
    fn analyze_with_state(
        &mut self,
        bytes: &[u8],
        _previous: Option<SyntaxState>,
        _retain_weight: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<AnalysisResult>, ModelError> {
        Ok(self
            .analyze_cancellable(bytes, cancelled)?
            .map(|analysis| AnalysisResult {
                analysis,
                state: None,
                tree_reused: false,
            }))
    }
}

/// An immutable context and analyzer factory shared by native workers.
pub trait LanguageAdapter: Send + Sync {
    /// Identifies language, configuration, grammar, and implementation; stable per registration.
    fn context(&self) -> Context;
    /// Creates one analyzer owned exclusively by its calling worker.
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError>;
}

/// A registered adapter with a frozen context used for cache validity.
pub struct Language {
    context: Context,
    adapter: Arc<dyn LanguageAdapter>,
}
impl Language {
    /// Returns the context captured during registration.
    pub fn context(&self) -> &Context {
        &self.context
    }
    /// Creates a worker-local analyzer. The engine verifies its output context.
    pub fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        self.adapter.create()
    }
}

/// Registered analyzer factories, independent of source selection.
pub struct Languages {
    languages: BTreeMap<String, Arc<Language>>,
}
impl Languages {
    /// Registers adapters, rejecting empty/duplicate language names and context identities.
    pub fn new(adapters: Vec<Arc<dyn LanguageAdapter>>) -> Result<Self, ModelError> {
        let mut languages = BTreeMap::new();
        let mut contexts = std::collections::BTreeSet::new();
        for adapter in adapters {
            let context = adapter.context();
            if context.language.is_empty() || !contexts.insert(context.id.clone()) {
                return Err(ModelError(
                    "empty language or duplicate analysis context".into(),
                ));
            }
            let key = context.language.clone();
            let language = Arc::new(Language { context, adapter });
            if languages.insert(key, language).is_some() {
                return Err(ModelError("duplicate analyzer language".into()));
            }
        }
        Ok(Self { languages })
    }
    /// Registers raw Rust, Python, C, and C++ factories; source policy selects inputs.
    pub fn builtin() -> Result<Self, ModelError> {
        Self::new(vec![
            Arc::new(RustAdapter),
            Arc::new(PythonAdapter),
            Arc::new(CAdapter),
            Arc::new(CppAdapter),
        ])
    }
    /// Looks up a registered analyzer language.
    pub fn select(&self, language: &str) -> Option<&Language> {
        self.languages.get(language).map(Arc::as_ref)
    }
    /// Lists registered language identifiers for source-policy construction.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.languages.keys().map(String::as_str)
    }
}
