//! Verify Cargo refreshes embedded versions across successive builds of a Git checkout.

use std::{fs, path::PathBuf, process::Command};

const VERSION_SOURCE: &str = r#"fn main() { println!("vizzi {}", env!("VIZZI_VERSION")); }"#;

struct Checkout {
    root: tempfile::TempDir,
    crate_dir: PathBuf,
}

impl Checkout {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let crate_dir = root.path().join("crates/vizzi");
        fs::create_dir_all(crate_dir.join("src")).unwrap();
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = 'vizzi'\nversion = '0.0.0'\nedition = '2024'\n",
        )
        .unwrap();
        fs::write(crate_dir.join("build.rs"), include_bytes!("../build.rs")).unwrap();
        fs::write(crate_dir.join("src/main.rs"), VERSION_SOURCE).unwrap();
        fs::write(root.path().join("README.md"), "Fixture\n").unwrap();
        let checkout = Self { root, crate_dir };
        checkout.git(&["init", "--quiet", "--initial-branch=version-test"]);
        checkout.git(&["add", "crates/vizzi", "README.md"]);
        checkout.git(&["commit", "--quiet", "-m", "Initial fixture"]);
        checkout.git(&["tag", "--no-sign", "v1.2.3"]);
        checkout
    }

    fn git(&self, args: &[&str]) -> String {
        output(
            Command::new("git")
                .current_dir(self.root.path())
                .args([
                    "--no-optional-locks",
                    "-c",
                    "user.name=Vizzi Test",
                    "-c",
                    "user.email=test@example.invalid",
                    "-c",
                    "commit.gpgSign=false",
                ])
                .args(args),
        )
    }

    fn version(&self) -> String {
        output(
            Command::new(env!("CARGO"))
                .current_dir(&self.crate_dir)
                .env("CARGO_TARGET_DIR", self.root.path().join("target"))
                .env_remove("GITHUB_REF_TYPE")
                .env_remove("GITHUB_REF_NAME")
                .args(["run", "--offline", "--quiet"]),
        )
    }
}

fn output(command: &mut Command) -> String {
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{command:?}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap().trim().to_owned()
}

#[test]
fn unstaged_edits_refresh_the_dirty_marker() {
    let checkout = Checkout::new();
    assert_eq!(checkout.version(), "vizzi 1.2.3");

    let source = checkout.crate_dir.join("src/main.rs");
    fs::write(&source, format!("{VERSION_SOURCE}\n// Unstaged edit\n")).unwrap();
    assert_eq!(checkout.version(), "vizzi 1.2.3-dirty");

    fs::write(source, VERSION_SOURCE).unwrap();
    assert_eq!(checkout.version(), "vizzi 1.2.3");

    fs::write(checkout.root.path().join("README.md"), "Workspace edit\n").unwrap();
    assert_eq!(checkout.version(), "vizzi 1.2.3-dirty");
}

#[test]
fn creating_a_loose_ref_for_a_packed_branch_refreshes_the_revision() {
    let checkout = Checkout::new();
    checkout.git(&["commit", "--quiet", "--allow-empty", "-m", "Next revision"]);
    checkout.git(&["pack-refs", "--all", "--prune"]);
    let loose_ref = checkout.root.path().join(".git/refs/heads/version-test");
    assert!(!loose_ref.exists());
    let description = checkout.git(&["describe", "--tags", "--always"]);
    assert_eq!(
        checkout.version(),
        format!("vizzi {}", description.strip_prefix('v').unwrap())
    );

    checkout.git(&["reset", "--soft", "HEAD^"]);
    assert!(loose_ref.exists());
    assert_eq!(checkout.version(), "vizzi 1.2.3");
}
