//! Native launch settings: an explicit file layer followed by command-line overrides.
use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use vizzi::language_servers::Configuration;
use vizzi_core::policy::{Assignment, FamilyLanguage, PolicyConfig};

const FILE_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Options {
    pub root: Option<PathBuf>,
    pub cache: Option<PathBuf>,
    pub workers: usize,
    pub port: u16,
    pub syntax_cache_mib: usize,
    pub analysis_input_mib: usize,
    pub source_spool_mib: usize,
    pub investigation_file: Option<PathBuf>,
    pub editor_code: PathBuf,
    pub language_servers: Configuration,
    c_family_root: Vec<String>,
    exclude_source: Vec<String>,
    syntax_language: Vec<String>,
    #[serde(skip)]
    pub stdio: bool,
    #[serde(skip)]
    pub once: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            root: None,
            cache: None,
            workers: 4,
            port: 7474,
            syntax_cache_mib: 0,
            analysis_input_mib: 0,
            source_spool_mib: 0,
            investigation_file: None,
            editor_code: "code".into(),
            language_servers: Configuration::default(),
            c_family_root: vec![],
            exclude_source: vec![],
            syntax_language: vec![],
            stdio: false,
            once: false,
        }
    }
}
fn path(base: &Path, value: &mut PathBuf) {
    if value.is_relative() {
        *value = base.join(&*value);
    }
}
fn executable(base: &Path, value: &mut PathBuf) {
    if value.components().count() > 1 {
        path(base, value);
    }
}
impl Options {
    fn from_file(file: &Path) -> Result<Self, String> {
        let file = std::path::absolute(file).map_err(|e| e.to_string())?;
        let read = || -> Result<Self, Box<dyn std::error::Error>> {
            let input = File::open(&file)?;
            if !input.metadata()?.is_file() {
                return Err("configuration must be a regular file".into());
            }
            let mut text = String::new();
            input.take(FILE_BYTES + 1).read_to_string(&mut text)?;
            if text.len() as u64 > FILE_BYTES {
                return Err("configuration exceeds 1 MiB".into());
            }
            Ok(toml::from_str(&text)?)
        };
        let mut options =
            read().map_err(|e| format!("Cannot load config {}: {e}", file.display()))?;
        let base = file.parent().ok_or("Config file has no parent directory")?;
        for value in [
            &mut options.root,
            &mut options.cache,
            &mut options.investigation_file,
            &mut options.language_servers.compilation_database,
            &mut options.language_servers.vscode_connection,
        ]
        .into_iter()
        .flatten()
        {
            path(base, value);
        }
        for value in [
            &mut options.language_servers.rust_analyzer,
            &mut options.language_servers.pyright,
            &mut options.language_servers.ruby_lsp,
            &mut options.language_servers.clangd,
            &mut options.language_servers.python_path,
        ]
        .into_iter()
        .flatten()
        {
            executable(base, value);
        }
        executable(base, &mut options.editor_code);
        Ok(options)
    }
    pub fn policy(&self) -> Result<PolicyConfig, String> {
        let assignments = self
            .syntax_language
            .iter()
            .map(|rule| {
                let (scope, language) = rule
                    .rsplit_once('=')
                    .ok_or("--syntax-language needs SCOPE=c|cpp")?;
                let language = match language {
                    "c" => FamilyLanguage::C,
                    "cpp" => FamilyLanguage::Cpp,
                    _ => return Err("syntax language must be c or cpp".to_owned()),
                };
                Ok(Assignment {
                    scope: scope.into(),
                    language,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(PolicyConfig {
            c_family_roots: self.c_family_root.clone(),
            exclusions: self.exclude_source.clone(),
            assignments,
        })
    }
}

pub(super) fn load(arguments: impl Iterator<Item = String>) -> Result<Option<Options>, String> {
    let arguments: Vec<_> = arguments.collect();
    if arguments
        .iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h"))
    {
        return Ok(None);
    }
    let mut arguments = arguments.into_iter();
    let mut overrides = Vec::new();
    let mut config = None;
    while let Some(argument) = arguments.next() {
        if matches!(argument.as_str(), "--once" | "--stdio") {
            overrides.push((argument, String::new()));
            continue;
        }
        if !matches!(
            argument.as_str(),
            "--config"
                | "--root"
                | "--cache"
                | "--workers"
                | "--port"
                | "--syntax-cache-mib"
                | "--analysis-input-mib"
                | "--source-spool-mib"
                | "--investigation-file"
                | "--editor-code"
                | "--rust-analyzer"
                | "--pyright"
                | "--ruby-lsp"
                | "--clangd"
                | "--python-path"
                | "--compile-commands"
                | "--c-family-root"
                | "--exclude-source"
                | "--syntax-language"
                | "--vscode-connection"
        ) {
            return Err(format!("unknown argument: {argument}"));
        }
        let value = arguments
            .next()
            .ok_or_else(|| format!("{argument} needs a value"))?;
        if argument == "--config" {
            if config.replace(PathBuf::from(value)).is_some() {
                return Err("choose one --config file".into());
            }
        } else {
            overrides.push((argument, value));
        }
    }
    let mut options = match config {
        Some(file) => Options::from_file(&file)?,
        None => Options::default(),
    };
    for (argument, value) in overrides {
        let invalid = |e| format!("Invalid {argument}: {e}");
        match argument.as_str() {
            "--root" => options.root = Some(value.into()),
            "--cache" => options.cache = Some(value.into()),
            "--workers" => options.workers = value.parse().map_err(invalid)?,
            "--port" => options.port = value.parse().map_err(invalid)?,
            "--syntax-cache-mib" => options.syntax_cache_mib = value.parse().map_err(invalid)?,
            "--analysis-input-mib" => {
                options.analysis_input_mib = value.parse().map_err(invalid)?
            }
            "--source-spool-mib" => options.source_spool_mib = value.parse().map_err(invalid)?,
            "--investigation-file" => options.investigation_file = Some(value.into()),
            "--editor-code" => options.editor_code = value.into(),
            "--rust-analyzer" => options.language_servers.rust_analyzer = Some(value.into()),
            "--pyright" => options.language_servers.pyright = Some(value.into()),
            "--ruby-lsp" => options.language_servers.ruby_lsp = Some(value.into()),
            "--clangd" => options.language_servers.clangd = Some(value.into()),
            "--python-path" => options.language_servers.python_path = Some(value.into()),
            "--compile-commands" => {
                options.language_servers.compilation_database = Some(value.into())
            }
            "--vscode-connection" => {
                options.language_servers.vscode_connection = Some(value.into())
            }
            "--c-family-root" => options.c_family_root.push(value),
            "--exclude-source" => options.exclude_source.push(value),
            "--syntax-language" => options.syntax_language.push(value),
            "--once" => options.once = true,
            "--stdio" => options.stdio = true,
            _ => unreachable!("validated command-line option"),
        }
    }
    if options.once && options.stdio {
        return Err("choose either --once or --stdio".into());
    }
    if !(1..=32).contains(&options.workers) {
        return Err("--workers must be between 1 and 32".into());
    }
    for (name, value) in [
        ("syntax-cache-mib", options.syntax_cache_mib),
        ("analysis-input-mib", options.analysis_input_mib),
        ("source-spool-mib", options.source_spool_mib),
    ] {
        if value.checked_mul(1024 * 1024).is_none() {
            return Err(format!("--{name} exceeds byte capacity"));
        }
    }
    options.language_servers.validate()?;
    Ok(Some(options))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_args(arguments: &[&str]) -> Options {
        load(arguments.iter().map(|arg| (*arg).to_owned()))
            .unwrap()
            .unwrap()
    }

    #[test]
    fn file_paths_and_executables_have_distinct_resolution_rules() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("vizzi.toml");
        std::fs::write(
            &file,
            r#"
root = 'project'
cache = '../cache'
investigation-file = '../notes.json'
editor-code = './bin/code'
workers = 2
port = 8000
syntax-cache-mib = 8
analysis-input-mib = 16
source-spool-mib = 32
c-family-root = ['src']
exclude-source = ['vendor']
syntax-language = ['src=cpp']
[language-servers]
rust-analyzer = 'rust-analyzer'
pyright = './bin/pyright'
ruby-lsp = '/absolute/ruby-lsp'
python-path = './env/python'
clangd = 'clangd'
compile-commands = 'build/compile_commands.json'
"#,
        )
        .unwrap();
        let options = load_args(&["--config", file.to_str().unwrap()]);
        let base = std::path::absolute(temp.path()).unwrap();
        assert_eq!(options.root, Some(base.join("project")));
        assert_eq!(options.cache, Some(base.join("../cache")));
        assert_eq!(options.investigation_file, Some(base.join("../notes.json")));
        assert_eq!(options.editor_code, base.join("./bin/code"));
        assert_eq!(
            options.language_servers.rust_analyzer,
            Some("rust-analyzer".into())
        );
        assert_eq!(
            options.language_servers.pyright,
            Some(base.join("./bin/pyright"))
        );
        assert_eq!(
            options.language_servers.ruby_lsp,
            Some("/absolute/ruby-lsp".into())
        );
        assert_eq!(
            options.language_servers.python_path,
            Some(base.join("./env/python"))
        );
        assert_eq!(
            options.language_servers.compilation_database,
            Some(base.join("build/compile_commands.json"))
        );
        assert_eq!(options.workers, 2);
        assert_eq!(options.port, 8000);
        assert_eq!(
            (
                options.syntax_cache_mib,
                options.analysis_input_mib,
                options.source_spool_mib
            ),
            (8, 16, 32)
        );
        let policy = options.policy().unwrap();
        assert_eq!(policy.c_family_roots, ["src"]);
        assert_eq!(policy.exclusions, ["vendor"]);
        assert_eq!(policy.assignments[0].scope, "src");
        assert_eq!(policy.assignments[0].language, FamilyLanguage::Cpp);
    }

    #[test]
    fn cli_scalars_replace_file_values_and_paths_keep_cli_interpretation() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("vizzi.toml");
        std::fs::write(&file,"root = 'file-root'\nworkers = 2\nport = 8000\nsyntax-cache-mib = 10\nanalysis-input-mib = 10\nsource-spool-mib = 10\n").unwrap();
        let options = load_args(&[
            "--root",
            "cli-root",
            "--config",
            file.to_str().unwrap(),
            "--cache",
            "cli-cache",
            "--port",
            "0",
            "--workers",
            "32",
            "--syntax-cache-mib",
            "0",
            "--analysis-input-mib",
            "1",
            "--source-spool-mib",
            "2",
            "--editor-code",
            "./cli-code",
            "--investigation-file",
            "cli-notes.json",
            "--rust-analyzer",
            "./cli-rust",
            "--pyright",
            "pyright",
            "--python-path",
            "./python",
            "--ruby-lsp",
            "ruby-lsp",
            "--clangd",
            "clangd",
            "--compile-commands",
            "cli-db.json",
            "--c-family-root",
            "c",
            "--exclude-source",
            "ignored",
            "--syntax-language",
            "c=c",
            "--stdio",
        ]);
        assert_eq!(options.root, Some("cli-root".into()));
        assert_eq!(options.cache, Some("cli-cache".into()));
        assert_eq!(options.workers, 32);
        assert_eq!(options.port, 0);
        assert!(options.stdio);
        assert!(!options.once);
        assert_eq!(
            (
                options.syntax_cache_mib,
                options.analysis_input_mib,
                options.source_spool_mib
            ),
            (0, 1, 2)
        );
        assert_eq!(options.editor_code, PathBuf::from("./cli-code"));
        assert_eq!(options.investigation_file, Some("cli-notes.json".into()));
        assert_eq!(
            options.language_servers.rust_analyzer,
            Some("./cli-rust".into())
        );
        assert_eq!(
            options.language_servers.python_path,
            Some("./python".into())
        );
        assert_eq!(options.language_servers.pyright, Some("pyright".into()));
        assert_eq!(options.language_servers.ruby_lsp, Some("ruby-lsp".into()));
        assert_eq!(options.language_servers.clangd, Some("clangd".into()));
        assert_eq!(
            options.language_servers.compilation_database,
            Some("cli-db.json".into())
        );
        let policy = options.policy().unwrap();
        assert_eq!(policy.c_family_roots, ["c"]);
        assert_eq!(policy.exclusions, ["ignored"]);
        assert_eq!(policy.assignments[0].scope, "c");
        assert_eq!(policy.assignments[0].language, FamilyLanguage::C);
    }

    #[test]
    fn editor_paths_can_be_overridden_without_enabling_owned_servers() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("vizzi.toml");
        std::fs::write(
            &file,
            "[language-servers]\nvscode-connection = 'connection.json'",
        )
        .unwrap();
        let options = load_args(&["--config", file.to_str().unwrap()]);
        assert_eq!(
            options.language_servers.vscode_connection,
            Some(
                std::path::absolute(temp.path())
                    .unwrap()
                    .join("connection.json")
            )
        );
        let options = load_args(&[
            "--config",
            file.to_str().unwrap(),
            "--vscode-connection",
            "cli.json",
        ]);
        assert_eq!(
            options.language_servers.vscode_connection,
            Some("cli.json".into())
        );
        assert!(!options.language_servers.is_empty());
    }

    #[test]
    fn invalid_cli_values_and_ownership_combinations_are_rejected() {
        let huge = usize::MAX.to_string();
        for arguments in [
            vec!["--config"],
            vec!["--config", "a", "--config", "b"],
            vec!["--unknown"],
            vec!["--workers", "0"],
            vec!["--workers", "33"],
            vec!["--port", "65536"],
            vec!["--syntax-cache-mib", &huge],
            vec!["--analysis-input-mib", &huge],
            vec!["--source-spool-mib", &huge],
            vec!["--once", "--stdio"],
            vec![
                "--vscode-connection",
                "bridge.json",
                "--rust-analyzer",
                "rust-analyzer",
            ],
            vec!["--vscode-connection", "bridge.json", "--pyright", "pyright"],
            vec![
                "--vscode-connection",
                "bridge.json",
                "--ruby-lsp",
                "ruby-lsp",
            ],
            vec!["--vscode-connection", "bridge.json", "--clangd", "clangd"],
            vec![
                "--vscode-connection",
                "bridge.json",
                "--python-path",
                "python",
            ],
            vec![
                "--vscode-connection",
                "bridge.json",
                "--compile-commands",
                "db.json",
            ],
        ] {
            let error = load(arguments.iter().map(|arg| (*arg).to_owned()))
                .err()
                .unwrap_or_else(|| panic!("accepted {arguments:?}"));
            if arguments.first() == Some(&"--vscode-connection") {
                assert!(error.contains("cannot be combined"), "{error}");
            }
        }
    }

    #[test]
    fn mib_budgets_reject_the_first_value_that_cannot_be_represented_in_bytes() {
        let maximum = usize::MAX >> 20;
        let valid = maximum.to_string();
        let invalid = (maximum + 1).to_string();
        for option in [
            "--syntax-cache-mib",
            "--analysis-input-mib",
            "--source-spool-mib",
        ] {
            assert!(
                load([option.to_owned(), valid.clone()].into_iter())
                    .unwrap()
                    .is_some()
            );
            let error = load([option.to_owned(), invalid.clone()].into_iter())
                .err()
                .unwrap();
            assert!(error.contains("exceeds byte capacity"), "{error}");
        }
    }
}
