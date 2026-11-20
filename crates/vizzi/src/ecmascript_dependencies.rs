//! Installed JavaScript and TypeScript package provenance shared by both providers.

use crate::{
    dependency_sources::{Input, PackageContext},
    review_source::{DirectoryInput, ReadEntry, ReviewInput},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{Duration, UNIX_EPOCH},
};
use vizzi_core::{
    analysis::ECMASCRIPT_LANGUAGES,
    comparison::{EntryKind, ReviewPath},
    model::{ContentId, ModelError},
    policy::{PolicyConfig, SourcePolicy},
};

const MANIFEST_BYTES: usize = 128 * 1024;

pub(crate) fn installed(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .is_ok_and(|relative| relative.iter().any(|part| part == "node_modules"))
}

fn package_part(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn layout_name(directory: &Path) -> Option<String> {
    let name = directory.file_name()?.to_str()?;
    if !package_part(name) {
        return None;
    }
    let parent = directory.parent()?;
    let container = parent.file_name()?.to_str()?;
    if container == "node_modules" {
        return Some(name.to_owned());
    }
    let scope = container.strip_prefix('@')?;
    (package_part(scope) && parent.parent()?.file_name()? == "node_modules")
        .then(|| format!("{container}/{name}"))
}

fn manifest_bytes(package: &Path) -> Result<Vec<u8>, String> {
    let mut directory = DirectoryInput::open(package).map_err(|error| error.to_string())?;
    let manifest =
        ReviewPath::parse(b"package.json".to_vec()).map_err(|error| error.to_string())?;
    let ReadEntry::Present {
        kind: EntryKind::Regular { .. },
        bytes,
    } = directory
        .read(&manifest, MANIFEST_BYTES, &|| false)
        .map_err(|error| error.to_string())?
    else {
        return Err("Installed package manifest is unavailable".into());
    };
    Ok(bytes)
}

pub(crate) fn manifest_id(package: &Path) -> Result<ContentId, String> {
    let bytes = manifest_bytes(package)?;
    Ok(ContentId::derive(&[&bytes]))
}

pub(crate) fn input(workspace: &Path, file: &Path, configuration: &str) -> Result<Input, String> {
    static POLICY: OnceLock<Result<SourcePolicy, ModelError>> = OnceLock::new();
    let workspace = workspace
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let file = file.canonicalize().map_err(|error| error.to_string())?;
    if !installed(&workspace, &file) {
        return Err("Definition is outside the workspace package installation".into());
    }
    let (root, fallback) = file
        .ancestors()
        .skip(1)
        .take_while(|parent| *parent != workspace)
        .find_map(|parent| layout_name(parent).map(|name| (parent, name)))
        .ok_or("Installed package root is unavailable")?;
    if !installed(&workspace, root) {
        return Err("Package root is outside the workspace package installation".into());
    }
    let path = file
        .strip_prefix(root)
        .map_err(|error| error.to_string())?
        .to_str()
        .ok_or("Package source path is not UTF-8")?
        .replace('\\', "/");
    let classification = POLICY
        .get_or_init(|| {
            SourcePolicy::new(
                PolicyConfig::default(),
                ECMASCRIPT_LANGUAGES.iter().map(|lang| (*lang).into()),
            )
        })
        .as_ref()
        .map_err(ToString::to_string)?
        .classify(&path)
        .map_err(|error| error.to_string())?;
    let language = classification
        .as_ref()
        .and_then(|value| value.assigned_language())
        .ok_or("Unsupported package source language")?;
    let bytes = manifest_bytes(root)?;
    let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let manifest: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let label = |key: &str, max: usize| {
        manifest[key]
            .as_str()
            .filter(|value| {
                !value.is_empty() && value.len() <= max && !value.contains(['\0', '\r', '\n'])
            })
            .map(str::to_owned)
    };
    Ok(Input {
        root: root.into(),
        path,
        language: language.into(),
        package: label("name", 256).unwrap_or(fallback),
        version: label("version", 128),
        configuration: configuration.into(),
        mirror_path: None,
        installation: Some(PackageContext {
            manifest: ContentId::derive(&[&bytes]),
            importer: None,
            editor_uri: None,
        }),
    })
}

struct Installation {
    directory: PathBuf,
    modified: Option<Duration>,
}

pub(super) struct Installations(BTreeMap<PathBuf, Installation>);

impl Installations {
    pub(super) fn discover<'a>(
        workspace: &Path,
        files: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, String> {
        let root = workspace
            .canonicalize()
            .map_err(|error| error.to_string())?;
        let mut ancestors = BTreeSet::new();
        for path in files {
            if path.contains([':', '\0', '\\']) {
                return Err("Invalid captured package search path".into());
            }
            ancestors.extend(Path::new(path).ancestors().skip(1));
        }
        let mut links = BTreeMap::new();
        for ancestor in ancestors {
            let relative = ancestor.join("node_modules");
            let absolute = root.join(&relative);
            if let Ok(directory) = absolute.canonicalize()
                && installed(&root, &directory)
                && let Ok(metadata) = directory.metadata()
                && metadata.is_dir()
            {
                links.insert(
                    relative,
                    Installation {
                        directory,
                        modified: metadata
                            .modified()
                            .ok()
                            .and_then(|time| time.duration_since(UNIX_EPOCH).ok()),
                    },
                );
            }
        }
        Ok(Self(links))
    }

    pub(super) fn fingerprint(&self, hash: &mut blake3::Hasher) {
        hash.update(b"ecmascript-installations-v1");
        for (local, installation) in &self.0 {
            for path in [local, &installation.directory] {
                let bytes = path.as_os_str().as_encoded_bytes();
                hash.update(&(bytes.len() as u64).to_le_bytes());
                hash.update(bytes);
            }
            if let Some(time) = installation.modified {
                hash.update(&[1]);
                hash.update(&time.as_nanos().to_le_bytes());
            } else {
                hash.update(&[0]);
            }
        }
    }

    pub(super) fn link(&self, mirror: &Path) -> Result<(), String> {
        for (relative, installation) in &self.0 {
            let target = mirror.join(relative);
            let parent = target.parent().ok_or("Package link needs a parent")?;
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&installation.directory, target)
                .map_err(|error| error.to_string())?;
            #[cfg(windows)]
            if let Err(error) = std::os::windows::fs::symlink_dir(&installation.directory, target) {
                if error.raw_os_error() != Some(1314) {
                    return Err(error.to_string());
                }
            }
        }
        Ok(())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn pnpm_and_scoped_names_use_the_canonical_installed_package() {
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path().canonicalize().unwrap();
        let package = root.join("node_modules/.pnpm/@demo+view@2.3.4/node_modules/@demo/view");
        fs::create_dir_all(package.join("src")).unwrap();
        fs::write(
            package.join("package.json"),
            r#"{"name":"@demo/view","version":"2.3.4"}"#,
        )
        .unwrap();
        fs::write(
            package.join("src/view.tsx"),
            "export const View = () => <div />;",
        )
        .unwrap();
        let scope = root.join("node_modules/@demo");
        fs::create_dir_all(&scope).unwrap();
        symlink(&package, scope.join("view")).unwrap();
        let input = input(&root, &scope.join("view/src/view.tsx"), &"a".repeat(64)).unwrap();
        assert_eq!(input.root, package);
        assert_eq!(input.path, "src/view.tsx");
        assert_eq!(input.language, "tsx");
        assert_eq!(input.package, "@demo/view");
        assert_eq!(input.version.as_deref(), Some("2.3.4"));
    }

    #[test]
    fn installation_admission_refuses_escapes_and_non_source_files() {
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = workspace.path().canonicalize().unwrap();
        let package = root.join("node_modules/helper");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("package.json"), r#"{"name":"helper"}"#).unwrap();
        let escaped = outside.path().join("node_modules/escaped");
        fs::create_dir_all(&escaped).unwrap();
        fs::write(escaped.join("package.json"), r#"{"name":"escaped"}"#).unwrap();
        fs::write(escaped.join("entry.ts"), "export function outside() {}").unwrap();
        symlink(escaped.join("entry.ts"), package.join("entry.ts")).unwrap();
        let cache = root.join("node_modules/.cache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("package.json"), r#"{"name":"hidden"}"#).unwrap();
        fs::write(cache.join("entry.ts"), "export function hidden() {}").unwrap();
        for path in [
            package.join("entry.ts"),
            package.join("package.json"),
            cache.join("entry.ts"),
        ] {
            assert!(input(&root, &path, &"a".repeat(64)).is_err(), "{path:?}");
        }
    }
}
