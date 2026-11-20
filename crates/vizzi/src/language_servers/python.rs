//! Pyright project inputs and admission of local Python dependency source.
use super::{
    inputs::{InputFiles, Inputs, absolute, save},
    project::{ExternalSource, Profile},
};
use crate::lsp::Setup;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub(super) struct Pyright {
    pub interpreter: Option<PathBuf>,
}
impl Profile for Pyright {
    fn languages(&self) -> &'static [&'static str] {
        &["python"]
    }

    fn inputs<'a>(&'a self, root: &'a Path) -> Result<Inputs<'a>, String> {
        let mut files = InputFiles::default();
        let project = PythonInputs::load(root, &mut files)?;
        Ok(files.freeze(
            &format!("python:{:?}", self.interpreter),
            &[include_bytes!("python.rs")],
            move |mirror| project.prepare(root, mirror, self.interpreter.as_deref()),
        ))
    }

    fn external_source<'a>(&self, path: &'a Path, _: &'a str) -> Option<ExternalSource<'a>> {
        if !matches!(path.extension()?.to_str()?, "py" | "pyi") {
            return None;
        }
        let mut root = path.parent()?;
        for _ in 0..16 {
            let Some(parent) = root.parent() else { break };
            if initialized(root) && initialized(parent) {
                root = parent;
                continue;
            }
            break;
        }
        let name = if initialized(root) {
            root.file_name()
        } else {
            path.file_stem()
        };
        Some(ExternalSource {
            root,
            language: "python",
            package: name
                .and_then(|name| name.to_str())
                .unwrap_or("Python source"),
        })
    }
}

fn initialized(path: &Path) -> bool {
    path.join("__init__.py").is_file() || path.join("__init__.pyi").is_file()
}

struct PythonInputs {
    python: Option<Value>,
    configs: BTreeMap<PathBuf, Value>,
    root_config: PathBuf,
}
impl PythonInputs {
    fn load(root: &Path, files: &mut InputFiles) -> Result<Self, String> {
        let config = root.join("pyrightconfig.json");
        let mut result = Self {
            python: None,
            configs: BTreeMap::new(),
            root_config: config.clone(),
        };
        let pyproject = root.join("pyproject.toml");
        let project = files.read(&pyproject)?;
        if files.read(&config)?.is_some() {
            result.python = Some(result.config(files, &config, 0)?);
        } else if let Some(bytes) = project {
            let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
            let table = text.parse::<toml::Table>().map_err(|e| e.to_string())?;
            if let Some(value) = table.get("tool").and_then(|tool| tool.get("pyright")) {
                let value = serde_json::to_value(value).map_err(|e| e.to_string())?;
                if let Some(parent) = value.get("extends").and_then(Value::as_str) {
                    result.config(files, &absolute(root, Path::new(parent)), 1)?;
                }
                result.python = Some(value);
            }
        }
        Ok(result)
    }

    fn config(
        &mut self,
        files: &mut InputFiles,
        path: &Path,
        depth: usize,
    ) -> Result<Value, String> {
        if depth > 8 || self.configs.contains_key(path) {
            return Err("Cyclic or deeply nested Python configuration".into());
        }
        let bytes = files
            .read(path)?
            .ok_or("Extended Python configuration is missing")?;
        let value = jsonc(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)?;
        self.configs.insert(path.into(), value.clone());
        if let Some(extends) = value.get("extends") {
            let extends = extends.as_str().ok_or("Python extends must be a path")?;
            let parent = path.parent().ok_or("Configuration parent missing")?;
            self.config(files, &absolute(parent, Path::new(extends)), depth + 1)?;
        }
        Ok(value)
    }

