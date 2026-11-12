//! Read-only directory snapshots, independent of source-control systems.

use std::sync::Arc;
use std::{
    collections::BTreeSet,
    io,
    path::{Component, Path, PathBuf},
};
use vizzi_core::{
    analysis::{Language, Languages},
    capture::CapturePlan,
    model::{CapturedSource, Snapshot, WorkspaceId},
    policy::{Classification, PolicyConfig, SourcePolicy},
};

pub(crate) fn workspace_root(root: &Path) -> io::Result<(PathBuf, WorkspaceId)> {
    let root = root.canonicalize()?;
    if !root.is_dir() {
        return Err(io::Error::other("workspace root is not a directory"));
    }
    let workspace = WorkspaceId::derive(&[root.as_os_str().as_encoded_bytes()]);
    Ok((root, workspace))
}

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
    /// Verified immutable bytes and line index, shared with resulting documents.
    pub source: Arc<CapturedSource>,
    /// Frozen source-selection metadata.
    pub classification: Classification,
}
/// Captured supported inputs and the directories to observe for changes.
pub struct Capture {
    /// Supported source files in path order.
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
    languages: Arc<Languages>,
    plan: CapturePlan,
    spool: Option<Arc<crate::source_spool::SourceSpool>>,
}
impl Directory {
    pub(crate) fn maintenance(&self) -> Option<Arc<dyn crate::maintenance::SourceMaintenance>> {
        self.spool
            .as_ref()
            .map(|spool| spool.clone() as Arc<dyn crate::maintenance::SourceMaintenance>)
    }
    /// Opens a directory; never executes project build scripts.
    pub fn open(root: &Path) -> io::Result<Self> {
        Self::with_languages(
            root,
            Arc::new(Languages::builtin().map_err(io::Error::other)?),
        )
    }
    /// Enables private source backing outside the analysed directory.
    pub fn with_spool(mut self, spool: Arc<crate::source_spool::SourceSpool>) -> io::Result<Self> {
        if spool.directory().starts_with(&self.root) {
            return Err(io::Error::other(
                "source spool must be outside analysed workspace",
            ));
        }
        self.spool = Some(spool);
        Ok(self)
    }
    /// Opens a directory with an explicit shared source and analysis policy.
    pub fn with_languages(root: &Path, languages: Arc<Languages>) -> io::Result<Self> {
        Self::with_policy(root, PolicyConfig::default(), languages)
    }
    /// Opens a directory with a portable source policy and registered analyzers.
    pub fn with_policy(
        root: &Path,
        config: PolicyConfig,
        languages: Arc<Languages>,
    ) -> io::Result<Self> {
        let plan = CapturePlan::new(config, &languages).map_err(io::Error::other)?;
        let (root, workspace) = workspace_root(root)?;
        Ok(Self {
            root,
            workspace,
            languages,
            plan,
            spool: None,
        })
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
            self.relative_key(relative).is_ok_and(|key| key.is_some())
                && (self.source_path(relative) || path.extension().is_none())
        })
    }
    /// Reads supported files without following symlinks; errors preserve the prior snapshot.
    pub fn capture(&self) -> io::Result<Capture> {
        self.capture_full(None, &|| false)?
            .ok_or_else(|| io::Error::other("uncancelled capture was interrupted"))
    }
    /// Reads and hashes current bytes, sharing matching source from an old observation.
    /// Cancellation abandons all partial work; metadata never establishes reuse.
    pub fn capture_reusing(
        &self,
        previous: Option<&Snapshot>,
        scope: &CaptureScope,
        cancelled: &dyn Fn() -> bool,
    ) -> io::Result<Option<Capture>> {
        match scope {
            CaptureScope::Reconcile => self.capture_full(previous, cancelled),
            CaptureScope::Paths(paths) => self.capture_hints(paths, previous, cancelled),
        }
    }
    fn capture_full(
        &self,
        previous: Option<&Snapshot>,
        cancelled: &dyn Fn() -> bool,
    ) -> io::Result<Option<Capture>> {
        if cancelled() {
            return Ok(None);
        }
        self.check_root()?;
        let mut capture = Capture {
            files: Vec::new(),
            directories: None,
            removed: Vec::new(),
        };
        let mut directories = Vec::new();
        let mut pending = vec![self.root.clone()];
        while let Some(directory) = pending.pop() {
            if cancelled() {
                return Ok(None);
            }
            for entry in std::fs::read_dir(&directory)? {
                if cancelled() {
                    return Ok(None);
                }
                let entry = entry?;
                let kind = entry.file_type()?;
                let path = entry.path();
                if !kind.is_dir() && !kind.is_file() {
                    continue;
                }
                let relative = path.strip_prefix(&self.root).map_err(io::Error::other)?;
                let Some(key) = self.relative_key(relative)? else {
                    continue;
                };
                if kind.is_dir() {
                    pending.push(path);
                } else if let Some(classification) = self
                    .plan
                    .policy()
                    .classify(&key)
                    .map_err(io::Error::other)?
                {
                    let Some(source) = self.read_source(&key, &path, previous, cancelled)? else {
                        return Ok(None);
                    };
                    capture.files.push(CapturedFile {
                        path: key,
                        source,
                        classification,
                    });
                }
            }
            directories.push(directory);
        }
        capture.files.sort_by(|a, b| a.path.cmp(&b.path));
        capture.directories = Some(directories);
        Ok((!cancelled()).then_some(capture))
    }

    /// Reopens hinted files; ambiguous paths fall back to complete discovery.
    pub fn capture_paths(&self, paths: &BTreeSet<PathBuf>) -> io::Result<Capture> {
        self.capture_hints(paths, None, &|| false)?
            .ok_or_else(|| io::Error::other("uncancelled capture was interrupted"))
    }
    fn capture_hints(
        &self,
        paths: &BTreeSet<PathBuf>,
        previous: Option<&Snapshot>,
        cancelled: &dyn Fn() -> bool,
    ) -> io::Result<Option<Capture>> {
        if cancelled() {
            return Ok(None);
        }
        self.check_root()?;
        let mut capture = Capture {
            files: Vec::new(),
            directories: None,
            removed: Vec::new(),
        };
        let mut reads = Vec::new();
        for path in paths {
            if cancelled() {
                return Ok(None);
            }
            let key = match self.relative_key(path) {
                Ok(Some(key)) => key,
                Ok(None) => continue,
                Err(_) => return self.capture_full(previous, cancelled),
            };
            let mut absolute = self.root.clone();
            let components: Vec<_> = path.components().collect();
            for (index, component) in components.iter().enumerate() {
                if cancelled() {
                    return Ok(None);
                }
                absolute.push(component.as_os_str());
                let leaf = index + 1 == components.len();
                let metadata = match std::fs::symlink_metadata(&absolute) {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == io::ErrorKind::NotFound && leaf => {
                        if self.source_path(path) {
                            capture.removed.push(key.clone());
                        }
                        break;
                    }
                    Err(_) => return self.capture_full(previous, cancelled),
                };
                if !leaf {
                    if !metadata.file_type().is_dir() {
                        return self.capture_full(previous, cancelled);
                    }
                } else if metadata.file_type().is_dir() {
                    return self.capture_full(previous, cancelled);
                } else if self.source_path(path) {
                    if metadata.file_type().is_file() {
                        if let Some(classification) = self
                            .plan
                            .policy()
                            .classify(&key)
                            .map_err(io::Error::other)?
                        {
                            reads.push((key.clone(), absolute.clone(), classification));
                        }
                    } else {
                        capture.removed.push(key.clone());
                    }
                }
            }
        }
        for (path, absolute, classification) in reads {
            let Some(source) = self.read_source(&path, &absolute, previous, cancelled)? else {
                return Ok(None);
            };
            capture.files.push(CapturedFile {
                path,
                source,
                classification,
            });
        }
        Ok((!cancelled()).then_some(capture))
    }
    fn read_source(
        &self,
        key: &str,
        path: &Path,
        previous: Option<&Snapshot>,
        cancelled: &dyn Fn() -> bool,
    ) -> io::Result<Option<Arc<CapturedSource>>> {
        if cancelled() {
            return Ok(None);
        }
        let old = previous
            .and_then(|snapshot| snapshot.file_by_path(key))
            .map(|file| file.captured());
        if let Some(spool) = &self.spool {
            let mut file = std::fs::File::open(path)?;
            return spool
                .capture(&mut file, old, cancelled)
                .map_err(io::Error::other);
        }
        let bytes = std::fs::read(path)?;
        if cancelled() {
            return Ok(None);
        }
        let source = CapturedSource::capture(bytes, old);
        if cancelled() {
            return Ok(None);
        }
        Ok((!cancelled()).then_some(source))
    }

    fn check_root(&self) -> io::Result<()> {
        if !std::fs::symlink_metadata(&self.root)?.file_type().is_dir() {
            return Err(io::Error::other("workspace root is no longer a directory"));
        }
        Ok(())
    }
    /// Returns the frozen source policy.
    pub fn policy(&self) -> &SourcePolicy {
        self.plan.policy()
    }
    /// Returns the frozen policy/context validation plan.
    pub fn plan(&self) -> &CapturePlan {
        &self.plan
    }
    pub(crate) fn source_path(&self, path: &Path) -> bool {
        self.classification(path).ok().flatten().is_some()
    }
    pub(crate) fn classification(&self, path: &Path) -> io::Result<Option<Classification>> {
        let path = if path.is_absolute() {
            path.strip_prefix(&self.root).map_err(io::Error::other)?
        } else {
            path
        };
        match self.relative_key(path)? {
            Some(key) => self.plan.policy().classify(&key).map_err(io::Error::other),
            None => Ok(None),
        }
    }
    pub(crate) fn language(&self, classification: &Classification) -> Option<&Language> {
        self.languages.select(classification.assigned_language()?)
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
        if self
            .plan
            .policy()
            .excludes(&parts.join("/"))
            .map_err(io::Error::other)?
        {
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
        assert_eq!(
            capture.files[0]
                .source
                .read_all(usize::MAX)
                .unwrap()
                .bytes(),
            b"fn f() {}"
        );
        assert!(!directory.relevant(&directory.root().join("target/build.rs")));
        assert!(directory.relevant(&directory.root().join("drivers/target/real.rs")));
        assert!(!directory.relevant(Path::new("/elsewhere/a.rs")));
        assert!(!directory.relevant(&directory.root().join("notes.txt")));
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
        assert_eq!(
            capture.files[0]
                .source
                .read_all(usize::MAX)
                .unwrap()
                .bytes(),
            b"fn a() {}"
        );
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
