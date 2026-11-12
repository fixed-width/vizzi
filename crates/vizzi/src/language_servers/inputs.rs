//! Frozen project configuration translated into a captured-source mirror.
use crate::lsp::Setup;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

#[derive(Clone)]
pub(super) enum Profile {
    Python { interpreter: Option<PathBuf> },
    CFamily { database: Option<PathBuf> },
    Ruby { executable: PathBuf },
}
impl Profile {
    pub(super) fn accepts(&self, language: &str) -> bool {
        match self {
            Self::Python { .. } => language == "python",
            Self::CFamily { .. } => matches!(language, "c" | "cpp"),
            Self::Ruby { .. } => language == "ruby",
        }
    }
}

pub(super) struct Inputs {
    profile: Vec<u8>,
    raw: BTreeMap<PathBuf, Option<Vec<u8>>>,
    python: Option<Value>,
    configs: BTreeMap<PathBuf, Value>,
    database: Option<Value>,
    flags: Option<String>,
    root_config: PathBuf,
}
pub(super) fn absolute(base: &Path, path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    };
    let mut result = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(component.as_os_str()),
        }
    }
    result
}
fn jsonc(text: &str) -> Result<Value, String> {
    jsonc_parser::parse_to_serde_value(
        text,
        &jsonc_parser::ParseOptions {
            allow_comments: true,
            allow_trailing_commas: true,
            allow_loose_object_property_names: false,
            allow_missing_commas: false,
            allow_single_quoted_strings: false,
            allow_hexadecimal_numbers: false,
            allow_unary_plus_numbers: false,
            allow_bare_decimal_point_numbers: false,
            allow_non_finite_numbers: false,
            allow_extended_string_escapes: false,
        },
    )
    .map_err(|e| e.to_string())
}
fn save(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, serde_json::to_vec(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
impl Inputs {
    fn read(&mut self, path: &Path) -> Result<Option<Vec<u8>>, String> {
        if let Some(value) = self.raw.get(path) {
            return Ok(value.clone());
        }
        let metadata = match fs::metadata(path) {
            Ok(value) => value,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.raw.insert(path.into(), None);
                return Ok(None);
            }
            Err(e) => return Err(e.to_string()),
        };
        if !metadata.is_file() {
            return Err("Language-server configuration must be a regular file".into());
        }
        let mut bytes = vec![];
        fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 8 * 1024 * 1024
            || self.raw.values().flatten().map(Vec::len).sum::<usize>() + bytes.len()
                > 16 * 1024 * 1024
            || self.raw.len() >= 64
        {
            return Err("Language-server configuration exceeds allowance".into());
        }
        self.raw.insert(path.into(), Some(bytes.clone()));
        Ok(Some(bytes))
    }
    fn config(&mut self, path: &Path, depth: usize) -> Result<Value, String> {
        if depth > 8 || self.configs.contains_key(path) {
            return Err("Cyclic or deeply nested Python configuration".into());
        }
        let bytes = self
            .read(path)?
            .ok_or("Extended Python configuration is missing")?;
        let value = jsonc(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)?;
        self.configs.insert(path.into(), value.clone());
        if let Some(extends) = value.get("extends") {
            let extends = extends.as_str().ok_or("Python extends must be a path")?;
            let parent = path.parent().ok_or("Configuration parent missing")?;
            self.config(&absolute(parent, Path::new(extends)), depth + 1)?;
        }
        Ok(value)
    }
    pub(super) fn load(root: &Path, profile: &Profile) -> Result<Self, String> {
        let mut result = Self {
            profile: match profile {
                Profile::Python { interpreter } => format!("python:{interpreter:?}"),
                Profile::CFamily { database } => format!("c-family:{database:?}"),
                Profile::Ruby { executable } => format!(
                    "ruby:{executable:?}:{:?}:{:?}",
                    std::env::var_os("GEM_HOME"),
                    std::env::var_os("GEM_PATH")
                ),
            }
            .into_bytes(),
            raw: BTreeMap::new(),
            python: None,
            configs: BTreeMap::new(),
            database: None,
            flags: None,
            root_config: root.join("pyrightconfig.json"),
        };
        match profile {
            Profile::Ruby { .. } => {
                for name in ["Gemfile", "Gemfile.lock", ".ruby-version", ".tool-versions"] {
                    result.read(&root.join(name))?;
                }
            }
            Profile::Python { .. } => {
                let config = root.join("pyrightconfig.json");
                let pyproject = root.join("pyproject.toml");
                let project = result.read(&pyproject)?;
                if result.read(&config)?.is_some() {
                    result.python = Some(result.config(&config, 0)?);
                } else if let Some(bytes) = project {
                    let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
                    let table = text.parse::<toml::Table>().map_err(|e| e.to_string())?;
                    if let Some(value) = table.get("tool").and_then(|tool| tool.get("pyright")) {
                        let value = serde_json::to_value(value).map_err(|e| e.to_string())?;
                        if let Some(parent) = value.get("extends").and_then(Value::as_str) {
                            result.config(&absolute(root, Path::new(parent)), 1)?;
                        }
                        result.python = Some(value);
                    }
                }
            }
            Profile::CFamily { database } => {
                let path = database.as_ref().map_or_else(
                    || root.join("compile_commands.json"),
                    |path| absolute(root, path),
                );
                result.root_config = path.clone();
                if let Some(bytes) = result.read(&path)? {
                    let value: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                    if !value.is_array() {
                        return Err("Compilation database must be an array".into());
                    }
                    result.database = Some(value);
                } else if database.is_none()
                    && let Some(bytes) = result.read(&root.join("compile_flags.txt"))?
                {
                    result.flags = Some(String::from_utf8(bytes).map_err(|e| e.to_string())?);
                }
                if result.database.is_none() && result.flags.is_none() {
                    return Err("clangd needs compile_commands.json or compile_flags.txt; no build is run to create it".into());
                }
            }
        }
        Ok(result)
    }
    pub(super) fn digest(&self) -> String {
        let mut hash = blake3::Hasher::new();
        if self.profile.starts_with(b"ruby:") {
            hash.update(include_bytes!("ruby.rs"));
            hash.update(include_bytes!("ruby-bootstrap.rb"));
        }
        hash.update(include_bytes!("inputs.rs"));
        hash.update(&self.profile);
        for (path, bytes) in &self.raw {
            let path = path.as_os_str().as_encoded_bytes();
            hash.update(&(path.len() as u64).to_le_bytes());
            hash.update(path);
            hash.update(&[u8::from(bytes.is_some())]);
            if let Some(bytes) = bytes {
                hash.update(&(bytes.len() as u64).to_le_bytes());
                hash.update(bytes);
            }
        }
        hash.finalize().to_hex().to_string()
    }
    fn config_path(mirror: &Path, path: &Path) -> PathBuf {
        mirror.join(".vizzi-configuration").join(format!(
            "{}.json",
            blake3::hash(path.as_os_str().as_encoded_bytes()).to_hex()
        ))
    }
    fn python_config(
        &self,
        mut value: Value,
        original: &Path,
        root: &Path,
        mirror: &Path,
    ) -> Result<Value, String> {
        let base = original.parent().ok_or("Configuration parent missing")?;
        if let Some(parent) = value.get_mut("extends") {
            let path = absolute(
                base,
                Path::new(parent.as_str().ok_or("Python extends must be a path")?),
            );
            *parent = Self::config_path(mirror, &path)
                .to_string_lossy()
                .to_string()
                .into();
        }
        if let Some(path) = value.get_mut("venvPath") {
            *path = absolute(
                base,
                Path::new(path.as_str().ok_or("venvPath must be a path")?),
            )
            .to_string_lossy()
            .to_string()
            .into();
        }
        fn rewrite(
            value: &mut Value,
            base: &Path,
            root: &Path,
            mirror: &Path,
        ) -> Result<(), String> {
            if let Some(path) = value.as_str() {
                let path = absolute(base, Path::new(path));
                let path = path
                    .strip_prefix(root)
                    .map(|relative| mirror.join(relative))
                    .unwrap_or(path);
                *value = path.to_string_lossy().to_string().into();
            } else {
                return Err("Python search paths must be strings".into());
            }
            Ok(())
        }
        for key in ["include", "exclude", "ignore", "strict", "extraPaths"] {
            if let Some(paths) = value.get_mut(key) {
                for path in paths
                    .as_array_mut()
                    .ok_or("Python paths must be an array")?
                {
                    rewrite(path, base, root, mirror)?;
                }
            }
        }
        if let Some(path) = value.get_mut("stubPath") {
            rewrite(path, base, root, mirror)?;
        }
        if let Some(environments) = value.get_mut("executionEnvironments") {
            for env in environments
                .as_array_mut()
                .ok_or("Python execution environments must be an array")?
            {
                if let Some(path) = env.get_mut("root") {
                    rewrite(path, base, root, mirror)?;
                }
                if let Some(paths) = env.get_mut("extraPaths") {
                    for path in paths
                        .as_array_mut()
                        .ok_or("Python extraPaths must be an array")?
                    {
                        rewrite(path, base, root, mirror)?;
                    }
                }
            }
        }
        Ok(value)
    }
    pub(super) fn prepare(
        &self,
        root: &Path,
        mirror: &Path,
        profile: &Profile,
    ) -> Result<Setup, String> {
        match profile {
            Profile::Ruby { .. } => super::ruby::prepare(
                mirror,
                self.raw
                    .get(&root.join("Gemfile.lock"))
                    .and_then(|v| v.as_deref()),
            ),
            Profile::Python { interpreter } => {
                for (path, value) in &self.configs {
                    save(
                        &Self::config_path(mirror, path),
                        &self.python_config(value.clone(), path, root, mirror)?,
                    )?;
                }
                if let Some(value) = &self.python {
                    save(
                        &mirror.join("pyrightconfig.json"),
                        &self.python_config(value.clone(), &self.root_config, root, mirror)?,
                    )?;
                }
                let mut python = json!({"analysis":{"diagnosticMode":"openFilesOnly","autoSearchPaths":true,"useLibraryCodeForTypes":true}});
                if let Some(path) = interpreter {
                    let path = if path.is_relative() && path.components().count() > 1 {
                        absolute(root, path)
                    } else {
                        path.clone()
                    };
                    python["pythonPath"] = path.to_string_lossy().to_string().into();
                }
                Ok(Setup {
                    arguments: vec!["--stdio".into()],
                    initialization: json!({}),
                    configuration: json!({"python":python}),
                    status_notifications: false,
                    progress_token: None,
                    reopen_document_sync: false,
                    environment: vec![],
                })
            }
            Profile::CFamily { .. } => {
                if let Some(database) = &self.database {
                    let commands = compilation_database(database, root, mirror)?;
                    if commands.is_empty() {
                        return Err(
                            "Compilation database has no files within the captured root".into()
                        );
                    }
                    save(
                        &mirror.join("compile_commands.json"),
                        &Value::Array(commands),
                    )?;
                }
                if let Some(flags) = &self.flags {
                    let args = flags
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.trim().is_empty())
                        .map(str::to_owned)
                        .collect::<Vec<_>>();
                    let mut translated = translate_args(&args, root, root, mirror)?;
                    translated.push(format!(
                        "-fmodules-cache-path={}",
                        mirror.join(".vizzi-modules").display()
                    ));
                    fs::write(mirror.join("compile_flags.txt"), translated.join("\n"))
                        .map_err(|e| e.to_string())?;
                }
                Ok(Setup {
                    arguments: vec![
                        "--log=error".into(),
                        "-j=1".into(),
                        "--background-index".into(),
                        "--enable-config=false".into(),
                        "--clang-tidy=false".into(),
                        "--pch-storage=memory".into(),
                    ],
                    initialization: json!({}),
                    configuration: json!({}),
                    status_notifications: false,
                    progress_token: None,
                    reopen_document_sync: false,
                    environment: vec![],
                })
            }
        }
    }
}

