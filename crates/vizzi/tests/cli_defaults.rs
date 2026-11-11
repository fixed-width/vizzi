//! Launch defaults are exercised in isolated child environments, without global mutation.
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct Fixture {
    directory: tempfile::TempDir,
    source: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::create_dir(directory.path().join("home")).unwrap();
        fs::write(source.join("example.py"), "def example(): return 1\n").unwrap();
        Self { directory, source }
    }
    fn command(&self, cwd: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vizzi"));
        command
            .current_dir(cwd)
            .args(["--once", "--workers", "1"])
            .env("HOME", self.directory.path().join("home"))
            .env("XDG_CACHE_HOME", self.directory.path().join("system-cache"));
        command
    }
    fn status(&self, command: &mut Command, root: &Path) -> Value {
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let status: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            status["name"],
            root.canonicalize().unwrap().to_string_lossy().as_ref()
        );
        assert_eq!(status["measurements"]["files"], 1);
        status
    }
}

#[test]
fn omitted_root_uses_the_process_current_directory() {
    let fixture = Fixture::new();
    let cache = fixture.directory.path().join("manual-cache");
    fixture.status(
        fixture.command(&fixture.source).arg("--cache").arg(&cache),
        &fixture.source,
    );
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 1);
}

#[test]
fn explicit_relative_root_and_cache_override_defaults() {
    let fixture = Fixture::new();
    fixture.status(
        fixture.command(fixture.directory.path()).args([
            "--root",
            "source",
            "--cache",
            "manual-cache",
        ]),
        &fixture.source,
    );
    assert_eq!(
        fs::read_dir(fixture.directory.path().join("manual-cache"))
            .unwrap()
            .count(),
        1
    );
    assert!(!fixture.directory.path().join("system-cache").exists());
}

#[test]
fn help_describes_optional_paths_without_creating_cache_data() {
    let fixture = Fixture::new();
    let output = fixture
        .command(&fixture.source)
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("[--root DIR]"));
    assert!(text.contains("[--cache DIR]"));
    assert!(text.contains("current directory"));
    assert!(!fixture.directory.path().join("system-cache").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn bare_launch_uses_xdg_cache_and_reuses_a_stable_workspace_directory() {
    let fixture = Fixture::new();
    let first = fixture.status(&mut fixture.command(&fixture.source), &fixture.source);
    let second = fixture.status(&mut fixture.command(&fixture.source), &fixture.source);
    assert_eq!(first["measurements"]["parsed"], 1);
    assert_eq!(second["measurements"]["cache_hits"], 1);
    let workspaces = fixture
        .directory
        .path()
        .join("system-cache/vizzi/workspaces");
    let entries = fs::read_dir(workspaces)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    assert_eq!(fs::read_dir(&entries[0]).unwrap().count(), 1);
    assert_eq!(fs::read_dir(&fixture.source).unwrap().count(), 1);
}

#[cfg(target_os = "linux")]
#[test]
fn default_cache_uses_the_selected_root_and_separates_repositories() {
    let fixture = Fixture::new();
    fixture.status(&mut fixture.command(&fixture.source), &fixture.source);
    let other = fixture.directory.path().join("second/source");
    fs::create_dir_all(&other).unwrap();
    fs::copy(fixture.source.join("example.py"), other.join("example.py")).unwrap();
    let selected = fixture.status(
        fixture.command(&fixture.source).arg("--root").arg(&other),
        &other,
    );
    assert_eq!(selected["measurements"]["parsed"], 1);
    assert_eq!(
        fs::read_dir(
            fixture
                .directory
                .path()
                .join("system-cache/vizzi/workspaces")
        )
        .unwrap()
        .count(),
        2
    );
    std::os::unix::fs::symlink(&other, fixture.directory.path().join("alias")).unwrap();
    let alias = fixture.status(
        fixture
            .command(fixture.directory.path())
            .args(["--root", "alias"]),
        &other,
    );
    assert_eq!(alias["measurements"]["cache_hits"], 1);
    assert_eq!(
        fs::read_dir(
            fixture
                .directory
                .path()
                .join("system-cache/vizzi/workspaces")
        )
        .unwrap()
        .count(),
        2
    );
}

#[cfg(target_os = "linux")]
#[test]
fn missing_empty_or_relative_xdg_cache_uses_the_home_cache_directory() {
    let fixture = Fixture::new();
    for xdg in [None, Some(""), Some("relative-cache")] {
        let mut command = fixture.command(&fixture.source);
        match xdg {
            None => {
                command.env_remove("XDG_CACHE_HOME");
            }
            Some(path) => {
                command.env("XDG_CACHE_HOME", path);
            }
        }
        fixture.status(&mut command, &fixture.source);
    }
    assert_eq!(
        fs::read_dir(
            fixture
                .directory
                .path()
                .join("home/.cache/vizzi/workspaces")
        )
        .unwrap()
        .count(),
        1
    );
    assert!(!fixture.source.join("relative-cache").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn a_default_cache_inside_source_requires_an_explicit_external_override() {
    let fixture = Fixture::new();
    let output = fixture
        .command(&fixture.source)
        .env("XDG_CACHE_HOME", fixture.source.join("cache"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("--cache"), "{error}");
    assert!(error.contains("outside"), "{error}");
    assert!(!fixture.source.join("cache").exists());
    fixture.status(
        fixture
            .command(&fixture.source)
            .env("XDG_CACHE_HOME", fixture.source.join("cache"))
            .arg("--cache")
            .arg(fixture.directory.path().join("manual-cache")),
        &fixture.source,
    );
}
