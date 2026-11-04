//! Read-only Git object discovery and observed working files.

use super::*;
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

const CONFIG_LIMIT: usize = 1024 * 1024;

#[derive(Clone)]
struct GitEntry {
    kind: EntryKind,
    oid: String,
}

/// A native Git root, independent of comparison-model identifiers.
#[derive(Clone)]
pub struct GitRepository {
    root: PathBuf,
    workspace: WorkspaceId,
}
/// A resolved immutable Git baseline.
pub struct GitRevision {
    repository: GitRepository,
    resolved: String,
    entries: Arc<BTreeMap<ReviewPath, GitEntry>>,
    batch: Option<Batch>,
}
/// Working candidate discovery against a resolved baseline, with disk after bytes.
pub struct GitWorking {
    repository: GitRepository,
    base: Arc<BTreeMap<ReviewPath, GitEntry>>,
    disk: DirectoryInput,
    links: BTreeSet<ReviewPath>,
    sparse: BTreeSet<ReviewPath>,
    unmerged: BTreeSet<ReviewPath>,
}

impl GitRepository {
    /// Opens the exact worktree root; linked worktrees are supported, bare/subdirectory roots are not.
    pub fn open(root: &Path) -> Result<Self, CaptureError> {
        let (root, workspace) = crate::source::workspace_root(root)?;
        let repository = Self { root, workspace };
        if repository.run(&["rev-parse", "--is-inside-work-tree"], 1024, &|| false)? != b"true\n"
            || repository.run(&["rev-parse", "--show-prefix"], 1024, &|| false)? != b"\n"
        {
            return Err(CaptureError::Unsupported("Git worktree root required"));
        }
        let config = repository.run(&["config", "--null", "--list"], CONFIG_LIMIT, &|| false)?;
        for record in records(&config)? {
            let key = record
                .split(|byte| *byte == b'\n')
                .next()
                .unwrap_or_default();
            if key == b"extensions.partialclone"
                || (key.starts_with(b"remote.") && key.ends_with(b".promisor"))
            {
                return Err(CaptureError::Unsupported(
                    "partial/promisor repository capture",
                ));
            }
        }
        Ok(repository)
    }
    /// Returns the same logical identity as the ordinary directory adapter.
    pub fn workspace(&self) -> &WorkspaceId {
        &self.workspace
    }
    /// Resolves a revision once and freezes its complete raw tree inventory.
    pub fn pin(
        &self,
        revision: &str,
        limits: CaptureLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<GitRevision, CaptureError> {
        let resolved = oid(&self.run(
            &[
                "rev-parse",
                "--verify",
                "--end-of-options",
                &format!("{revision}^{{}}"),
            ],
            1024,
            cancelled,
        )?)?;
        let tree = oid(&self.run(
            &[
                "rev-parse",
                "--verify",
                "--end-of-options",
                &format!("{resolved}^{{tree}}"),
            ],
            1024,
            cancelled,
        )?)?;
        let entries = parse_tree(
            &self.run(
                &["ls-tree", "-r", "-z", "--full-tree", &tree],
                limits.max_discovery_bytes,
                cancelled,
            )?,
            limits.max_paths,
        )?;
        Ok(GitRevision {
            repository: self.clone(),
            resolved,
            entries: Arc::new(entries),
            batch: None,
        })
    }
    /// Creates working discovery tied to this exact baseline/workspace namespace.
    pub fn working(&self, base: &GitRevision) -> Result<GitWorking, CaptureError> {
        if self.workspace != base.repository.workspace {
            return Err(CaptureError::Unsupported(
                "baseline belongs to another workspace",
            ));
        }
        Ok(GitWorking {
            repository: self.clone(),
            base: base.entries.clone(),
            disk: DirectoryInput::open(&self.root)?,
            links: BTreeSet::new(),
            sparse: BTreeSet::new(),
            unmerged: BTreeSet::new(),
        })
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(&self.root).args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.untrackedCache=false",
        ]);
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("GIT_") {
                cmd.env_remove(key);
            }
        }
        cmd.env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_NO_LAZY_FETCH", "1")
            .env("GIT_NO_REPLACE_OBJECTS", "1");
        cmd.args(args).stderr(Stdio::null());
        cmd
    }
    fn run(
        &self,
        args: &[&str],
        limit: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, CaptureError> {
        let mut cmd = self.command(args);
        let mut process = Process(cmd.stdin(Stdio::null()).stdout(Stdio::piped()).spawn()?);
        let mut output = process
            .0
            .stdout
            .take()
            .ok_or_else(|| CaptureError::Git("missing command output".into()))?;
        let bytes = read_bytes(&mut output, limit, cancelled)?;
        if !process.0.wait()?.success() {
            return Err(CaptureError::Git(format!("{} failed", args[0])));
        }
        check(cancelled)?;
        Ok(bytes)
    }
}