fn mapped(path: &str, base: &Path, root: &Path, mirror: &Path) -> String {
    let path = absolute(base, Path::new(path));
    path.strip_prefix(root)
        .map(|relative| mirror.join(relative))
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}
fn translate_args(
    args: &[String],
    base: &Path,
    root: &Path,
    mirror: &Path,
) -> Result<Vec<String>, String> {
    let mut result = vec![];
    let mut paths = false;
    for argument in args {
        if paths {
            result.push(mapped(argument, base, root, mirror));
            paths = false;
            continue;
        }
        if matches!(
            argument.as_str(),
            "-Xclang" | "-load" | "-plugin" | "-fplugin"
        ) || argument.starts_with("-fplugin=")
            || argument.starts_with("-Xclang=")
            || argument.starts_with('@')
        {
            return Err("Compiler plugin/response-file flags are not supported by the captured clangd adapter".into());
        }
        if matches!(
            argument.as_str(),
            "-I" | "-isystem"
                | "-iquote"
                | "-include"
                | "-imacros"
                | "-isysroot"
                | "--sysroot"
                | "-o"
                | "-MF"
                | "-MJ"
        ) {
            result.push(argument.clone());
            paths = true;
            continue;
        }
        let mut translated = None;
        for prefix in [
            "--sysroot=",
            "-isystem",
            "-iquote",
            "-I",
            "-fmodule-map-file=",
        ] {
            if let Some(path) = argument
                .strip_prefix(prefix)
                .filter(|path| !path.is_empty())
            {
                translated = Some(format!("{prefix}{}", mapped(path, base, root, mirror)));
                break;
            }
        }
        result.push(translated.unwrap_or_else(|| {
            if !argument.starts_with('-')
                && (Path::new(argument).is_absolute()
                    || argument.contains('/')
                    || argument.ends_with(".c")
                    || argument.ends_with(".cpp")
                    || argument.ends_with(".cc")
                    || argument.ends_with(".cxx"))
            {
                mapped(argument, base, root, mirror)
            } else {
                argument.replace(
                    &format!("{}/", root.display()),
                    &format!("{}/", mirror.display()),
                )
            }
        }));
    }
    if paths {
        return Err("Compiler path option has no value".into());
    }
    Ok(result)
}
fn compilation_database(value: &Value, root: &Path, mirror: &Path) -> Result<Vec<Value>, String> {
    let mut result = vec![];
    for entry in value
        .as_array()
        .ok_or("Compilation database must be an array")?
    {
        if result.len() >= 16384 {
            return Err("Compilation database exceeds command allowance".into());
        }
        let base = absolute(
            root,
            Path::new(
                entry["directory"]
                    .as_str()
                    .ok_or("Compile command directory missing")?,
            ),
        );
        let file = absolute(
            &base,
            Path::new(
                entry["file"]
                    .as_str()
                    .ok_or("Compile command file missing")?,
            ),
        );
        let Ok(relative) = file.strip_prefix(root) else {
            continue;
        };
        let args = if let Some(arguments) = entry.get("arguments") {
            arguments
                .as_array()
                .ok_or("Compile arguments must be an array")?
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_owned)
                        .ok_or("Compile argument must be a string")
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            shlex::split(entry["command"].as_str().ok_or("Compile command missing")?)
                .ok_or("Invalid shell quoting in compilation database")?
        };
        if args.is_empty() || args.len() > 4096 {
            return Err("Compile arguments exceed allowance".into());
        }
        let mut args = translate_args(&args, &base, root, mirror)?;
        args.push(format!(
            "-fmodules-cache-path={}",
            mirror.join(".vizzi-modules").display()
        ));
        let directory = base
            .strip_prefix(root)
            .map(|relative| mirror.join(relative))
            .unwrap_or_else(|_| mirror.join(".vizzi-build"));
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        result.push(json!({"directory":directory,"file":mirror.join(relative),"arguments":args}));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiles_accept_only_their_languages_and_paths_keep_parent_semantics() {
        let python = Profile::Python { interpreter: None };
        let c = Profile::CFamily { database: None };
        let ruby = Profile::Ruby {
            executable: "ruby-lsp".into(),
        };
        for language in ["python", "ruby", "rust", "c", "cpp", "unknown"] {
            assert_eq!(ruby.accepts(language), language == "ruby");
            assert_eq!(python.accepts(language), language == "python");
            assert_eq!(c.accepts(language), matches!(language, "c" | "cpp"));
        }
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            absolute(root.path(), Path::new("src/../include/./value.h")),
            root.path().join("include/value.h")
        );
        assert_eq!(
            absolute(root.path(), root.path().join("lib/../worker.cpp").as_path()),
            root.path().join("worker.cpp")
        );
    }
    #[test]
    fn configuration_reads_preserve_exact_byte_bounds_and_reject_filesystem_errors() {
        let root = tempfile::tempdir().unwrap();
        let profile = Profile::Python { interpreter: None };
        let limit = 8 * 1024 * 1024;
        let exact = format!("{}{{}}", " ".repeat(limit - 2));
        fs::write(root.path().join("pyrightconfig.json"), &exact).unwrap();
        assert!(Inputs::load(root.path(), &profile).is_ok());
        fs::write(root.path().join("pyrightconfig.json"), format!("{exact} ")).unwrap();
        assert!(Inputs::load(root.path(), &profile).is_err());
        let file = root.path().join("not-a-directory");
        fs::write(&file, "{}").unwrap();
        assert!(Inputs::load(&file, &profile).is_err());
        fs::write(
            root.path().join("pyrightconfig.json"),
            format!("{}{{\"extends\":\"base.json\"}}", " ".repeat(limit - 23)),
        )
        .unwrap();
        fs::write(root.path().join("base.json"), &exact).unwrap();
        assert!(
            Inputs::load(root.path(), &profile).is_ok(),
            "exactly 16 MiB of configuration is admitted"
        );
        fs::write(
            root.path().join("base.json"),
            format!("{}{{\"extends\":\"leaf.json\"}}", " ".repeat(limit - 23)),
        )
        .unwrap();
        fs::write(root.path().join("leaf.json"), "{}").unwrap();
        assert!(Inputs::load(root.path(), &profile).is_err());
    }
    #[test]
    fn inherited_configuration_depth_accepts_the_boundary_and_rejects_the_next_file() {
        let root = tempfile::tempdir().unwrap();
        let profile = Profile::Python { interpreter: None };
        for depth in 0..=8 {
            let name = if depth == 0 {
                "pyrightconfig.json".into()
            } else {
                format!("config{depth}.json")
            };
            let value = if depth == 8 {
                json!({})
            } else {
                json!({"extends":format!("config{}.json",depth+1)})
            };
            fs::write(root.path().join(name), serde_json::to_vec(&value).unwrap()).unwrap();
        }
        assert!(Inputs::load(root.path(), &profile).is_ok());
        fs::write(
            root.path().join("config8.json"),
            "{\"extends\":\"config9.json\"}",
        )
        .unwrap();
        fs::write(root.path().join("config9.json"), "{}").unwrap();
        assert!(Inputs::load(root.path(), &profile).is_err());
    }
    #[test]
    fn python_configuration_is_frozen_with_comments_inheritance_and_environment_paths() {
        let root = tempfile::tempdir().unwrap();
        let mirror = tempfile::tempdir().unwrap();
        fs::write(root.path().join("base.json"), r#"{"extraPaths":["typed"]}"#).unwrap();
        let original = "{ // project configuration\n\"extends\":\"base.json\",\"include\":[\"src\"],\"venvPath\":\".\",\"venv\":\".venv\",}";
        fs::write(root.path().join("pyrightconfig.json"), original).unwrap();
        let profile = Profile::Python {
            interpreter: Some("python3".into()),
        };
        let inputs = Inputs::load(root.path(), &profile).unwrap();
        let setup = inputs
            .prepare(root.path(), mirror.path(), &profile)
            .unwrap();
        assert_eq!(setup.arguments, ["--stdio"]);
        assert!(!setup.status_notifications);
        assert_eq!(setup.configuration["python"]["pythonPath"], "python3");
        let output: Value =
            serde_json::from_slice(&fs::read(mirror.path().join("pyrightconfig.json")).unwrap())
                .unwrap();
        assert_eq!(
            output["include"][0],
            mirror.path().join("src").to_str().unwrap()
        );
        assert_eq!(output["venvPath"], root.path().to_str().unwrap());
        let parent: Value =
            serde_json::from_slice(&fs::read(output["extends"].as_str().unwrap()).unwrap())
                .unwrap();
        assert_eq!(
            parent["extraPaths"][0],
            mirror.path().join("typed").to_str().unwrap()
        );
        fs::write(
            root.path().join("base.json"),
            r#"{"extraPaths":["changed"]}"#,
        )
        .unwrap();
        assert_ne!(
            inputs.digest(),
            Inputs::load(root.path(), &profile).unwrap().digest()
        );
        assert_eq!(
            fs::read_to_string(root.path().join("pyrightconfig.json")).unwrap(),
            original
        );
        assert_ne!(
            Inputs::load(root.path(), &profile).unwrap().digest(),
            Inputs::load(root.path(), &Profile::Python { interpreter: None })
                .unwrap()
                .digest()
        );
    }
    #[test]
    fn pyproject_settings_and_invalid_project_inputs_remain_explicit() {
        let root = tempfile::tempdir().unwrap();
        let mirror = tempfile::tempdir().unwrap();
        let profile = Profile::Python { interpreter: None };
        fs::write(
            root.path().join("pyproject.toml"),
            "[tool.pyright]\nextraPaths=['src']\n",
        )
        .unwrap();
        Inputs::load(root.path(), &profile)
            .unwrap()
            .prepare(root.path(), mirror.path(), &profile)
            .unwrap();
        assert!(mirror.path().join("pyrightconfig.json").is_file());
        for text in [
            "{broken",
            "{\"extends\":\"pyrightconfig.json\"}",
            "{\"extends\":\"missing.json\"}",
        ] {
            fs::write(root.path().join("pyrightconfig.json"), text).unwrap();
            assert!(Inputs::load(root.path(), &profile).is_err());
        }
        assert!(Inputs::load(root.path(), &Profile::CFamily { database: None }).is_err());
    }
    #[test]
    fn compilation_commands_translate_quoted_source_include_and_output_paths_without_execution() {
        let root = tempfile::tempdir().unwrap();
        let mirror = tempfile::tempdir().unwrap();
        let command = format!(
            "clang++ -I '{}' -DCHOICE=1 -std=c++20 -c '{}' -o '{}'",
            root.path().join("include files").display(),
            root.path().join("worker.cpp").display(),
            root.path().join("build/worker.o").display()
        );
        let database = json!([{"directory":root.path(),"file":"worker.cpp","command":command}]);
        fs::write(
            root.path().join("compile_commands.json"),
            serde_json::to_vec(&database).unwrap(),
        )
        .unwrap();
        let profile = Profile::CFamily { database: None };
        let inputs = Inputs::load(root.path(), &profile).unwrap();
        let setup = inputs
            .prepare(root.path(), mirror.path(), &profile)
            .unwrap();
        assert!(setup.arguments.contains(&"-j=1".into()));
        let transformed: Value =
            serde_json::from_slice(&fs::read(mirror.path().join("compile_commands.json")).unwrap())
                .unwrap();
        assert_eq!(
            transformed[0]["file"],
            mirror.path().join("worker.cpp").to_str().unwrap()
        );
        let args = transformed[0]["arguments"].as_array().unwrap();
        assert!(args.contains(&json!(mirror.path().join("include files"))));
        assert!(args.contains(&json!(mirror.path().join("build/worker.o"))));
        assert!(args.contains(&json!("-DCHOICE=1")));
        for flag in ["@response", "-Xclang", "-fplugin=plugin.so"] {
            assert!(
                translate_args(&[flag.into()], root.path(), root.path(), mirror.path()).is_err()
            );
        }
        assert!(translate_args(&["-I".into()], root.path(), root.path(), mirror.path()).is_err());
        fs::remove_file(root.path().join("compile_commands.json")).unwrap();
        fs::write(
            root.path().join("compile_flags.txt"),
            "-std=c++20\n-Iinclude\n",
        )
        .unwrap();
        Inputs::load(root.path(), &profile)
            .unwrap()
            .prepare(root.path(), mirror.path(), &profile)
            .unwrap();
        assert!(
            fs::read_to_string(mirror.path().join("compile_flags.txt"))
                .unwrap()
                .contains(mirror.path().join("include").to_str().unwrap())
        );
    }
    #[test]
    fn relative_environment_and_compiler_paths_keep_flags_distinct_from_source_arguments() {
        let root = tempfile::tempdir().unwrap();
        let mirror = tempfile::tempdir().unwrap();
        let profile = Profile::Python {
            interpreter: Some(".venv/bin/python".into()),
        };
        let setup = Inputs::load(root.path(), &profile)
            .unwrap()
            .prepare(root.path(), mirror.path(), &profile)
            .unwrap();
        assert_eq!(
            setup.configuration["python"]["pythonPath"],
            root.path().join(".venv/bin/python").to_str().unwrap()
        );
        for file in [
            "worker.c",
            "worker.cpp",
            "worker.cc",
            "worker.cxx",
            "src/worker.cpp",
            "build/worker.o",
        ] {
            let output =
                translate_args(&[file.into()], root.path(), root.path(), mirror.path()).unwrap();
            assert_eq!(
                output,
                vec![mirror.path().join(file).to_string_lossy().to_string()]
            );
        }
        let flag = format!("-DPROJECT={}/src", root.path().display());
        let output = translate_args(
            &[flag, "-std=c++20".into()],
            root.path(),
            root.path(),
            mirror.path(),
        )
        .unwrap();
        assert_eq!(
            output,
            vec![
                format!("-DPROJECT={}/src", mirror.path().display()),
                "-std=c++20".into()
            ]
        );
    }
    #[test]
    fn compilation_argument_limits_accept_the_boundary_and_reject_empty_or_oversized_entries() {
        let root = tempfile::tempdir().unwrap();
        let mirror = tempfile::tempdir().unwrap();
        for (count, allowed) in [(0, false), (4096, true), (4097, false)] {
            let args = vec!["-DCHOICE=1"; count];
            let data = json!([{"directory":root.path(),"file":"worker.cpp","arguments":args}]);
            assert_eq!(
                compilation_database(&data, root.path(), mirror.path()).is_ok(),
                allowed,
                "{count}"
            );
        }
    }
}
