//! clangd compilation inputs and admission of local C/C++ dependency source.
use super::{
    inputs::{InputFiles, Inputs, absolute, save},
    project::{ExternalSource, Profile},
};
use crate::lsp::Setup;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) struct Clangd {
    pub database: Option<PathBuf>,
}
impl Profile for Clangd {
    fn languages(&self) -> &'static [&'static str] {
        &["c", "cpp"]
    }

    fn inputs<'a>(&'a self, root: &'a Path) -> Result<Inputs<'a>, String> {
        let mut files = InputFiles::default();
        let path = self.database.as_ref().map_or_else(
            || root.join("compile_commands.json"),
            |path| absolute(root, path),
        );
        let compilation = if let Some(bytes) = files.read(&path)? {
            let value: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if !value.is_array() {
                return Err("Compilation database must be an array".into());
            }
            Compilation::Database(value)
        } else if self.database.is_none()
            && let Some(bytes) = files.read(&root.join("compile_flags.txt"))?
        {
            Compilation::Flags(String::from_utf8(bytes).map_err(|e| e.to_string())?)
        } else {
            return Err("clangd needs compile_commands.json or compile_flags.txt; no build is run to create it".into());
        };
        Ok(files.freeze(
            &format!("c-family:{:?}", self.database),
            &[include_bytes!("c_family.rs")],
            move |mirror| compilation.prepare(root, mirror),
        ))
    }

    fn external_source<'a>(&self, path: &'a Path, hint: &'a str) -> Option<ExternalSource<'a>> {
        let language = match path.extension().and_then(|value| value.to_str()) {
            Some("c") => "c",
            Some("cc" | "cpp" | "cxx" | "hh" | "hpp" | "hxx" | "tcc" | "ipp" | "inl") => "cpp",
            Some("h" | "inc") | None if self.accepts(hint) => hint,
            _ => return None,
        };
        Some(ExternalSource {
            root: path.parent()?,
            language,
            package: "C/C++ source",
        })
    }
}

enum Compilation {
    Database(Value),
    Flags(String),
}
impl Compilation {
    fn prepare(self, root: &Path, mirror: &Path) -> Result<Setup, String> {
        match self {
            Self::Database(database) => {
                let commands = compilation_database(&database, root, mirror)?;
                if commands.is_empty() {
                    return Err("Compilation database has no files within the captured root".into());
                }
                save(
                    &mirror.join("compile_commands.json"),
                    &Value::Array(commands),
                )?;
            }
            Self::Flags(flags) => {
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
    fn external_headers_use_the_caller_language_when_ambiguous() {
        let profile = Clangd { database: None };
        for (file, hint, language) in [
            ("lib/shared.h", "c", "c"),
            ("lib/shared.inc", "cpp", "cpp"),
            ("lib/cassert", "cpp", "cpp"),
            ("lib/source.c", "cpp", "c"),
            ("lib/template.tcc", "c", "cpp"),
        ] {
            let source = profile.external_source(Path::new(file), hint).unwrap();
            assert_eq!(source.root, Path::new("lib"));
            assert_eq!(source.language, language);
        }
        assert!(
            profile
                .external_source(Path::new("lib/helper.py"), "cpp")
                .is_none()
        );
        assert!(
            profile
                .external_source(Path::new("lib/shared.h"), "python")
                .is_none()
        );
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
        let profile = Clangd { database: None };
        let inputs = profile.inputs(root.path()).unwrap();
        let setup = inputs.prepare(mirror.path()).unwrap();
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
        profile
            .inputs(root.path())
            .unwrap()
            .prepare(mirror.path())
            .unwrap();
        assert!(
            fs::read_to_string(mirror.path().join("compile_flags.txt"))
                .unwrap()
                .contains(mirror.path().join("include").to_str().unwrap())
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

    #[test]
    fn compiler_flags_and_relative_source_paths_are_translated() {
        let root = tempfile::tempdir().unwrap();
        let mirror = tempfile::tempdir().unwrap();
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
    fn compile_flags_fallback_requires_the_default_database_path() {
        let root = tempfile::tempdir().unwrap();
        let profile = Clangd { database: None };
        assert!(profile.inputs(root.path()).is_err());
        fs::write(root.path().join("compile_flags.txt"), "-std=c++20").unwrap();
        assert!(profile.inputs(root.path()).is_ok());
        let profile = Clangd {
            database: Some("custom.json".into()),
        };
        assert!(profile.inputs(root.path()).is_err());
    }
}