impl GitRevision {
    /// Clones immutable baseline metadata without keeping or duplicating a blob-reader child.
    pub fn fork(&self) -> Self {
        Self {
            repository: self.repository.clone(),
            resolved: self.resolved.clone(),
            entries: self.entries.clone(),
            batch: None,
        }
    }
    /// Returns the resolved native object identity, for baseline provenance only.
    pub fn resolved(&self) -> &str {
        &self.resolved
    }
}

impl ReviewInput for GitRevision {
    fn workspace(&self) -> &WorkspaceId {
        self.repository.workspace()
    }
    fn paths(
        &mut self,
        limits: CaptureLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<ReviewPath>, CaptureError> {
        check(cancelled)?;
        if self.entries.len() > limits.max_paths {
            return Err(CaptureError::Limit("candidate count"));
        }
        Ok(self.entries.keys().cloned().collect())
    }
    fn read(
        &mut self,
        path: &ReviewPath,
        limit: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ReadEntry, CaptureError> {
        check(cancelled)?;
        let Some(entry) = self.entries.get(path) else {
            return Ok(ReadEntry::Absent);
        };
        if entry.kind == EntryKind::Reference {
            if entry.oid.len() > limit {
                return Err(CaptureError::Limit("content bytes"));
            }
            return Ok(ReadEntry::Present {
                kind: entry.kind,
                bytes: entry.oid.as_bytes().to_vec(),
            });
        }
        if self.batch.is_none() {
            self.batch = Some(Batch::spawn(&self.repository)?);
        }
        let result = match self.batch.as_mut() {
            Some(batch) => batch.read(&entry.oid, limit, cancelled),
            None => return Err(CaptureError::Git("missing object reader".into())),
        };
        match result {
            Ok(bytes) => Ok(ReadEntry::Present {
                kind: entry.kind,
                bytes,
            }),
            Err(CaptureError::Cancelled) => {
                self.batch = None;
                Err(CaptureError::Cancelled)
            }
            Err(CaptureError::Limit(kind)) => {
                self.batch = None;
                Err(CaptureError::Limit(kind))
            }
            Err(_) => {
                self.batch = None;
                Ok(ReadEntry::Unavailable {
                    kind: entry.kind,
                    reason: Unavailability::Unreadable,
                })
            }
        }
    }
    fn notices(&self) -> Vec<CaptureNotice> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.kind == EntryKind::Reference)
            .map(|(path, _)| CaptureNotice {
                path: path.clone(),
                kind: NoticeKind::NestedContentsNotCaptured,
            })
            .collect()
    }
}

