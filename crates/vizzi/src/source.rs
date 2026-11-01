//! Read-only directory snapshots, independent of source-control systems.

use std::{
    io,
    path::{Path, PathBuf},
};
use vizzi_core::model::WorkspaceId;

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
    pub directories: Vec<PathBuf>,
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
        let mut capture = Capture {
            files: Vec::new(),
            directories: Vec::new(),
        };
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
            capture.directories.push(directory);
        }
        capture.files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(capture)
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
}
