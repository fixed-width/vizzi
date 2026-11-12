//! Native review acquisition through the same seam as a non-Git in-memory input.

#![cfg(unix)]

use std::collections::BTreeMap;
use vizzi::review_source::*;
use vizzi_core::{
    comparison::{ChangeKind, Comparison, EntryKind, EntryState, ReviewPath, Side, Unavailability},
    model::WorkspaceId,
};

fn path(name: &[u8]) -> ReviewPath {
    ReviewPath::parse(name.to_vec()).unwrap()
}

struct MemoryInput {
    workspace: WorkspaceId,
    entries: BTreeMap<ReviewPath, ReadEntry>,
    reads: usize,
    enumeration_fails: bool,
}

impl MemoryInput {
    fn with(entries: Vec<(&[u8], ReadEntry)>) -> Self {
        Self {
            workspace: WorkspaceId::derive(&[b"logical-root"]),
            entries: entries
                .into_iter()
                .map(|(name, entry)| (path(name), entry))
                .collect(),
            reads: 0,
            enumeration_fails: false,
        }
    }
}

impl ReviewInput for MemoryInput {
    fn workspace(&self) -> &WorkspaceId {
        &self.workspace
    }
    fn paths(
        &mut self,
        _: CaptureLimits,
        _: &dyn Fn() -> bool,
    ) -> Result<Vec<ReviewPath>, CaptureError> {
        if self.enumeration_fails {
            return Err(CaptureError::Unsupported("fixture enumeration failure"));
        }
        Ok(self.entries.keys().cloned().collect())
    }
    fn read(
        &mut self,
        path: &ReviewPath,
        _: usize,
        _: &dyn Fn() -> bool,
    ) -> Result<ReadEntry, CaptureError> {
        self.reads += 1;
        Ok(self.entries.get(path).cloned().unwrap_or(ReadEntry::Absent))
    }
    fn notices(&self) -> Vec<CaptureNotice> {
        vec![]
    }
}

fn file(bytes: &[u8]) -> ReadEntry {
    ReadEntry::Present {
        kind: EntryKind::Regular { executable: false },
        bytes: bytes.to_vec(),
    }
}
fn policy() -> CapturePolicy {
    CapturePolicy::scope(None, vec![])
}

#[test]
fn supplied_inputs_capture_source_only_text_and_complete_nonregular_inventory() {
    let mut input = MemoryInput::with(vec![
        (b"guide.md", file(b"hello\n")),
        (b"data.bin", file(b"\0\xff")),
        (
            b"link",
            ReadEntry::Present {
                kind: EntryKind::Symlink,
                bytes: b"../elsewhere".to_vec(),
            },
        ),
        (
            b"unknown",
            ReadEntry::Unobserved(Unavailability::Unreadable),
        ),
    ]);
    let captured = acquire(&mut input, &policy(), CaptureLimits::default(), &|| false).unwrap();
    assert_eq!(captured.observation.inventory().len(), 4);
    assert_eq!(captured.observation.snapshot().documents().len(), 2);
    assert_eq!(
        captured
            .observation
            .snapshot()
            .file_by_path("guide.md")
            .unwrap()
            .resident_source(),
        Some("hello\n")
    );
    assert!(
        captured
            .observation
            .snapshot()
            .documents()
            .iter()
            .all(|doc| doc.analyses().is_empty())
    );
    assert_eq!(captured.values[&path(b"link")], b"../elsewhere");
    assert_eq!(captured.bytes, 6 + 2 + 12);
}

