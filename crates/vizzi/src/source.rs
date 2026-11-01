//! Read-only directory snapshots, independent of source-control systems.

use std::{
    collections::BTreeSet,
    io,
    path::{Component, Path, PathBuf},
};
use vizzi_core::model::WorkspaceId;

/// Input verification scope; paths are hints relative to the canonical workspace.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CaptureScope {
    /// Rediscover and verify every supported source file.
    #[default]
    Reconcile,
    /// Verify these candidates while retaining other captured documents.
    Paths(BTreeSet<PathBuf>),
}

/// Maximum retained path hints before promoting work to a full reconciliation.
pub const MAX_PATH_HINTS: usize = 4096;

impl CaptureScope {
    pub(crate) fn merge(&mut self, next: Self) {
        match (&mut *self, next) {
            (Self::Paths(paths), Self::Paths(next)) => {
                paths.extend(next);
            }
            (current, _) => *current = Self::Reconcile,
        }
        if matches!(self, Self::Paths(paths) if paths.len() > MAX_PATH_HINTS) {
            *self = Self::Reconcile;
        }
    }
}

/// One captured file awaiting analysis or reuse.
pub struct CapturedFile {
    /// Normalized workspace-relative path.
    pub path: String,
    /// Frozen bytes read during discovery.
    pub bytes: Vec<u8>,
}
/// Captured Rust inputs and the directories to observe for changes.
pub struct Capture {
    /// Rust source files in path order.
    pub files: Vec<CapturedFile>,
    /// Traversed directories, excluding root build/VCS dependencies.
    pub directories: Option<Vec<PathBuf>>,
    /// Paths removed from a partial capture's baseline; empty for reconciliation.
    pub removed: Vec<String>,
}
/// Initial source adapter: a canonical local directory with no SCM dependency.
pub struct Directory {
    root: PathBuf,
    workspace: WorkspaceId,
}
impl Directory {
    /// Opens a directory; never executes project build scripts.
    pub fn open(root: &Path) -> io::Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(io::Error::other("workspace root is not a directory"));
        }
        let workspace = WorkspaceId::derive(&[root.as_os_str().as_encoded_bytes()]);
        Ok(Self { root, workspace })
    }
    /// Returns the canonical root.
    pub fn root(&self) -> &Path {
        &self.root
    }
    /// Returns the directory adapter's workspace identity.
    pub fn workspace(&self) -> &WorkspaceId {
        &self.workspace
    }
    /// Tests whether a native event could affect discovered source.
    pub fn relevant(&self, path: &Path) -> bool {
        path.strip_prefix(&self.root).is_ok_and(|relative| {
            let first = relative.components().next().map(|part| part.as_os_str());
            !first.is_some_and(|part| {
                [".git", "target", "node_modules"]
                    .iter()
                    .any(|name| part == *name)
            }) && (path.extension().is_some_and(|extension| extension == "rs")
                || path.extension().is_none())
        })
    }
    /// Reads Rust files without following symlinks; errors preserve the prior snapshot.
    pub fn capture(&self) -> io::Result<Capture> {
        self.check_root()?;
        let mut capture = Capture {
            files: Vec::new(),
            directories: None,
            removed: Vec::new(),
        };
        let mut directories = Vec::new();
        let mut pending = vec![self.root.clone()];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(&directory)? {
                let entry = entry?;
                let kind = entry.file_type()?;
                let path = entry.path();
                if kind.is_dir() {
                    if directory == self.root
                        && [".git", "target", "node_modules"]
                            .iter()
                            .any(|name| entry.file_name() == *name)
                    {
                        continue;
                    }
                    pending.push(path);
                } else if kind.is_file()
                    && path.extension().is_some_and(|extension| extension == "rs")
                {
                    let relative = path.strip_prefix(&self.root).map_err(io::Error::other)?;
                    let parts: Result<Vec<_>, _> = relative
                        .components()
                        .map(|component| {
                            component
                                .as_os_str()
                                .to_str()
                                .ok_or_else(|| io::Error::other("source path is not UTF-8"))
                        })
                        .collect();
                    capture.files.push(CapturedFile {
                        path: parts?.join("/"),
                        bytes: std::fs::read(&path)?,
                    });
                }
            }
            directories.push(directory);
        }
        capture.files.sort_by(|a, b| a.path.cmp(&b.path));
        capture.directories = Some(directories);
        Ok(capture)
    }

    /// Reopens hinted files; ambiguous paths fall back to complete discovery.
    pub fn capture_paths(&self, paths: &BTreeSet<PathBuf>) -> io::Result<Capture> {
        self.check_root()?;
        let mut capture = Capture {
            files: Vec::new(),
            directories: None,
            removed: Vec::new(),
        };
        let mut reads = Vec::new();
        for path in paths {
            let key = match self.relative_key(path) {
                Ok(Some(key)) => key,
                Ok(None) => continue,
                Err(_) => return self.capture(),
            };
            let mut absolute = self.root.clone();
            let components: Vec<_> = path.components().collect();
            for (index, component) in components.iter().enumerate() {
                absolute.push(component.as_os_str());
                let leaf = index + 1 == components.len();
                let metadata = match std::fs::symlink_metadata(&absolute) {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == io::ErrorKind::NotFound && leaf => {
                        if Self::source_path(path) {
                            capture.removed.push(key.clone());
                        }
                        break;
                    }
                    Err(_) => return self.capture(),
                };
                if !leaf {
                    if !metadata.file_type().is_dir() {
                        return self.capture();
                    }
                } else if metadata.file_type().is_dir() {
                    return self.capture();
                } else if Self::source_path(path) {
                    if metadata.file_type().is_file() {
                        reads.push((key.clone(), absolute.clone()));
                    } else {
                        capture.removed.push(key.clone());
                    }
                }
            }
        }
        for (path, absolute) in reads {
            capture.files.push(CapturedFile {
                path,
                bytes: std::fs::read(absolute)?,
            });
        }
        Ok(capture)
    }

    fn check_root(&self) -> io::Result<()> {
        if !std::fs::symlink_metadata(&self.root)?.file_type().is_dir() {
            return Err(io::Error::other("workspace root is no longer a directory"));
        }
        Ok(())
    }
    pub(crate) fn source_path(path: &Path) -> bool {
        path.extension().is_some_and(|extension| extension == "rs")
    }
    pub(crate) fn relative_key(&self, path: &Path) -> io::Result<Option<String>> {
        let mut parts = Vec::new();
        for component in path.components() {
            let Component::Normal(part) = component else {
                return Err(io::Error::other("expected a relative source hint"));
            };
            parts.push(
                part.to_str()
                    .ok_or_else(|| io::Error::other("source path is not UTF-8"))?,
            );
        }
        if parts.is_empty() {
            return Err(io::Error::other("empty source hint"));
        }
        if [".git", "target", "node_modules"].contains(&parts[0]) {
            return Ok(None);
        }
        Ok(Some(parts.join("/")))
    }
    pub(crate) fn event_path(&self, path: &Path) -> Option<PathBuf> {
        let relative = path.strip_prefix(&self.root).ok()?;
        match self.relative_key(relative) {
            Ok(Some(_)) => Some(relative.to_path_buf()),
            Ok(None) => None,
            Err(_) => Some(PathBuf::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_excludes_root_build_trees_but_preserves_nested_source() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["src", "target", ".git", "node_modules", "drivers/target"] {
            std::fs::create_dir_all(temp.path().join(name)).unwrap();
        }
        for name in [
            "src/main.rs",
            "target/build.rs",
            ".git/a.rs",
            "node_modules/b.rs",
            "drivers/target/real.rs",
            "src/lpc.lpc",
        ] {
            std::fs::write(temp.path().join(name), "fn f() {}").unwrap();
        }
        let directory = Directory::open(temp.path()).unwrap();
        let capture = directory.capture().unwrap();
        assert_eq!(
            capture
                .files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            ["drivers/target/real.rs", "src/main.rs"]
        );
        assert_eq!(capture.files[0].bytes, b"fn f() {}");
        assert!(!directory.relevant(&temp.path().join("target/build.rs")));
        assert!(directory.relevant(&temp.path().join("drivers/target/real.rs")));
        assert!(!directory.relevant(Path::new("/elsewhere/a.rs")));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(temp.path().join("src"), temp.path().join("linked"))
                .unwrap();
            std::os::unix::fs::symlink(
                temp.path().join("src/main.rs"),
                temp.path().join("link.rs"),
            )
            .unwrap();
            assert_eq!(directory.capture().unwrap().files.len(), 2);
        }
    }

    #[test]
    fn path_capture_only_returns_candidates_and_detects_removal() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("a.rs"), "fn a() {}").unwrap();
        std::fs::write(temp.path().join("b.rs"), "fn b() {}").unwrap();
        let directory = Directory::open(temp.path()).unwrap();
        let capture = directory
            .capture_paths(&BTreeSet::from(["a.rs".into(), "gone.rs".into()]))
            .unwrap();
        assert!(capture.directories.is_none());
        assert_eq!(capture.files.len(), 1);
        assert_eq!(capture.files[0].path, "a.rs");
        assert_eq!(capture.files[0].bytes, b"fn a() {}");
        assert_eq!(capture.removed, ["gone.rs"]);
    }

    #[test]
    fn excluded_hints_are_silent_and_hint_capacity_promotes_to_reconciliation() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Directory::open(temp.path()).unwrap();
        std::fs::create_dir(temp.path().join("target")).unwrap();
        std::fs::write(temp.path().join("target/a.rs"), "fn ignored() {}").unwrap();
        let capture = dir
            .capture_paths(&BTreeSet::from(["target/a.rs".into()]))
            .unwrap();
        assert!(capture.files.is_empty());
        assert!(capture.removed.is_empty());
        assert!(capture.directories.is_none());
        let mut scope = CaptureScope::Paths(
            (0..MAX_PATH_HINTS)
                .map(|i| format!("f{i}.rs").into())
                .collect(),
        );
        scope.merge(CaptureScope::Paths(["f0.rs".into()].into()));
        assert!(matches!(&scope, CaptureScope::Paths(paths) if paths.len() == MAX_PATH_HINTS));
        scope.merge(CaptureScope::Paths(["extra.rs".into()].into()));
        assert_eq!(scope, CaptureScope::Reconcile);
        scope.merge(CaptureScope::Paths(["a.rs".into()].into()));
        assert_eq!(scope, CaptureScope::Reconcile);
    }
    #[test]
    fn directory_and_invalid_hints_reconcile_without_reading_outside_root() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join("nested")).unwrap();
        std::fs::write(temp.path().join("nested/a.rs"), "fn a() {}").unwrap();
        let directory = Directory::open(temp.path()).unwrap();
        for path in ["nested", "../outside.rs", "/outside.rs", ""] {
            let capture = directory
                .capture_paths(&BTreeSet::from([path.into()]))
                .unwrap();
            assert!(capture.directories.is_some());
            assert_eq!(capture.files[0].path, "nested/a.rs");
        }
    }

    #[test]
    fn missing_and_non_directory_ancestors_require_discovery() {
        let root = tempfile::tempdir().unwrap();
        let directory = Directory::open(root.path()).unwrap();
        std::fs::write(root.path().join("live.rs"), "fn live() {}").unwrap();
        std::fs::write(root.path().join("file"), "plain").unwrap();
        for hint in ["missing/a.rs", "file/a.rs"] {
            let capture = directory
                .capture_paths(&BTreeSet::from([hint.into()]))
                .unwrap();
            assert!(capture.directories.is_some());
            assert_eq!(capture.files[0].path, "live.rs");
        }
    }
    #[cfg(unix)]
    #[test]
    fn replaced_root_and_unreadable_parents_cannot_be_treated_as_leaf_deletions() {
        use std::os::unix::fs::PermissionsExt;
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("root");
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir(&root).unwrap();
        std::fs::write(outside.path().join("secret.rs"), "secret").unwrap();
        let directory = Directory::open(&root).unwrap();
        std::fs::remove_dir(&root).unwrap();
        std::os::unix::fs::symlink(outside.path(), &root).unwrap();
        assert!(directory.capture().is_err());
        assert!(
            directory
                .capture_paths(&["secret.rs".into()].into())
                .is_err()
        );
        std::fs::remove_file(&root).unwrap();
        std::fs::create_dir(&root).unwrap();
        let locked = root.join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::write(locked.join("a.rs"), "fn a() {}").unwrap();
        let permissions = std::fs::metadata(&locked).unwrap().permissions();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o0)).unwrap();
        let inaccessible = std::fs::symlink_metadata(locked.join("a.rs")).is_err();
        let result = directory.capture_paths(&["locked/a.rs".into()].into());
        std::fs::set_permissions(&locked, permissions).unwrap();
        if inaccessible {
            assert!(result.is_err());
        }
    }
    #[cfg(unix)]
    #[test]
    fn path_capture_excludes_leaf_symlinks_and_reconciles_changed_ancestors() {
        let source = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.rs"), "fn outside() {}").unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret.rs"),
            source.path().join("leaf.rs"),
        )
        .unwrap();
        std::os::unix::fs::symlink(outside.path(), source.path().join("parent")).unwrap();
        let directory = Directory::open(source.path()).unwrap();
        let leaf = directory
            .capture_paths(&BTreeSet::from(["leaf.rs".into()]))
            .unwrap();
        assert!(leaf.files.is_empty());
        assert_eq!(leaf.removed, ["leaf.rs"]);
        assert!(leaf.directories.is_none());
        let parent = directory
            .capture_paths(&BTreeSet::from(["parent/secret.rs".into()]))
            .unwrap();
        assert!(parent.files.is_empty());
        assert!(parent.directories.is_some());
    }
}
