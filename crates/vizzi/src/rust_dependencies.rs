//! Rust-specific local package discovery and private Cargo path translation.
use crate::dependency_sources::Input;
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};
fn bounded(path: &Path) -> Result<String, String> {
    let mut s = String::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(128 * 1024 + 1)
        .read_to_string(&mut s)
        .map_err(|e| e.to_string())?;
    if s.len() > 128 * 1024 {
        return Err("Cargo manifest exceeds dependency allowance".into());
    }
    Ok(s)
}
pub(crate) fn input(path: &Path, configuration: &str) -> Result<Input, String> {
    let file = fs::canonicalize(path).map_err(|e| e.to_string())?;
    if !file.is_file() {
        return Err("Dependency definition is not a local file".into());
    }
    let mut parent = file.parent();
    for _ in 0..24 {
        let Some(dir) = parent else {
            break;
        };
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            let table = bounded(&manifest)?
                .parse::<toml::Table>()
                .map_err(|e| e.to_string())?;
            if let Some(package) = table.get("package") {
                let label = package
                    .get("name")
                    .and_then(toml::Value::as_str)
                    .ok_or("Dependency package name is unavailable")?;
                return Ok(Input {
                    root: dir.into(),
                    path: file
                        .strip_prefix(dir)
                        .map_err(|e| e.to_string())?
                        .to_str()
                        .ok_or("Dependency source path is not UTF-8")?
                        .replace('\\', "/"),
                    language: "rust".into(),
                    package: label.into(),
                    version: package
                        .get("version")
                        .and_then(toml::Value::as_str)
                        .map(str::to_owned),
                    configuration: configuration.into(),
                });
            }
        }
        parent = dir.parent();
    }
    let root = file
        .parent()
        .ok_or("Dependency source directory is unavailable")?;
    Ok(Input {
        root: root.into(),
        path: file
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("Dependency source path is not UTF-8")?
            .into(),
        language: "rust".into(),
        package: "Rust source".into(),
        version: None,
        configuration: configuration.into(),
    })
}
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(c.as_os_str()),
        }
    }
    result
}
fn dependencies(
    table: &mut toml::Table,
    original: &Path,
    mirror: &Path,
    base: &Path,
) -> Result<bool, String> {
    let mut changed = false;
    for value in table.iter_mut().map(|(_, v)| v) {
        if let Some(path) = value.get_mut("path") {
            let raw = path
                .as_str()
                .ok_or("Cargo dependency path is not a string")?;
            let joined = if Path::new(raw).is_absolute() {
                PathBuf::from(raw)
            } else {
                base.join(raw)
            };
            let absolute = fs::canonicalize(&joined).unwrap_or_else(|_| normalize(&joined));
            let target = if let Ok(relative) = absolute.strip_prefix(original) {
                mirror.join(relative)
            } else {
                absolute
            };
            let target = target
                .to_str()
                .ok_or("Cargo dependency path is not UTF-8")?;
            *path = toml::Value::String(target.into());
            changed = true;
        }
    }
    Ok(changed)
}
/// Only dependency path tables are translated; source targets retain their captured layout.
pub(crate) fn manifest(
    bytes: &[u8],
    relative: &str,
    original: &Path,
    mirror: &Path,
) -> Result<Vec<u8>, String> {
    let mut value = std::str::from_utf8(bytes)
        .map_err(|e| e.to_string())?
        .parse::<toml::Table>()
        .map_err(|e| e.to_string())?;
    let base = original.join(
        Path::new(relative)
            .parent()
            .unwrap_or_else(|| Path::new("")),
    );
    let mut changed = false;
    for name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(t) = value.get_mut(name).and_then(toml::Value::as_table_mut) {
            changed |= dependencies(t, original, mirror, &base)?;
        }
    }
    if let Some(t) = value
        .get_mut("workspace")
        .and_then(|v| v.get_mut("dependencies"))
        .and_then(toml::Value::as_table_mut)
    {
        changed |= dependencies(t, original, mirror, &base)?;
    }
    if let Some(targets) = value.get_mut("target").and_then(toml::Value::as_table_mut) {
        for target in targets.iter_mut().map(|(_, v)| v) {
            for name in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(t) = target.get_mut(name).and_then(toml::Value::as_table_mut) {
                    changed |= dependencies(t, original, mirror, &base)?;
                }
            }
        }
    }
    if let Some(patches) = value.get_mut("patch").and_then(toml::Value::as_table_mut) {
        for patch in patches.iter_mut().map(|(_, v)| v) {
            if let Some(t) = patch.as_table_mut() {
                changed |= dependencies(t, original, mirror, &base)?;
            }
        }
    }
    if let Some(t) = value.get_mut("replace").and_then(toml::Value::as_table_mut) {
        changed |= dependencies(t, original, mirror, &base)?;
    }
    if changed {
        toml::to_string(&value)
            .map(String::into_bytes)
            .map_err(|e| e.to_string())
    } else {
        Ok(bytes.to_vec())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_paths_preserve_dependency_kinds_and_private_workspace_sources() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let original = root.join("project");
        let mirror = root.join("mirror");
        let external = root.join("helper");
        fs::create_dir_all(&original).unwrap();
        fs::create_dir_all(&external).unwrap();
        let text=b"[package]\nname='consumer'\n[lib]\npath='source.rs'\n[dependencies]\nlocal={path='inside'}\nhelper={path='../helper'}\n[target.'cfg(unix)'.build-dependencies]\nhelper={path='../helper'}\n[workspace.dependencies]\nhelper={path='../helper'}\n[patch.crates-io]\nhelper={path='../helper'}\n[package.metadata]\npath='unchanged'\n";
        let value = String::from_utf8(manifest(text, "Cargo.toml", &original, &mirror).unwrap())
            .unwrap()
            .parse::<toml::Table>()
            .unwrap();
        assert_eq!(
            value["dependencies"]["local"]["path"].as_str().unwrap(),
            mirror.join("inside").to_str().unwrap()
        );
        assert_eq!(
            value["dependencies"]["helper"]["path"].as_str().unwrap(),
            external.to_str().unwrap()
        );
        assert_eq!(
            value["workspace"]["dependencies"]["helper"]["path"]
                .as_str()
                .unwrap(),
            external.to_str().unwrap()
        );
        assert_eq!(
            value["target"]["cfg(unix)"]["build-dependencies"]["helper"]["path"]
                .as_str()
                .unwrap(),
            external.to_str().unwrap()
        );
        assert_eq!(
            value["patch"]["crates-io"]["helper"]["path"]
                .as_str()
                .unwrap(),
            external.to_str().unwrap()
        );
        assert_eq!(value["lib"]["path"].as_str(), Some("source.rs"));
        assert_eq!(
            value["package"]["metadata"]["path"].as_str(),
            Some("unchanged")
        );
    }
    #[test]
    fn discovery_is_local_language_specific_and_tolerates_an_unavailable_version() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "# local manifest metadata\n[package]\nname='helper'\nversion.workspace=true\n",
        )
        .unwrap();
        fs::write(root.path().join("src/worker.rs"), "fn run(){}").unwrap();
        let found = input(&root.path().join("src/worker.rs"), &"a".repeat(64)).unwrap();
        assert_eq!(found.package, "helper");
        assert_eq!(found.path, "src/worker.rs");
        assert_eq!(found.version, None);
        assert_eq!(found.root, fs::canonicalize(root.path()).unwrap());
        fs::write(root.path().join("src/source.py"), "def run(): pass").unwrap();
        assert_eq!(
            input(&root.path().join("src/source.py"), &"a".repeat(64))
                .unwrap()
                .language,
            "rust"
        );
        assert!(manifest(b"invalid [[", "Cargo.toml", root.path(), root.path()).is_err());
    }
    #[test]
    fn dependency_manifest_reads_accept_the_boundary_without_truncating_larger_inputs() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("outside.inc");
        fs::write(&path, "pub fn run(){}").unwrap();
        let prefix = "[package]\nname='helper'\n";
        for size in [2048, 128 * 1024] {
            fs::write(
                root.path().join("Cargo.toml"),
                format!("{prefix}{}", " ".repeat(size - prefix.len())),
            )
            .unwrap();
            assert_eq!(input(&path, &"a".repeat(64)).unwrap().package, "helper");
        }
        fs::write(
            root.path().join("Cargo.toml"),
            format!("{prefix}{}", " ".repeat(128 * 1024 + 1 - prefix.len())),
        )
        .unwrap();
        assert!(input(&path, &"a".repeat(64)).is_err());
    }
    #[test]
    fn dependency_translation_normalises_missing_paths_and_rejects_invalid_path_types() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        let mirror = root.path().join("mirror");
        fs::create_dir(&project).unwrap();
        let text = b"[dependencies]\nhelper={path='../missing'}\n";
        let translated =
            String::from_utf8(manifest(text, "Cargo.toml", &project, &mirror).unwrap())
                .unwrap()
                .parse::<toml::Table>()
                .unwrap();
        assert_eq!(
            translated["dependencies"]["helper"]["path"]
                .as_str()
                .unwrap(),
            root.path().join("missing").to_str().unwrap()
        );
        assert!(
            manifest(
                b"[dependencies]\nhelper={path=1}\n",
                "Cargo.toml",
                &project,
                &mirror
            )
            .is_err()
        );
    }
    #[test]
    fn dependency_path_translation_works_when_each_table_kind_is_the_only_changed_input() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        let mirror = root.path().join("mirror");
        fs::create_dir(&project).unwrap();
        for (section, keys) in [
            ("dependencies", vec!["dependencies"]),
            ("dev-dependencies", vec!["dev-dependencies"]),
            ("build-dependencies", vec!["build-dependencies"]),
            ("workspace.dependencies", vec!["workspace", "dependencies"]),
            (
                "target.'cfg(unix)'.dependencies",
                vec!["target", "cfg(unix)", "dependencies"],
            ),
            ("patch.crates-io", vec!["patch", "crates-io"]),
            ("replace", vec!["replace"]),
        ] {
            let text = format!("[{section}]\nhelper={{path='../missing'}}\n");
            let parsed = String::from_utf8(
                manifest(text.as_bytes(), "Cargo.toml", &project, &mirror).unwrap(),
            )
            .unwrap()
            .parse::<toml::Table>()
            .unwrap();
            let mut value = parsed.get(keys[0]).unwrap();
            for key in keys.iter().skip(1) {
                value = value.get(*key).unwrap();
            }
            assert_eq!(
                value["helper"]["path"].as_str().unwrap(),
                root.path().join("missing").to_str().unwrap(),
                "{section}"
            );
        }
    }
}