#[test]
fn scope_exclusions_failures_and_limits_do_not_invent_complete_empty_capture() {
    let mut input = MemoryInput::with(vec![
        (b"src/a.rs", file(b"a")),
        (b"src/secret", file(b"secret")),
        (b"src-other/a.rs", file(b"outside")),
    ]);
    let captured = acquire(
        &mut input,
        &CapturePolicy::scope(Some(path(b"src")), vec![path(b"src/secret")]),
        CaptureLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(input.reads, 1);
    assert_eq!(captured.observation.inventory().len(), 2);
    assert_eq!(
        captured.observation.inventory()[1].state,
        EntryState::Unobserved(Unavailability::Excluded)
    );
    input.enumeration_fails = true;
    assert!(acquire(&mut input, &policy(), CaptureLimits::default(), &|| false).is_err());
    input.enumeration_fails = false;
    assert!(matches!(
        acquire(
            &mut input,
            &policy(),
            CaptureLimits {
                max_bytes: 0,
                ..CaptureLimits::default()
            },
            &|| false
        ),
        Err(CaptureError::Limit(_))
    ));
    assert!(matches!(
        acquire(
            &mut input,
            &policy(),
            CaptureLimits {
                max_paths: 1,
                ..CaptureLimits::default()
            },
            &|| false
        ),
        Err(CaptureError::Limit(_))
    ));
    assert!(matches!(
        acquire(&mut input, &policy(), CaptureLimits::default(), &|| true),
        Err(CaptureError::Cancelled)
    ));
}

struct Repo(tempfile::TempDir);
impl Repo {
    fn init() -> Self {
        let repo = Self(tempfile::tempdir().unwrap());
        repo.git(&["init", "--quiet", "--initial-branch=main", "--template="]);
        repo.git(&[
            "remote",
            "add",
            "origin",
            "https://example.invalid/not-fetched",
        ]);
        repo
    }
    fn root(&self) -> &std::path::Path {
        self.0.path()
    }
    fn git(&self, args: &[&str]) -> Vec<u8> {
        let output = self.command(args).output().unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }
    fn command(&self, args: &[&str]) -> std::process::Command {
        let mut command = std::process::Command::new("git");
        command
            .arg("-C")
            .arg(self.root())
            .args([
                "-c",
                "core.fsmonitor=false",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
            ])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_OPTIONAL_LOCKS", "0");
        command
    }
    fn write(&self, path: &str, bytes: &[u8]) {
        let path = self.root().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    fn commit(&self) {
        self.git(&["add", "--all"]);
        self.git(&["commit", "--quiet", "-m", "fixture"]);
    }
}

#[test]
fn git_working_bytes_combine_staged_unstaged_untracked_and_ignore_decisions() {
    let fixture = Repo::init();
    fixture.write("code.rs", b"base\n");
    fixture.write("reverted.rs", b"original\n");
    fixture.write("removed.md", b"gone\n");
    fixture.write(".gitignore", b"ignored\n");
    fixture.commit();
    let repository = GitRepository::open(fixture.root()).unwrap();
    let mut baseline = repository
        .pin("HEAD", CaptureLimits::default(), &|| false)
        .unwrap();
    fixture.write("code.rs", b"staged\n");
    fixture.write("reverted.rs", b"staged\n");
    fixture.git(&["add", "code.rs", "reverted.rs"]);
    fixture.write("code.rs", b"working\n");
    fixture.write("reverted.rs", b"original\n");
    fixture.write("notes.md", b"untracked\n");
    fixture.write("ignored", b"ignored\n");
    std::fs::remove_file(fixture.root().join("removed.md")).unwrap();
    let before = fixture.git(&["status", "--porcelain=v1", "-z", "--untracked-files=all"]);
    let base = acquire(&mut baseline, &policy(), CaptureLimits::default(), &|| {
        false
    })
    .unwrap();
    let mut working = repository.working(&baseline).unwrap();
    let after = acquire(&mut working, &policy(), CaptureLimits::default(), &|| false).unwrap();
    assert!(after.notices.is_empty());
    let pair = Comparison::between(base.observation, after.observation).unwrap();
    assert_eq!(
        pair.document(Side::Base, &path(b"code.rs"))
            .unwrap()
            .resident_source(),
        Some("base\n")
    );
    assert_eq!(
        pair.document(Side::Working, &path(b"code.rs"))
            .unwrap()
            .resident_source(),
        Some("working\n")
    );
    assert_eq!(
        pair.entry(&path(b"reverted.rs")).unwrap().change,
        ChangeKind::Unchanged
    );
    assert_eq!(
        pair.entry(&path(b"notes.md")).unwrap().change,
        ChangeKind::Added
    );
    assert_eq!(
        pair.entry(&path(b"removed.md")).unwrap().change,
        ChangeKind::Removed
    );
    assert!(pair.entry(&path(b"ignored")).is_none());
    assert_eq!(
        pair.document(Side::Base, &path(b"code.rs")).unwrap().id(),
        pair.document(Side::Working, &path(b"code.rs"))
            .unwrap()
            .id()
    );
    assert_eq!(
        fixture.git(&["status", "--porcelain=v1", "-z", "--untracked-files=all"]),
        before
    );
}

#[test]
fn pinned_ref_and_logical_workspace_survive_ref_movement_and_index_removal() {
    let fixture = Repo::init();
    fixture.write("a.rs", b"old\n");
    fixture.write("restore.md", b"present\n");
    fixture.commit();
    let repository = GitRepository::open(fixture.root()).unwrap();
    assert_eq!(
        repository.workspace(),
        vizzi::source::Directory::open(fixture.root())
            .unwrap()
            .workspace()
    );
    let mut baseline = repository
        .pin("HEAD", CaptureLimits::default(), &|| false)
        .unwrap();
    let pin = baseline.resolved().to_owned();
    fixture.write("a.rs", b"new\n");
    fixture.commit();
    fixture.git(&["rm", "--cached", "restore.md"]);
    let base = acquire(&mut baseline, &policy(), CaptureLimits::default(), &|| {
        false
    })
    .unwrap();
    let after = acquire(
        &mut repository.working(&baseline).unwrap(),
        &policy(),
        CaptureLimits::default(),
        &|| false,
    )
    .unwrap();
    let pair = Comparison::between(base.observation, after.observation).unwrap();
    assert_eq!(baseline.resolved(), pin);
    assert_eq!(
        pair.document(Side::Base, &path(b"a.rs"))
            .unwrap()
            .resident_source(),
        Some("old\n")
    );
    assert_eq!(
        pair.entry(&path(b"restore.md")).unwrap().change,
        ChangeKind::Unchanged
    );
    assert!(
        repository
            .pin("--help", CaptureLimits::default(), &|| false)
            .is_err()
    );
}

#[test]
fn directory_revision_and_working_sources_keep_namespace_and_preserve_unavailable_evidence() {
    let fixture = Repo::init();
    fixture.write("a.rs", b"base\n");
    fixture.commit();
    let mut directory = DirectoryInput::open(fixture.root()).unwrap();
    let repo = GitRepository::open(fixture.root()).unwrap();
    let mut base = repo
        .pin("HEAD", CaptureLimits::default(), &|| false)
        .unwrap();
    let disk = acquire(&mut directory, &policy(), CaptureLimits::default(), &|| {
        false
    })
    .unwrap();
    let revision = acquire(&mut base, &policy(), CaptureLimits::default(), &|| false).unwrap();
    let pair = Comparison::between(revision.observation, disk.observation).unwrap();
    assert_eq!(pair.entries().count(), 1);
    assert_eq!(pair.entries().next().unwrap().change, ChangeKind::Unchanged);
    let mut other = MemoryInput::with(vec![(b"a.rs", file(b"base\n"))]);
    other.workspace = repo.workspace().clone();
    let memory = acquire(&mut other, &policy(), CaptureLimits::default(), &|| false).unwrap();
    assert_eq!(pair.observation(Side::Base).id(), memory.observation.id());
    let snapshot = memory.observation.clone();
    other.entries.insert(path(b"a.rs"), file(b"edited\n"));
    assert_eq!(
        snapshot.snapshot().documents()[0].resident_source(),
        Some("base\n")
    );
    let mut working = repo.working(&base).unwrap();
    fixture.git(&["update-index", "--skip-worktree", "a.rs"]);
    std::fs::remove_file(fixture.root().join("a.rs")).unwrap();
    let working = acquire(&mut working, &policy(), CaptureLimits::default(), &|| false).unwrap();
    let pair = Comparison::between(snapshot, working.observation).unwrap();
    assert_eq!(
        pair.entry(&path(b"a.rs")).unwrap().change,
        ChangeKind::Unassessed
    );
    assert_eq!(
        pair.entry(&path(b"a.rs")).unwrap().working,
        Some(&EntryState::Unobserved(Unavailability::Sparse))
    );
}

#[test]
fn sparse_partial_invalid_roots_limits_and_cancellation_fail_explicitly() {
    let fixture = Repo::init();
    fixture.write("file", b"content");
    fixture.commit();
    let repo = GitRepository::open(fixture.root()).unwrap();
    assert!(matches!(
        repo.pin(
            "HEAD",
            CaptureLimits {
                max_discovery_bytes: 1,
                ..CaptureLimits::default()
            },
            &|| false
        ),
        Err(CaptureError::Limit(_))
    ));
    assert!(matches!(
        repo.pin("HEAD", CaptureLimits::default(), &|| true),
        Err(CaptureError::Cancelled)
    ));
    let mut base = repo
        .pin("HEAD", CaptureLimits::default(), &|| false)
        .unwrap();
    assert!(matches!(
        acquire(
            &mut base,
            &policy(),
            CaptureLimits {
                max_file_bytes: 1,
                ..CaptureLimits::default()
            },
            &|| false
        ),
        Err(CaptureError::Limit(_))
    ));
    assert!(acquire(&mut base, &policy(), CaptureLimits::default(), &|| false).is_ok());
    fixture.git(&["config", "core.sparseCheckout", "true"]);
    assert!(matches!(
        acquire(
            &mut repo.working(&base).unwrap(),
            &policy(),
            CaptureLimits::default(),
            &|| false
        ),
        Err(CaptureError::Unsupported(_))
    ));
    fixture.git(&["config", "core.sparseCheckout", "false"]);
    fixture.git(&["config", "remote.origin.promisor", "true"]);
    assert!(matches!(
        GitRepository::open(fixture.root()),
        Err(CaptureError::Unsupported(_))
    ));
    let other = Repo::init();
    other.write("sub/file", b"x");
    other.commit();
    assert!(GitRepository::open(&other.root().join("sub")).is_err());
    assert!(GitRepository::open(tempfile::tempdir().unwrap().path()).is_err());
    assert!(
        repo.working(
            &GitRepository::open(other.root())
                .unwrap()
                .pin("HEAD", CaptureLimits::default(), &|| false)
                .unwrap()
        )
        .is_err()
    );
}

#[test]
fn unmerged_and_assume_unchanged_index_flags_do_not_choose_after_bytes() {
    let fixture = Repo::init();
    fixture.write("a.rs", b"base\n");
    fixture.commit();
    let repo = GitRepository::open(fixture.root()).unwrap();
    let mut base = repo
        .pin("HEAD", CaptureLimits::default(), &|| false)
        .unwrap();
    fixture.git(&["update-index", "--assume-unchanged", "a.rs"]);
    fixture.write("a.rs", b"live\n");
    let after = acquire(
        &mut repo.working(&base).unwrap(),
        &policy(),
        CaptureLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        after.observation.snapshot().documents()[0].resident_source(),
        Some("live\n")
    );
    fixture.git(&["update-index", "--no-assume-unchanged", "a.rs"]);
    let object = String::from_utf8(fixture.git(&["rev-parse", "HEAD:a.rs"])).unwrap();
    fixture.git(&["update-index", "--force-remove", "a.rs"]);
    let mut child = fixture
        .command(&["update-index", "--index-info"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    let input = format!(
        "100644 {} 1\ta.rs\n100644 {} 2\ta.rs\n100644 {} 3\ta.rs\n",
        object.trim(),
        object.trim(),
        object.trim()
    );
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success());
    fixture.write(
        "a.rs",
        b"<<<<<<< local\nlive\n=======\nother\n>>>>>>> incoming\n",
    );
    let before = acquire(&mut base, &policy(), CaptureLimits::default(), &|| false).unwrap();
    let after = acquire(
        &mut repo.working(&base).unwrap(),
        &policy(),
        CaptureLimits::default(),
        &|| false,
    )
    .unwrap();
    assert!(
        after
            .notices
            .iter()
            .any(|notice| notice.path == path(b"a.rs") && notice.kind == NoticeKind::UnmergedIndex)
    );
    let pair = Comparison::between(before.observation, after.observation).unwrap();
    assert!(
        pair.document(Side::Working, &path(b"a.rs"))
            .unwrap()
            .resident_source()
            .unwrap()
            .starts_with("<<<<<<< local")
    );
}

#[cfg(unix)]
#[test]
fn no_follow_reads_preserve_symlinks_modes_unusual_names_and_avoid_special_file_reads() {
    use std::os::unix::{
        ffi::OsStringExt,
        fs::{PermissionsExt, symlink},
    };
    let fixture = Repo::init();
    fixture.write("a.rs", b"same\n");
    fixture.commit();
    let repo = GitRepository::open(fixture.root()).unwrap();
    let mut base = repo
        .pin("HEAD", CaptureLimits::default(), &|| false)
        .unwrap();
    std::fs::set_permissions(
        fixture.root().join("a.rs"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret"), b"outside").unwrap();
    symlink(outside.path(), fixture.root().join("link")).unwrap();
    let odd = b"odd\xff\n\\file";
    std::fs::write(
        fixture
            .root()
            .join(std::ffi::OsString::from_vec(odd.to_vec())),
        b"raw",
    )
    .unwrap();
    fixture.write("empty", b"");
    let after = acquire(
        &mut repo.working(&base).unwrap(),
        &policy(),
        CaptureLimits::default(),
        &|| false,
    )
    .unwrap();
    assert!(after.values.contains_key(&path(b"link")));
    assert!(
        after
            .observation
            .inventory()
            .iter()
            .any(|entry| entry.path == path(odd))
    );
    let before = acquire(&mut base, &policy(), CaptureLimits::default(), &|| false).unwrap();
    let pair = Comparison::between(before.observation, after.observation).unwrap();
    assert_eq!(
        pair.entry(&path(b"a.rs")).unwrap().change,
        ChangeKind::Modified
    );
    let mut directory = DirectoryInput::open(fixture.root()).unwrap();
    assert!(matches!(
        directory
            .read(&path(b"link/secret"), 100, &|| false)
            .unwrap(),
        ReadEntry::Unobserved(Unavailability::Unreadable)
    ));
    assert!(matches!(
        directory.read(&path(b"absent"), 100, &|| false).unwrap(),
        ReadEntry::Absent
    ));
    let fifo = std::process::Command::new("mkfifo")
        .args(["-m", "600"])
        .arg(fixture.root().join("pipe"))
        .output()
        .unwrap();
    assert!(fifo.status.success(), "mkfifo failed: {fifo:?}");
    assert!(matches!(
        directory.read(&path(b"pipe"), 100, &|| false).unwrap(),
        ReadEntry::Unavailable {
            kind: EntryKind::Special,
            ..
        }
    ));
}

#[cfg(unix)]
#[test]
fn changing_files_roots_and_mid_capture_cancellation_remain_explicit() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("file"), b"old").unwrap();
    let mut input = DirectoryInput::open(&root).unwrap();
    let calls = std::cell::Cell::new(0);
    let read = input
        .read(&path(b"file"), 100, &|| {
            let n = calls.get();
            calls.set(n + 1);
            if n == 1 {
                std::fs::write(root.join("file"), b"changed length").unwrap();
            }
            false
        })
        .unwrap();
    assert!(matches!(
        read,
        ReadEntry::Unobserved(Unavailability::Changing)
    ));
    let calls = std::cell::Cell::new(0);
    assert!(matches!(
        acquire(&mut input, &policy(), CaptureLimits::default(), &|| {
            let n = calls.get();
            calls.set(n + 1);
            n > 4
        }),
        Err(CaptureError::Cancelled)
    ));
    std::fs::rename(&root, parent.path().join("old")).unwrap();
    std::fs::create_dir(&root).unwrap();
    assert!(matches!(
        input.paths(CaptureLimits::default(), &|| false),
        Err(CaptureError::Unsupported(_))
    ));
    assert!(matches!(
        input.read(&path(b"file"), 100, &|| false),
        Err(CaptureError::Unsupported(_))
    ));
}

#[test]
fn nested_reference_values_are_captured_without_recursing_into_the_child() {
    let fixture = Repo::init();
    fixture.write("root.md", b"root\n");
    fixture.commit();
    let child = Repo::init();
    child.write("inside.rs", b"old\n");
    child.commit();
    let old = String::from_utf8(child.git(&["rev-parse", "HEAD"]))
        .unwrap()
        .trim()
        .to_owned();
    let nested = fixture.root().join("child");
    std::fs::rename(child.root(), &nested).unwrap();
    fixture.git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("160000,{old},child"),
    ]);
    fixture.git(&["commit", "--quiet", "-m", "reference"]);
    let repo = GitRepository::open(fixture.root()).unwrap();
    let mut base = repo
        .pin("HEAD", CaptureLimits::default(), &|| false)
        .unwrap();
    let mut command = fixture.command(&[
        "-C",
        nested.to_str().unwrap(),
        "commit",
        "--allow-empty",
        "--quiet",
        "-m",
        "next",
    ]);
    assert!(command.status().unwrap().success());
    let before = acquire(&mut base, &policy(), CaptureLimits::default(), &|| false).unwrap();
    let after = acquire(
        &mut repo.working(&base).unwrap(),
        &policy(),
        CaptureLimits::default(),
        &|| false,
    )
    .unwrap();
    assert!(
        after
            .notices
            .iter()
            .any(|notice| notice.kind == NoticeKind::NestedContentsNotCaptured)
    );
    assert_ne!(
        before.values[&path(b"child")],
        after.values[&path(b"child")]
    );
    assert_eq!(
        before.notices,
        [CaptureNotice {
            path: path(b"child"),
            kind: NoticeKind::NestedContentsNotCaptured
        }]
    );
    assert!(matches!(
        base.read(&path(b"child"), 39, &|| false),
        Err(CaptureError::Limit(_))
    ));
    assert!(
        matches!(base.read(&path(b"child"),40,&||false).unwrap(),ReadEntry::Present {bytes,..} if bytes.len()==40)
    );
    let mut direct = repo.working(&base).unwrap();
    direct.paths(CaptureLimits::default(), &|| false).unwrap();
    assert!(matches!(
        direct.read(&path(b"child"), 39, &|| false),
        Err(CaptureError::Limit(_))
    ));
    assert!(
        matches!(direct.read(&path(b"child"),40,&||false).unwrap(),ReadEntry::Present {bytes,..} if bytes.len()==40)
    );
    let pair = Comparison::between(before.observation, after.observation).unwrap();
    assert_eq!(
        pair.entry(&path(b"child")).unwrap().change,
        ChangeKind::Modified
    );
    assert!(pair.entry(&path(b"child/inside.rs")).is_none());
}

#[cfg(unix)]
#[test]
fn raw_git_objects_ignore_filters_export_attributes_and_fsmonitor_commands() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Repo::init();
    fixture.write("a.rs", b"base\r\n");
    fixture.write(".gitattributes", b"a.rs export-ignore\n");
    fixture.commit();
    fixture.write("probe.sh", b"#!/bin/sh\nprintf invoked > marker\n");
    std::fs::set_permissions(
        fixture.root().join("probe.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    fixture.git(&[
        "config",
        "core.fsmonitor",
        fixture.root().join("probe.sh").to_str().unwrap(),
    ]);
    fixture.git(&[
        "config",
        "filter.probe.clean",
        fixture.root().join("probe.sh").to_str().unwrap(),
    ]);
    fixture.write(".gitattributes", b"a.rs filter=probe\n");
    fixture.write("a.rs", b"working\r\n");
    fixture.git(&["config", "harmless.payload", &"x".repeat(5000)]);
    fixture.git(&["tag", "-a", "-m", "tag", "baseline"]);
    let repo = GitRepository::open(fixture.root()).unwrap();
    let mut base = repo
        .pin("baseline", CaptureLimits::default(), &|| false)
        .unwrap();
    assert_eq!(
        base.resolved(),
        String::from_utf8(fixture.git(&["rev-parse", "HEAD"]))
            .unwrap()
            .trim()
    );
    let before = acquire(&mut base, &policy(), CaptureLimits::default(), &|| false).unwrap();
    let after = acquire(
        &mut repo.working(&base).unwrap(),
        &policy(),
        CaptureLimits::default(),
        &|| false,
    )
    .unwrap();
    let pair = Comparison::between(before.observation, after.observation).unwrap();
    assert_eq!(
        pair.document(Side::Base, &path(b"a.rs"))
            .unwrap()
            .resident_source(),
        Some("base\r\n")
    );
    assert_eq!(
        pair.document(Side::Working, &path(b"a.rs"))
            .unwrap()
            .resident_source(),
        Some("working\r\n")
    );
    assert!(!fixture.root().join("marker").exists());
}

