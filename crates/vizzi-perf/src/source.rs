//! Filesystem input for content-scan measurements; symlinks are not followed.

use crate::snapshot::{Fingerprint, Manifest};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

/// Capture all regular files except root `.git`, `target`, and `node_modules` directories.
pub fn capture(root: &Path) -> io::Result<Manifest> {
    let mut fingerprints = BTreeMap::new();
    for relative in files(root)? {
        fingerprints.insert(relative.clone(), fingerprint_file(&root.join(&relative))?);
    }
    Ok(Manifest::new(fingerprints))
}

/// Discover regular files without following symlinks; paths are relative and sorted.
pub fn files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut pending = vec![root.to_path_buf()];
    let mut found = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                if directory == root
                    && [".git", "target", "node_modules"]
                        .iter()
                        .any(|name| entry.file_name() == *name)
                {
                    continue;
                }
                pending.push(entry.path());
            } else if kind.is_file() {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(io::Error::other)?
                    .to_path_buf();
                found.push(relative);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Hash a complete file without loading the entire contents into memory.
pub fn fingerprint_file(path: &Path) -> io::Result<Fingerprint> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(fs::File::open(path)?)?;
    Ok(Fingerprint {
        hash: hasher.finalize(),
        bytes: hasher.count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn captures_nested_source_and_binary_contents_without_skipping_nested_target() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("drivers/target")).unwrap();
        fs::create_dir(root.path().join("target")).unwrap();
        fs::write(root.path().join("drivers/target/code.c"), [0, 255, 7]).unwrap();
        fs::write(root.path().join("target/ignored"), b"build output").unwrap();
        let captured = capture(root.path()).unwrap();
        let expected = Manifest::new(std::collections::BTreeMap::from([(
            "drivers/target/code.c".into(),
            crate::snapshot::Fingerprint::of(&[0, 255, 7]),
        )]));
        assert_eq!(captured, expected);
    }

    #[test]
    fn rescan_detects_same_length_edits_additions_and_deletions() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("edited"), b"old").unwrap();
        fs::write(root.path().join("removed"), b"gone").unwrap();
        let before = capture(root.path()).unwrap();
        fs::write(root.path().join("edited"), b"new").unwrap();
        fs::write(root.path().join("added"), b"here").unwrap();
        fs::remove_file(root.path().join("removed")).unwrap();
        let after = capture(root.path()).unwrap();
        assert_eq!(after.changes_from(&before).len(), 3);
    }

    #[test]
    fn missing_root_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        assert!(capture(&root.path().join("missing")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_a_symlink_outside_the_root() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
        assert_eq!(capture(root.path()).unwrap(), Manifest::default());
    }
}
