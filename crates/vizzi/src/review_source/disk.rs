//! Descriptor-relative, no-follow reads of review inputs.

use super::*;
use std::path::Path;

#[cfg(unix)]
const DIR_FLAGS: rustix::fs::OFlags = rustix::fs::OFlags::RDONLY
    .union(rustix::fs::OFlags::CLOEXEC)
    .union(rustix::fs::OFlags::DIRECTORY)
    .union(rustix::fs::OFlags::NOFOLLOW);
#[cfg(unix)]
const FILE_FLAGS: rustix::fs::OFlags = rustix::fs::OFlags::RDONLY
    .union(rustix::fs::OFlags::CLOEXEC)
    .union(rustix::fs::OFlags::NOFOLLOW)
    .union(rustix::fs::OFlags::NONBLOCK);

/// Raw directory input; discovery excludes the root Git control directory.
pub struct DirectoryInput {
    root: std::path::PathBuf,
    workspace: WorkspaceId,
    #[cfg(unix)]
    directory: std::fs::File,
}

impl DirectoryInput {
    #[cfg(unix)]
    fn check_root(&self) -> Result<(), CaptureError> {
        use std::os::unix::fs::MetadataExt;
        let current = std::fs::symlink_metadata(&self.root)?;
        let held = self.directory.metadata()?;
        if (current.dev(), current.ino()) != (held.dev(), held.ino()) {
            return Err(CaptureError::Unsupported(
                "workspace root changed during capture",
            ));
        }
        Ok(())
    }
    /// Opens a canonical workspace root without executing project code.
    pub fn open(root: &Path) -> Result<Self, CaptureError> {
        let (root, workspace) = crate::source::workspace_root(root)?;
        #[cfg(unix)]
        let directory = std::fs::File::open(&root)?;
        Ok(Self {
            root,
            workspace,
            #[cfg(unix)]
            directory,
        })
    }
    /// Returns the canonical storage root; it does not define revision input identity.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl ReviewInput for DirectoryInput {
    fn workspace(&self) -> &WorkspaceId {
        &self.workspace
    }
    fn paths(
        &mut self,
        limits: CaptureLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<ReviewPath>, CaptureError> {
        #[cfg(unix)]
        {
            self.discover(limits, cancelled)
        }
        #[cfg(not(unix))]
        {
            let _ = (limits, cancelled);
            Err(CaptureError::Unsupported(
                "no-follow review filesystem reads on this platform",
            ))
        }
    }
    fn read(
        &mut self,
        path: &ReviewPath,
        limit: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ReadEntry, CaptureError> {
        check(cancelled)?;
        #[cfg(unix)]
        {
            self.check_root()?;
            match self.read_unix(path, limit, cancelled) {
                Err(CaptureError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                    Ok(ReadEntry::Absent)
                }
                Err(CaptureError::Io(_)) => Ok(ReadEntry::Unobserved(Unavailability::Unreadable)),
                result => result,
            }
        }
        #[cfg(not(unix))]
        {
            let _ = (path, limit);
            Err(CaptureError::Unsupported(
                "no-follow review filesystem reads on this platform",
            ))
        }
    }
    fn notices(&self) -> Vec<CaptureNotice> {
        vec![]
    }
}

#[cfg(unix)]
impl DirectoryInput {
    fn parent(
        &self,
        path: &ReviewPath,
    ) -> Result<(std::fs::File, std::ffi::OsString), CaptureError> {
        self.check_root()?;
        use rustix::fs::{Mode, openat};
        use std::os::unix::ffi::OsStrExt;
        let mut parts = path.bytes().split(|byte| *byte == b'/').peekable();
        let mut parent = self.directory.try_clone()?;
        while let Some(part) = parts.next() {
            let name = std::ffi::OsStr::from_bytes(part);
            if parts.peek().is_none() {
                return Ok((parent, name.to_owned()));
            }
            parent = openat(&parent, name, DIR_FLAGS, Mode::empty())?.into();
        }
        Err(CaptureError::Unsupported("empty native path"))
    }
    fn discover(
        &self,
        limits: CaptureLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<ReviewPath>, CaptureError> {
        use rustix::fs::{AtFlags, Dir, FileType, Mode, openat, statat};
        let mut pending = vec![(vec![], self.directory.try_clone()?)];
        let mut paths = vec![];
        let mut names = 0usize;
        let mut discovery_bytes = 0usize;
        self.check_root()?;
        while let Some((prefix, directory)) = pending.pop() {
            check(cancelled)?;
            for entry in Dir::read_from(&directory)? {
                check(cancelled)?;
                let entry = entry?;
                let name = entry.file_name();
                if matches!(name.to_bytes(), b"." | b".." | b".git") {
                    continue;
                }
                names += 1;
                if names > limits.max_paths {
                    return Err(CaptureError::Limit("candidate count"));
                }
                let mut bytes = prefix.clone();
                if !bytes.is_empty() {
                    bytes.push(b'/');
                }
                bytes.extend_from_slice(name.to_bytes());
                discovery_bytes = discovery_bytes
                    .checked_add(bytes.len())
                    .ok_or(CaptureError::Limit("discovery bytes"))?;
                if discovery_bytes > limits.max_discovery_bytes {
                    return Err(CaptureError::Limit("discovery bytes"));
                }
                let kind = FileType::from_raw_mode(
                    statat(&directory, name, AtFlags::SYMLINK_NOFOLLOW)?.st_mode,
                );
                if kind == FileType::Directory {
                    let child = openat(&directory, name, DIR_FLAGS, Mode::empty())?;
                    pending.push((bytes, child.into()));
                } else {
                    paths.push(ReviewPath::parse(bytes)?);
                }
            }
        }
        Ok(paths)
    }
    fn read_unix(
        &self,
        path: &ReviewPath,
        limit: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ReadEntry, CaptureError> {
        use rustix::fs::{AtFlags, FileType, Mode, openat, readlinkat, statat};
        use std::os::unix::fs::MetadataExt;
        let (parent, name) = self.parent(path)?;
        let stat = statat(&parent, &name, AtFlags::SYMLINK_NOFOLLOW)?;
        match FileType::from_raw_mode(stat.st_mode) {
            FileType::Symlink => {
                let bytes = readlinkat(&parent, &name, Vec::new())?.into_bytes();
                if bytes.len() > limit {
                    return Err(CaptureError::Limit("content bytes"));
                }
                Ok(ReadEntry::Present {
                    kind: EntryKind::Symlink,
                    bytes,
                })
            }
            FileType::RegularFile => {
                let mut file: std::fs::File =
                    openat(&parent, &name, FILE_FLAGS, Mode::empty())?.into();
                let before = file.metadata()?;
                if !before.is_file() {
                    return Ok(ReadEntry::Unobserved(Unavailability::Changing));
                }
                if before.len() > limit as u64 {
                    return Err(CaptureError::Limit("content bytes"));
                }
                let bytes = match read_bytes(&mut file, limit, cancelled) {
                    Err(CaptureError::Io(_)) => {
                        return Ok(ReadEntry::Unavailable {
                            kind: regular_kind(before.mode()),
                            reason: Unavailability::Unreadable,
                        });
                    }
                    result => result?,
                };
                let after = file.metadata()?;
                let current = statat(&parent, &name, AtFlags::SYMLINK_NOFOLLOW)?;
                if !FileStamp::of(&before).verifies(
                    &FileStamp::of(&after),
                    (i128::from(current.st_dev), i128::from(current.st_ino)),
                    bytes.len() as u64,
                ) {
                    return Ok(ReadEntry::Unobserved(Unavailability::Changing));
                }
                Ok(ReadEntry::Present {
                    kind: regular_kind(after.mode()),
                    bytes,
                })
            }
            _ => Ok(ReadEntry::Unavailable {
                kind: EntryKind::Special,
                reason: Unavailability::UnsupportedKind,
            }),
        }
    }
}

#[cfg(unix)]
fn regular_kind(mode: u32) -> EntryKind {
    EntryKind::Regular {
        executable: mode & 0o111 != 0,
    }
}

#[cfg(unix)]
#[derive(Clone, PartialEq, Eq)]
struct FileStamp {
    identity: (i128, i128),
    len: u64,
    mode: u32,
    modified: (i64, i64),
    changed: (i64, i64),
}
#[cfg(unix)]
impl FileStamp {
    fn of(meta: &std::fs::Metadata) -> Self {
        use std::os::unix::fs::MetadataExt;
        Self {
            identity: (i128::from(meta.dev()), i128::from(meta.ino())),
            len: meta.len(),
            mode: meta.mode(),
            modified: (meta.mtime(), meta.mtime_nsec()),
            changed: (meta.ctime(), meta.ctime_nsec()),
        }
    }
    fn verifies(&self, after: &Self, named_entry: (i128, i128), bytes: u64) -> bool {
        self == after && self.identity == named_entry && self.len == bytes
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn directory_descriptors_close_on_exec() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("nested")).unwrap();
        let source = DirectoryInput::open(root.path()).unwrap();
        let (parent, _) = source
            .parent(&ReviewPath::parse(b"nested/file".to_vec()).unwrap())
            .unwrap();
        assert!(
            rustix::io::fcntl_getfd(&parent)
                .unwrap()
                .contains(rustix::io::FdFlags::CLOEXEC)
        );
        assert_eq!(
            regular_kind(0o644),
            EntryKind::Regular { executable: false }
        );
        assert_eq!(regular_kind(0o755), EntryKind::Regular { executable: true });
    }
    #[test]
    fn equal_timestamps_do_not_prove_path_identity_or_complete_read_length() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("file"), b"four").unwrap();
        let stamp = FileStamp::of(&std::fs::metadata(root.path().join("file")).unwrap());
        assert!(stamp.verifies(&stamp, stamp.identity, 4));
        assert!(!stamp.verifies(&stamp, (stamp.identity.0, stamp.identity.1 + 1), 4));
        assert!(!stamp.verifies(&stamp, stamp.identity, 3));
        let mut changed = stamp.clone();
        changed.mode ^= 0o100;
        assert!(!stamp.verifies(&changed, stamp.identity, 4));
    }
}