    fn config_path(mirror: &Path, path: &Path) -> PathBuf {
        mirror.join(".vizzi-configuration").join(format!(
            "{}.json",
            blake3::hash(path.as_os_str().as_encoded_bytes()).to_hex()
        ))
    }
    fn python_config(
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
    fn prepare(
        &self,
        root: &Path,
        mirror: &Path,
        interpreter: Option<&Path>,
    ) -> Result<Setup, String> {
        for (path, value) in &self.configs {
            save(
                &Self::config_path(mirror, path),
                &Self::python_config(value.clone(), path, root, mirror)?,
            )?;
        }
        if let Some(value) = &self.python {
            save(
                &mirror.join("pyrightconfig.json"),
                &Self::python_config(value.clone(), &self.root_config, root, mirror)?,
            )?;
        }
        let mut python = json!({"analysis":{"diagnosticMode":"openFilesOnly","autoSearchPaths":true,"useLibraryCodeForTypes":true}});
        if let Some(path) = interpreter {
            let path = if path.is_relative() && path.components().count() > 1 {
                absolute(root, path)
            } else {
                path.to_owned()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn external_stub_packages_use_the_highest_captured_initializer_chain() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("package");
        let nested = package.join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::write(package.join("__init__.py"), "").unwrap();
        fs::write(nested.join("__init__.pyi"), "").unwrap();
        let path = nested.join("worker.pyi");
        let profile = Pyright { interpreter: None };
        let source = profile.external_source(&path, "python").unwrap();
        assert_eq!(source.root, package);
        assert_eq!(source.package, "package");
        assert_eq!(source.language, "python");
        fs::remove_file(nested.join("__init__.pyi")).unwrap();
        let source = profile.external_source(&path, "python").unwrap();
        assert_eq!(source.root, nested);
        assert_eq!(source.package, "worker");
        assert!(
            profile
                .external_source(&nested.join("worker.rb"), "python")
                .is_none()
        );
    }

    #[test]
    fn configuration_reads_preserve_exact_byte_bounds_and_reject_filesystem_errors() {
        let root = tempfile::tempdir().unwrap();
        let profile = Pyright { interpreter: None };
        let limit = 8 * 1024 * 1024;
        let exact = format!("{}{{}}", " ".repeat(limit - 2));
        fs::write(root.path().join("pyrightconfig.json"), &exact).unwrap();
        assert!(profile.inputs(root.path()).is_ok());
        fs::write(root.path().join("pyrightconfig.json"), format!("{exact} ")).unwrap();
        assert!(profile.inputs(root.path()).is_err());
        let file = root.path().join("not-a-directory");
        fs::write(&file, "{}").unwrap();
        assert!(profile.inputs(&file).is_err());
        fs::write(
            root.path().join("pyrightconfig.json"),
            format!("{}{{\"extends\":\"base.json\"}}", " ".repeat(limit - 23)),
        )
        .unwrap();
        fs::write(root.path().join("base.json"), &exact).unwrap();
        assert!(
            profile.inputs(root.path()).is_ok(),
            "exactly 16 MiB of configuration is admitted"
        );
        fs::write(
            root.path().join("base.json"),
            format!("{}{{\"extends\":\"leaf.json\"}}", " ".repeat(limit - 23)),
        )
        .unwrap();
        fs::write(root.path().join("leaf.json"), "{}").unwrap();
        assert!(profile.inputs(root.path()).is_err());
    }

    #[test]
    fn inherited_configuration_depth_accepts_the_boundary_and_rejects_the_next_file() {
        let root = tempfile::tempdir().unwrap();
        let profile = Pyright { interpreter: None };
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
        assert!(profile.inputs(root.path()).is_ok());
        fs::write(
            root.path().join("config8.json"),
            "{\"extends\":\"config9.json\"}",
        )
        .unwrap();
        fs::write(root.path().join("config9.json"), "{}").unwrap();
        assert!(profile.inputs(root.path()).is_err());
    }

    #[test]
    fn python_configuration_is_frozen_with_comments_inheritance_and_environment_paths() {
        let root = tempfile::tempdir().unwrap();
        let mirror = tempfile::tempdir().unwrap();
        fs::write(root.path().join("base.json"), r#"{"extraPaths":["typed"]}"#).unwrap();
        let original = "{ // project configuration\n\"extends\":\"base.json\",\"include\":[\"src\"],\"venvPath\":\".\",\"venv\":\".venv\",}";
        fs::write(root.path().join("pyrightconfig.json"), original).unwrap();
        let profile = Pyright {
            interpreter: Some("python3".into()),
        };
        let inputs = profile.inputs(root.path()).unwrap();
        let digest = inputs.digest().to_owned();
        let setup = inputs.prepare(mirror.path()).unwrap();
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
        assert_ne!(digest, profile.inputs(root.path()).unwrap().digest());
        assert_eq!(
            fs::read_to_string(root.path().join("pyrightconfig.json")).unwrap(),
            original
        );
        assert_ne!(
            profile.inputs(root.path()).unwrap().digest(),
            Pyright { interpreter: None }
                .inputs(root.path())
                .unwrap()
                .digest()
        );
    }

    #[test]
    fn pyproject_settings_and_invalid_project_inputs_remain_explicit() {
        let root = tempfile::tempdir().unwrap();
        let mirror = tempfile::tempdir().unwrap();
        let profile = Pyright { interpreter: None };
        fs::write(
            root.path().join("pyproject.toml"),
            "[tool.pyright]\nextraPaths=['src']\n",
        )
        .unwrap();
        profile
            .inputs(root.path())
            .unwrap()
            .prepare(mirror.path())
            .unwrap();
        assert!(mirror.path().join("pyrightconfig.json").is_file());
        for text in [
            "{broken",
            "{\"extends\":\"pyrightconfig.json\"}",
            "{\"extends\":\"missing.json\"}",
        ] {
            fs::write(root.path().join("pyrightconfig.json"), text).unwrap();
            assert!(profile.inputs(root.path()).is_err());
        }
    }

    #[test]
    fn relative_interpreter_path_uses_the_original_root() {
        let root = tempfile::tempdir().unwrap();
        let mirror = tempfile::tempdir().unwrap();
        let profile = Pyright {
            interpreter: Some(".venv/bin/python".into()),
        };
        let setup = profile
            .inputs(root.path())
            .unwrap()
            .prepare(mirror.path())
            .unwrap();
        assert_eq!(
            setup.configuration["python"]["pythonPath"],
            root.path().join(".venv/bin/python").to_str().unwrap()
        );
    }
}
