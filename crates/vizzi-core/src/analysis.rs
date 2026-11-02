//! Worker-owned analysis adapters, independent of source selection and filesystem access.

use crate::{
    model::{Analysis, Context, ModelError},
    python::PythonAdapter,
    rust::RustAdapter,
};
use std::{collections::BTreeMap, sync::Arc};

/// A worker-owned analyzer of captured bytes. It must not execute project tooling.
pub trait Analyzer {
    /// Returns facts for exactly these bytes and the adapter's registered context.
    fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError>;
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
    /// Configures raw Rust and Python syntax, without guessing C/C++ or CUDA dialects.
    pub fn builtin() -> Result<Self, ModelError> {
        Self::new(vec![Arc::new(RustAdapter), Arc::new(PythonAdapter)])
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