#[cfg(unix)]
#[test]
fn nested_regular_directory_reads_are_complete_and_bounded_before_scope_filtering() {
    let fixture = Repo::init();
    fixture.write("src/deep/a.rs", b"one\n");
    fixture.write("doc/guide.md", b"two\n");
    fixture.write("empty", b"");
    let mut source = DirectoryInput::open(fixture.root()).unwrap();
    assert!(
        matches!(source.read(&path(b"src/deep/a.rs"),4,&||false).unwrap(),ReadEntry::Present {bytes,..} if bytes==b"one\n")
    );
    let capture = acquire(&mut source, &policy(), CaptureLimits::default(), &|| false).unwrap();
    assert_eq!(capture.observation.inventory().len(), 3);
    assert_eq!(capture.bytes, 8);
    assert!(
        source
            .paths(
                CaptureLimits {
                    max_paths: 6,
                    max_discovery_bytes: 44,
                    ..CaptureLimits::default()
                },
                &|| false
            )
            .is_ok()
    );
    assert!(matches!(
        source.paths(
            CaptureLimits {
                max_paths: 5,
                ..CaptureLimits::default()
            },
            &|| false
        ),
        Err(CaptureError::Limit(_))
    ));
    assert!(matches!(
        source.paths(
            CaptureLimits {
                max_paths: 0,
                ..CaptureLimits::default()
            },
            &|| false
        ),
        Err(CaptureError::Limit(_))
    ));
    assert!(matches!(
        source.paths(
            CaptureLimits {
                max_discovery_bytes: 1,
                ..CaptureLimits::default()
            },
            &|| false
        ),
        Err(CaptureError::Limit(_))
    ));
    let empty = acquire(
        &mut source,
        &CapturePolicy::scope(Some(path(b"empty")), vec![]),
        CaptureLimits {
            max_bytes: 0,
            ..CaptureLimits::default()
        },
        &|| false,
    )
    .unwrap();
    assert_eq!(empty.bytes, 0);
    assert_eq!(empty.observation.snapshot().documents().len(), 1);
}

