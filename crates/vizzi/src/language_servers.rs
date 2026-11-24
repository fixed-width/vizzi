//! Language-aware semantic routing with one shared dependency-source catalog.
use crate::{
    definitions::{Answer, Provider},
    dependency_sources::Catalog,
};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use vizzi_core::{
    analysis::ECMASCRIPT_LANGUAGES,
    model::Snapshot,
    structure::{CallLookup, Symbol},
};
mod c_family;
mod gradle;
mod inputs;
mod java;
mod project;
mod python;
mod ruby;
mod typescript;

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
    /// Optional TypeScript compiler executable with the native LSP.
    pub typescript_server: Option<PathBuf>,
    /// Optional Eclipse JDT LS launcher.
    pub jdtls: Option<PathBuf>,
    /// Java source roots relative to the workspace, defaulting to the workspace root.
    #[serde(rename = "java-source-root")]
    pub java_source_roots: Vec<String>,
    /// Local binary JARs on the managed Java classpath.
    #[serde(rename = "java-library")]
    pub java_libraries: Vec<PathBuf>,
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
                || self.typescript_server.is_some()
                || self.jdtls.is_some()
                || !self.java_source_roots.is_empty()
                || !self.java_libraries.is_empty()
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
        if !self.java_source_roots.is_empty() {
            if self.jdtls.is_none() {
                return Err("--java-source-root requires --jdtls".into());
            }
            java::source_roots(&self.java_source_roots)?;
        }
        if !self.java_libraries.is_empty() && self.jdtls.is_none() {
            return Err("--java-library requires --jdtls".into());
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
            && self.typescript_server.is_none()
            && self.jdtls.is_none()
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
        let mut servers = Self {
            sources: Arc::new(Catalog::new(cache.join("dependency-references.json"))),
            providers: BTreeMap::new(),
        };
        if let Some(connection) = configuration.vscode_connection {
            let provider: Arc<dyn Provider> = Arc::new(crate::editor_bridge::Bridge::new(
                root,
                connection,
                servers.sources.clone(),
            ));
            for language in ["rust", "python", "ruby", "c", "cpp", "java"]
                .into_iter()
                .chain(ECMASCRIPT_LANGUAGES.iter().copied())
            {
                servers.providers.insert(language, provider.clone());
            }
            return Ok(servers);
        }
        if let Some(executable) = configuration.rust_analyzer {
            servers.providers.insert(
                "rust",
                Arc::new(crate::rust_definitions::RustAnalyzer::with_sources(
                    root.clone(),
                    cache.join("rust"),
                    executable,
                    servers.sources.clone(),
                )),
            );
        }
        if let Some(executable) = configuration.pyright {
            servers.add_project(
                root.clone(),
                cache.join("python"),
                executable,
                python::Pyright {
                    interpreter: configuration.python_path,
                },
            );
        }
        if let Some(executable) = configuration.ruby_lsp {
            servers.add_project(
                root.clone(),
                cache.join("ruby"),
                executable.clone(),
                ruby::RubyLsp { executable },
            );
        }
        if let Some(executable) = configuration.clangd {
            servers.add_project(
                root.clone(),
                cache.join("clangd"),
                executable,
                c_family::Clangd {
                    database: configuration.compilation_database,
                },
            );
        }
        if let Some(executable) = configuration.jdtls {
            servers.add_project(
                root.clone(),
                cache.join("java"),
                executable,
                java::JdtLs {
                    roots: java::source_roots(&configuration.java_source_roots)?,
                    libraries: configuration.java_libraries,
                },
            );
        }
        if let Some(executable) = configuration.typescript_server {
            servers.add_project(
                root,
                cache.join("typescript"),
                executable,
                typescript::TypeScript,
            );
        }
        Ok(servers)
    }
    fn add_project(
        &mut self,
        root: PathBuf,
        cache: PathBuf,
        executable: PathBuf,
        profile: impl project::Profile + 'static,
    ) {
        let languages = profile.languages();
        let provider: Arc<dyn Provider> = Arc::new(project::ProjectServer::new(
            root,
            cache,
            executable,
            profile,
            self.sources.clone(),
        ));
        for language in languages {
            self.providers.insert(language, provider.clone());
        }
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
    fn supports_java_project_import(&self) -> bool {
        self.provider("java")
            .is_ok_and(Provider::supports_java_project_import)
    }
    fn java_project_kind(&self, snapshot: &Snapshot) -> Option<&'static str> {
        self.provider("java").ok()?.java_project_kind(snapshot)
    }
    fn import_java_project(
        &self,
        snapshot: Arc<Snapshot>,
        current: &dyn Fn() -> Option<Arc<Snapshot>>,
    ) -> Result<(), String> {
        self.provider("java")?
            .import_java_project(snapshot, current)
    }
}
