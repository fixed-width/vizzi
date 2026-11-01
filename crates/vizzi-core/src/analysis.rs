//! Language selection and worker-owned analysis adapters, without filesystem access.

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

/// An immutable factory and source-selection policy shared by native workers.
pub trait LanguageAdapter: Send + Sync {
    /// Exact, case-sensitive extensions without dots. Ambiguous extensions are rejected.
    fn extensions(&self) -> &[&str];
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

/// A source policy shared by discovery, invalidation, and analysis dispatch.
pub struct Languages {
    extensions: BTreeMap<String, Arc<Language>>,
}
impl Languages {
    /// Registers adapters, rejecting invalid or conflicting extension claims and contexts.
    pub fn new(adapters: Vec<Arc<dyn LanguageAdapter>>) -> Result<Self, ModelError> {
        let mut extensions = BTreeMap::new();
        let mut contexts = std::collections::BTreeSet::new();
        for adapter in adapters {
            let context = adapter.context();
            if !contexts.insert(context.id.clone()) {
                return Err(ModelError("duplicate language context".into()));
            }
            let language = Arc::new(Language { context, adapter });
            if language.adapter.extensions().is_empty() {
                return Err(ModelError("adapter has no source extensions".into()));
            }
            for extension in language.adapter.extensions() {
                if extension.is_empty()
                    || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
                {
                    return Err(ModelError("invalid source extension".into()));
                }
                if extensions
                    .insert((*extension).into(), language.clone())
                    .is_some()
                {
                    return Err(ModelError(format!(
                        "conflicting source extension: {extension}"
                    )));
                }
            }
        }
        Ok(Self { extensions })
    }
    /// Configures raw Rust and Python syntax, without guessing C/C++ or CUDA dialects.
    pub fn builtin() -> Result<Self, ModelError> {
        Self::new(vec![Arc::new(RustAdapter), Arc::new(PythonAdapter)])
    }
    /// Selects an adapter using an exact extension; unknown inputs have no analysis claim.
    pub fn select(&self, extension: &str) -> Option<&Language> {
        self.extensions.get(extension).map(Arc::as_ref)
    }
}