#[test]
fn unavailable_content_policy_identity_and_missing_objects_remain_explicit() {
    let mut memory = MemoryInput::with(vec![
        (
            b"denied",
            ReadEntry::Unavailable {
                kind: EntryKind::Regular { executable: false },
                reason: Unavailability::Unreadable,
            },
        ),
        (b"x", file(b"x")),
        (b"y", file(b"y")),
    ]);
    let a = acquire(
        &mut memory,
        &CapturePolicy::scope(None, vec![path(b"x"), path(b"y")]),
        CaptureLimits::default(),
        &|| false,
    )
    .unwrap();
    let b = acquire(
        &mut memory,
        &CapturePolicy::scope(None, vec![path(b"y"), path(b"x"), path(b"x")]),
        CaptureLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(a.observation.id(), b.observation.id());
    assert!(matches!(
        a.observation.inventory()[0].state,
        EntryState::Present {
            content: vizzi_core::comparison::EntryContent::Unavailable(Unavailability::Unreadable),
            ..
        }
    ));
    let fixture = Repo::init();
    fixture.write("a", b"object\n");
    fixture.commit();
    let repo = GitRepository::open(fixture.root()).unwrap();
    let mut base = repo
        .pin("HEAD", CaptureLimits::default(), &|| false)
        .unwrap();
    let id = String::from_utf8(fixture.git(&["rev-parse", "HEAD:a"]))
        .unwrap()
        .trim()
        .to_owned();
    std::fs::remove_file(
        fixture
            .root()
            .join(".git/objects")
            .join(&id[..2])
            .join(&id[2..]),
    )
    .unwrap();
    let captured = acquire(&mut base, &policy(), CaptureLimits::default(), &|| false).unwrap();
    assert!(matches!(
        captured.observation.inventory()[0].state,
        EntryState::Present {
            content: vizzi_core::comparison::EntryContent::Unavailable(Unavailability::Unreadable),
            ..
        }
    ));
}

#[test]
fn exact_byte_and_candidate_allowances_are_inclusive_for_supplied_and_git_inputs() {
    let mut memory = MemoryInput::with(vec![(b"a", file(b"abc")), (b"b", file(b"def"))]);
    assert_eq!(
        acquire(
            &mut memory,
            &policy(),
            CaptureLimits {
                max_bytes: 6,
                max_paths: 2,
                max_file_bytes: 3,
                ..CaptureLimits::default()
            },
            &|| false
        )
        .unwrap()
        .bytes,
        6
    );
    assert!(matches!(
        acquire(
            &mut memory,
            &policy(),
            CaptureLimits {
                max_bytes: 5,
                ..CaptureLimits::default()
            },
            &|| false
        ),
        Err(CaptureError::Limit(_))
    ));
    let fixture = Repo::init();
    fixture.write("a", &vec![b'x'; 70_000]);
    fixture.write("b", b"small");
    fixture.commit();
    let repo = GitRepository::open(fixture.root()).unwrap();
    let mut base = repo
        .pin("HEAD", CaptureLimits::default(), &|| false)
        .unwrap();
    assert_eq!(
        base.paths(
            CaptureLimits {
                max_paths: 2,
                ..CaptureLimits::default()
            },
            &|| false
        )
        .unwrap()
        .len(),
        2
    );
    assert!(matches!(
        base.paths(
            CaptureLimits {
                max_paths: 0,
                ..CaptureLimits::default()
            },
            &|| false
        ),
        Err(CaptureError::Limit(_))
    ));
    assert!(
        matches!(base.read(&path(b"a"),70_000,&||false).unwrap(),ReadEntry::Present {bytes,..} if bytes.len()==70_000)
    );
    assert!(matches!(
        base.read(&path(b"a"), 69_999, &|| false),
        Err(CaptureError::Limit(_))
    ));
    let mut work = repo.working(&base).unwrap();
    assert_eq!(
        work.paths(
            CaptureLimits {
                max_paths: 2,
                ..CaptureLimits::default()
            },
            &|| false
        )
        .unwrap()
        .len(),
        2
    );
    assert!(matches!(
        work.paths(
            CaptureLimits {
                max_paths: 0,
                ..CaptureLimits::default()
            },
            &|| false
        ),
        Err(CaptureError::Limit(_))
    ));
    fixture.write("c", b"untracked");
    assert!(matches!(
        work.paths(
            CaptureLimits {
                max_paths: 2,
                ..CaptureLimits::default()
            },
            &|| false
        ),
        Err(CaptureError::Limit(_))
    ));
    assert_eq!(
        work.paths(
            CaptureLimits {
                max_paths: 3,
                ..CaptureLimits::default()
            },
            &|| false
        )
        .unwrap()
        .len(),
        3
    );
}

#[cfg(unix)]
#[test]
fn committed_symlinks_and_executables_and_atomic_saves_keep_correct_entry_evidence() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let fixture = Repo::init();
    fixture.write("executable", b"file");
    std::fs::set_permissions(
        fixture.root().join("executable"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    symlink("executable", fixture.root().join("link")).unwrap();
    fixture.commit();
    let repo = GitRepository::open(fixture.root()).unwrap();
    let mut base = repo
        .pin("HEAD", CaptureLimits::default(), &|| false)
        .unwrap();
    let captured = acquire(&mut base, &policy(), CaptureLimits::default(), &|| false).unwrap();
    assert_eq!(captured.values[&path(b"link")], b"executable");
    let mut directory = DirectoryInput::open(fixture.root()).unwrap();
    assert!(
        matches!(directory.read(&path(b"link"),10,&||false).unwrap(),ReadEntry::Present {bytes,..} if bytes==b"executable")
    );
    assert!(matches!(
        directory.read(&path(b"link"), 9, &|| false),
        Err(CaptureError::Limit(_))
    ));
    assert!(matches!(
        directory.read(&path(b"executable"), 4, &|| false).unwrap(),
        ReadEntry::Present {
            kind: EntryKind::Regular { executable: true },
            ..
        }
    ));
    let calls = std::cell::Cell::new(0);
    let read = directory
        .read(&path(b"executable"), 100, &|| {
            let n = calls.get();
            calls.set(n + 1);
            if n == 1 {
                std::fs::rename(
                    fixture.root().join("executable"),
                    fixture.root().join("old"),
                )
                .unwrap();
                std::fs::write(fixture.root().join("executable"), b"next").unwrap();
            }
            false
        })
        .unwrap();
    assert!(matches!(
        read,
        ReadEntry::Unobserved(Unavailability::Changing)
    ));
}

#[test]
fn git_revision_expressions_and_short_ids_use_the_local_commit_graph() {
    let fixture = Repo::init();
    fixture.write("a.rs", b"fn first() {}\n");
    fixture.commit();
    let first = String::from_utf8(fixture.git(&["rev-parse", "HEAD"]))
        .unwrap()
        .trim()
        .to_owned();
    fixture.write("a.rs", b"fn second() {}\n");
    fixture.commit();
    let second = String::from_utf8(fixture.git(&["rev-parse", "HEAD"]))
        .unwrap()
        .trim()
        .to_owned();
    fixture.write("a.rs", b"fn third() {}\n");
    fixture.commit();
    let repo = GitRepository::open(fixture.root()).unwrap();
    for (expression, expected) in [
        ("HEAD~1", second.as_str()),
        ("HEAD^^", first.as_str()),
        (&first[..8], first.as_str()),
    ] {
        let pin = repo
            .pin(expression, CaptureLimits::default(), &|| false)
            .unwrap();
        assert_eq!(pin.resolved(), expected, "{expression}");
    }
}

#[test]
fn missing_parents_in_shallow_history_have_an_actionable_resolution_error() {
    let fixture = Repo::init();
    fixture.write("a.rs", b"fn first() {}\n");
    fixture.commit();
    let head = String::from_utf8(fixture.git(&["rev-parse", "HEAD"]))
        .unwrap()
        .trim()
        .to_owned();
    std::fs::write(fixture.root().join(".git/shallow"), format!("{head}\n")).unwrap();
    let repo = GitRepository::open(fixture.root()).unwrap();
    let error = repo
        .pin("HEAD~1", CaptureLimits::default(), &|| false)
        .err()
        .unwrap()
        .to_string();
    assert!(
        error.contains("shallow") && error.contains("history"),
        "{error}"
    );
    assert_eq!(
        repo.pin(&head[..8], CaptureLimits::default(), &|| false)
            .unwrap()
            .resolved(),
        head
    );
}