impl ReviewInput for GitWorking {
    fn workspace(&self) -> &WorkspaceId {
        self.repository.workspace()
    }
    fn paths(
        &mut self,
        limits: CaptureLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<ReviewPath>, CaptureError> {
        // Fail explicit sparse mode; absence must not masquerade as a user deletion.
        let sparse = self.repository.run(
            &[
                "config",
                "--bool",
                "--default=false",
                "--get",
                "core.sparseCheckout",
            ],
            1024,
            cancelled,
        )?;
        if sparse != b"false\n" {
            return Err(CaptureError::Unsupported("sparse checkout capture"));
        }
        let index = self.repository.run(
            &["ls-files", "--stage", "--full-name", "-z"],
            limits.max_discovery_bytes,
            cancelled,
        )?;
        if self.base.len() > limits.max_paths {
            return Err(CaptureError::Limit("candidate count"));
        }
        let mut paths: BTreeSet<_> = self.base.keys().cloned().collect();
        self.links = self
            .base
            .iter()
            .filter(|(_, entry)| entry.kind == EntryKind::Reference)
            .map(|(path, _)| path.clone())
            .collect();
        self.unmerged.clear();
        self.sparse.clear();
        for record in records(&index)? {
            let (meta, name) = split_record(record)?;
            let fields: Vec<_> = meta.split(|byte| *byte == b' ').collect();
            if fields.len() != 3 {
                return Err(CaptureError::Git("invalid index metadata".into()));
            }
            let kind = kind(fields[0])?;
            let _ = oid(fields[1])?;
            let path = ReviewPath::parse(name.to_vec())?;
            if fields[2] != b"0" {
                self.unmerged.insert(path.clone());
            }
            if kind == EntryKind::Reference {
                self.links.insert(path.clone());
            }
            paths.insert(path);
            if paths.len() > limits.max_paths {
                return Err(CaptureError::Limit("candidate count"));
            }
        }
        let tags = self.repository.run(
            &["ls-files", "-v", "--full-name", "-z"],
            limits.max_discovery_bytes,
            cancelled,
        )?;
        self.sparse = parse_tags(&tags)?;
        let others = self.repository.run(
            &[
                "ls-files",
                "--others",
                "--exclude-standard",
                "--full-name",
                "-z",
            ],
            limits.max_discovery_bytes,
            cancelled,
        )?;
        for record in records(&others)? {
            let name = record.strip_suffix(b"/").unwrap_or(record);
            let path = ReviewPath::parse(name.to_vec())?;
            if name.len() != record.len() {
                self.links.insert(path.clone());
            }
            paths.insert(path);
            if paths.len() > limits.max_paths {
                return Err(CaptureError::Limit("candidate count"));
            }
        }
        check(cancelled)?;
        Ok(paths.into_iter().collect())
    }
    fn read(
        &mut self,
        path: &ReviewPath,
        limit: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ReadEntry, CaptureError> {
        let read = self.disk.read(path, limit, cancelled)?;
        if matches!(read, ReadEntry::Absent) && self.sparse.contains(path) {
            return Ok(ReadEntry::Unobserved(Unavailability::Sparse));
        }
        if self.links.contains(path)
            && matches!(
                read,
                ReadEntry::Absent
                    | ReadEntry::Unavailable {
                        kind: EntryKind::Special,
                        ..
                    }
            )
        {
            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStrExt;
                let root = self
                    .repository
                    .root
                    .join(std::ffi::OsStr::from_bytes(path.bytes()));
                if std::fs::symlink_metadata(&root).is_ok_and(|meta| meta.is_dir())
                    && let Ok(nested) = GitRepository::open(&root)
                    && nested.root == root
                    && let Ok(bytes) =
                        nested.run(&["rev-parse", "--verify", "HEAD"], 1024, cancelled)
                {
                    let value = oid(&bytes)?.into_bytes();
                    if value.len() > limit {
                        return Err(CaptureError::Limit("content bytes"));
                    }
                    return Ok(ReadEntry::Present {
                        kind: EntryKind::Reference,
                        bytes: value,
                    });
                }
            }
            check(cancelled)?;
            return Ok(ReadEntry::Unobserved(Unavailability::Unreadable));
        }
        Ok(read)
    }
    fn notices(&self) -> Vec<CaptureNotice> {
        self.unmerged
            .iter()
            .map(|path| CaptureNotice {
                path: path.clone(),
                kind: NoticeKind::UnmergedIndex,
            })
            .chain(self.links.iter().map(|path| CaptureNotice {
                path: path.clone(),
                kind: NoticeKind::NestedContentsNotCaptured,
            }))
            .collect()
    }
}

fn parse_tags(bytes: &[u8]) -> Result<BTreeSet<ReviewPath>, CaptureError> {
    let mut sparse = BTreeSet::new();
    for record in records(bytes)? {
        let [flag, b' ', name @ ..] = record else {
            return Err(CaptureError::Git("invalid index flags".into()));
        };
        let path = ReviewPath::parse(name.to_vec())?;
        if matches!(flag, b'S' | b's') {
            sparse.insert(path);
        }
    }
    Ok(sparse)
}

fn blob_size(fields: &[&[u8]], id: &str) -> Result<usize, CaptureError> {
    match fields {
        [object, b"blob", size] if *object == id.as_bytes() => std::str::from_utf8(size)
            .ok()
            .and_then(|size| size.parse().ok())
            .ok_or_else(|| CaptureError::Git("invalid object length".into())),
        _ => Err(CaptureError::Git("missing/non-blob object".into())),
    }
}

fn records(bytes: &[u8]) -> Result<impl Iterator<Item = &[u8]>, CaptureError> {
    let body = if bytes.is_empty() {
        bytes
    } else {
        bytes
            .strip_suffix(b"\0")
            .ok_or_else(|| CaptureError::Git("unterminated path inventory".into()))?
    };
    if !bytes.is_empty()
        && body
            .split(|byte| *byte == 0)
            .any(|record| record.is_empty())
    {
        return Err(CaptureError::Git("unterminated path inventory".into()));
    }
    Ok(body
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty()))
}
fn split_record(record: &[u8]) -> Result<(&[u8], &[u8]), CaptureError> {
    let split = record
        .iter()
        .position(|byte| *byte == b'\t')
        .ok_or_else(|| CaptureError::Git("invalid path record".into()))?;
    Ok((&record[..split], &record[split + 1..]))
}
fn oid(bytes: &[u8]) -> Result<String, CaptureError> {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if !matches!(bytes.len(), 40 | 64) || !bytes.iter().all(u8::is_ascii_hexdigit) {
        return Err(CaptureError::Git("invalid object identity".into()));
    }
    String::from_utf8(bytes.to_vec())
        .map_err(|_| CaptureError::Git("non-ASCII object identity".into()))
}
fn kind(mode: &[u8]) -> Result<EntryKind, CaptureError> {
    match mode {
        b"100644" => Ok(EntryKind::Regular { executable: false }),
        b"100755" => Ok(EntryKind::Regular { executable: true }),
        b"120000" => Ok(EntryKind::Symlink),
        b"160000" => Ok(EntryKind::Reference),
        _ => Err(CaptureError::Unsupported("Git entry mode")),
    }
}
fn parse_tree(bytes: &[u8], limit: usize) -> Result<BTreeMap<ReviewPath, GitEntry>, CaptureError> {
    let mut entries = BTreeMap::new();
    for record in records(bytes)? {
        let (meta, name) = split_record(record)?;
        let fields: Vec<_> = meta.split(|byte| *byte == b' ').collect();
        if fields.len() != 3 {
            return Err(CaptureError::Git("invalid tree metadata".into()));
        }
        let entry = GitEntry {
            kind: kind(fields[0])?,
            oid: oid(fields[2])?,
        };
        if fields[1]
            != if entry.kind == EntryKind::Reference {
                b"commit".as_slice()
            } else {
                b"blob".as_slice()
            }
        {
            return Err(CaptureError::Git("tree object type mismatch".into()));
        }
        if entries
            .insert(ReviewPath::parse(name.to_vec())?, entry)
            .is_some()
        {
            return Err(CaptureError::Git("duplicate tree path".into()));
        }
        if entries.len() > limit {
            return Err(CaptureError::Limit("candidate count"));
        }
    }
    Ok(entries)
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct Batch {
    _process: Process,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Batch {
    fn spawn(repository: &GitRepository) -> Result<Self, CaptureError> {
        let mut process = Process(
            repository
                .command(&["cat-file", "--batch"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?,
        );
        let input = process
            .0
            .stdin
            .take()
            .ok_or_else(|| CaptureError::Git("missing object input".into()))?;
        let output = BufReader::new(
            process
                .0
                .stdout
                .take()
                .ok_or_else(|| CaptureError::Git("missing object output".into()))?,
        );
        Ok(Self {
            _process: process,
            input,
            output,
        })
    }
    fn read(
        &mut self,
        id: &str,
        limit: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, CaptureError> {
        check(cancelled)?;
        writeln!(self.input, "{id}")?;
        self.input.flush()?;
        let mut header = vec![];
        (&mut self.output)
            .take(512)
            .read_until(b'\n', &mut header)?;
        if !header.ends_with(b"\n") {
            return Err(CaptureError::Git("invalid object header".into()));
        }
        let fields: Vec<_> = header[..header.len() - 1]
            .split(|byte| *byte == b' ')
            .collect();
        let count = blob_size(&fields, id)?;
        if count > limit {
            return Err(CaptureError::Limit("content bytes"));
        }
        let mut bytes = Vec::with_capacity(count);
        let mut buffer = [0u8; READ_CHUNK];
        while bytes.len() < count {
            check(cancelled)?;
            let size = buffer.len().min(count - bytes.len());
            self.output.read_exact(&mut buffer[..size])?;
            bytes.extend_from_slice(&buffer[..size]);
        }
        let mut ending = [0];
        self.output.read_exact(&mut ending)?;
        if ending != *b"\n" {
            return Err(CaptureError::Git("invalid object terminator".into()));
        }
        check(cancelled)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_protocol_keeps_path_bytes_and_rejects_malformed_tree_records() {
        let id = "a".repeat(40);
        let bytes = format!("100644 blob {id}\twith\ttab\nand newline\0");
        let entries = parse_tree(bytes.as_bytes(), 1).unwrap();
        assert!(
            entries.contains_key(&ReviewPath::parse(b"with\ttab\nand newline".to_vec()).unwrap())
        );
        for bad in [
            b"no terminator".to_vec(),
            b"missing tab\0".to_vec(),
            b"100644 blob\tname\0".to_vec(),
            format!("100644 tree {id}\tname\0").into_bytes(),
            format!("040000 tree {id}\tname\0").into_bytes(),
            b"100644 blob invalid\tname\0".to_vec(),
            format!("100644 blob {id}\t/absolute\0").into_bytes(),
            [bytes.as_bytes(), bytes.as_bytes()].concat(),
        ] {
            assert!(parse_tree(&bad, 10).is_err(), "{bad:?}");
        }
        assert!(matches!(
            parse_tree(bytes.as_bytes(), 0),
            Err(CaptureError::Limit(_))
        ));
        assert!(parse_tree(b"\0", 10).is_err());
        assert!(parse_tree(&[bytes.as_bytes(), b"\0"].concat(), 10).is_err());
        assert!(oid(b"a\n").is_err());
        assert!(oid("z".repeat(40).as_bytes()).is_err());
        assert_eq!(oid("b".repeat(64).as_bytes()).unwrap(), "b".repeat(64));
        assert_eq!(
            parse_tags(b"S a\0H b\0s c\0").unwrap(),
            [
                ReviewPath::parse(b"a".to_vec()).unwrap(),
                ReviewPath::parse(b"c".to_vec()).unwrap()
            ]
            .into()
        );
        for invalid in [b"H".as_slice(), b"H:\0", b"H \0"] {
            assert!(parse_tags(invalid).is_err());
        }
        assert_eq!(
            blob_size(&[id.as_bytes(), b"blob", b"70000"], &id).unwrap(),
            70_000
        );
        for fields in [
            vec![id.as_bytes(), b"missing"],
            vec![id.as_bytes(), b"commit", b"0"],
            vec![b"wrong", b"blob", b"1"],
            vec![id.as_bytes(), b"blob", b"bad"],
        ] {
            assert!(blob_size(&fields, &id).is_err());
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn owned_process_is_terminated_and_reaped_on_drop() {
        let child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        drop(Process(child));
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }
}
