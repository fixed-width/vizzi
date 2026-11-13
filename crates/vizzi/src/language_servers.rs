//! Language-aware semantic routing with one shared dependency-source catalog.
use crate::{
    definitions::{Answer, Provider},
    dependency_sources::Catalog,
};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use vizzi_core::{
    model::Snapshot,
    structure::{CallLookup, Symbol},
};
mod inputs;
mod project;
mod ruby;

/// Explicit native tool configuration; no executable is accepted from browser requests.
#[derive(Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Configuration {
    /// VS Code plugin connection file; mutually exclusive with managed servers.
    pub vscode_connection: Option<PathBuf>,
    /// Optional Rust language-server executable.
    pub rust_analyzer: Option<PathBuf>,
    /// Optional Pyright-compatible stdio language-server executable.
    pub pyright: Option<PathBuf>,
    /// Optional clangd executable shared by C and C++.
    pub clangd: Option<PathBuf>,
    /// Optional Ruby LSP executable from the selected local Ruby environment.
    pub ruby_lsp: Option<PathBuf>,
    /// Optional interpreter/environment used by the Python server.
    pub python_path: Option<PathBuf>,
    /// Optional compilation database, otherwise root/compile_commands.json.
    #[serde(rename = "compile-commands")]
    pub compilation_database: Option<PathBuf>,
}
impl Configuration {
    /// Validates language-server ownership and dependent project options.
    pub fn validate(&self) -> Result<(), String> {
        if self.vscode_connection.is_some()
            && (self.rust_analyzer.is_some()
                || self.pyright.is_some()
                || self.ruby_lsp.is_some()
                || self.clangd.is_some()
                || self.python_path.is_some()
                || self.compilation_database.is_some())
        {
            return Err("--vscode-connection cannot be combined with Vizzi-managed language servers or their project options".into());
        }
        if self.python_path.is_some() && self.pyright.is_none() {
            return Err("--python-path requires --pyright".into());
        }
        if self.compilation_database.is_some() && self.clangd.is_none() {
            return Err("--compile-commands requires --clangd".into());
        }
        Ok(())
    }
    /// Whether no semantic language server has been selected.
    pub fn is_empty(&self) -> bool {
        self.vscode_connection.is_none()
            && self.rust_analyzer.is_none()
            && self.pyright.is_none()
            && self.clangd.is_none()
            && self.ruby_lsp.is_none()
    }
}

/// Providers selected by source language, including dependency captures.
pub struct Servers {
    providers: BTreeMap<&'static str, Arc<dyn Provider>>,
    sources: Arc<Catalog>,
}
impl Servers {
    /// Configures lazy server sessions over a captured workspace.
    pub fn new(
        root: PathBuf,
        cache: PathBuf,
        configuration: Configuration,
    ) -> Result<Self, String> {
        configuration.validate()?;
        let sources = Arc::new(Catalog::new(cache.join("dependency-references.json")));
        let mut providers: BTreeMap<&'static str, Arc<dyn Provider>> = BTreeMap::new();
        if let Some(connection) = configuration.vscode_connection {
            let provider: Arc<dyn Provider> = Arc::new(crate::editor_bridge::Bridge::new(
                root,
                connection,
                sources.clone(),
            ));
            for language in ["rust", "python", "ruby", "c", "cpp"] {
                providers.insert(language, provider.clone());
            }
            return Ok(Self { providers, sources });
        }
        if let Some(executable) = configuration.rust_analyzer {
            providers.insert(
                "rust",
                Arc::new(crate::rust_definitions::RustAnalyzer::with_sources(
                    root.clone(),
                    cache.join("rust"),
                    executable,
                    sources.clone(),
                )),
            );
        }
        if let Some(executable) = configuration.pyright {
            providers.insert(
                "python",
                Arc::new(project::ProjectServer::new(
                    root.clone(),
                    cache.join("python"),
                    executable,
                    inputs::Profile::Python {
                        interpreter: configuration.python_path,
                    },
                    sources.clone(),
                )),
            );
        }
        if let Some(executable) = configuration.ruby_lsp {
            providers.insert(
                "ruby",
                Arc::new(project::ProjectServer::new(
                    root.clone(),
                    cache.join("ruby"),
                    executable.clone(),
                    inputs::Profile::Ruby { executable },
                    sources.clone(),
                )),
            );
        }
        if let Some(executable) = configuration.clangd {
            let provider: Arc<dyn Provider> = Arc::new(project::ProjectServer::new(
                root,
                cache.join("clangd"),
                executable,
                inputs::Profile::CFamily {
                    database: configuration.compilation_database,
                },
                sources.clone(),
            ));
            providers.insert("c", provider.clone());
            providers.insert("cpp", provider);
        }
        Ok(Self { providers, sources })
    }
    fn provider(&self, language: &str) -> Result<&dyn Provider, String> {
        self.providers
            .get(language)
            .map(Arc::as_ref)
            .ok_or_else(|| format!("No language server configured for {language}"))
    }
}
impl Provider for Servers {
    fn resolve(&self, snapshot: Arc<Snapshot>, call: &CallLookup) -> Result<Answer, String> {
        self.provider(&call.language)?.resolve(snapshot, call)
    }
    fn references(
        &self,
        snapshot: Arc<Snapshot>,
        symbol: &Symbol,
        identifier: usize,
    ) -> Result<Answer, String> {
        let external = symbol
            .origin
            .as_ref()
            .map(|origin| self.sources.get(&origin.capture))
            .transpose()?
            .flatten();
        let doc = external
            .as_ref()
            .map(|source| source.doc())
            .or_else(|| snapshot.file(&symbol.source.file).map(AsRef::as_ref))
            .ok_or("Reference source is unavailable")?;
        let language = doc
            .analyses()
            .first()
            .ok_or("Reference language is unavailable")?
            .context
            .language
            .as_str();
        self.provider(language)?
            .references(snapshot, symbol, identifier)
    }
    fn dependencies(&self) -> Option<&Catalog> {
        Some(&self.sources)
    }
}
