//! Embed the release tag or local source revision in the executable.

use std::{env, path::Path, process::Command};

const WORKSPACE: &str = "../..";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=GITHUB_REF_TYPE");
    println!("cargo:rerun-if-env-changed=GITHUB_REF_NAME");

    let version = release_version()
        .or_else(git_version)
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned());
    println!("cargo:rustc-env=VIZZI_VERSION={version}");
}

fn release_version() -> Option<String> {
    if env::var("GITHUB_REF_TYPE").as_deref() != Ok("tag") {
        return None;
    }
    normalize_version(env::var("GITHUB_REF_NAME").ok()?)
}

fn git_version() -> Option<String> {
    // A source archive may be unpacked inside an unrelated Git checkout.
    if !Path::new(WORKSPACE).join(".git").exists() {
        return None;
    }
    for path in ["HEAD", "index", "packed-refs", "refs/heads", "refs/tags"] {
        watch_git_path(path);
    }
    watch_tracked_files();
    let mut description = git_output(&["describe", "--tags", "--always"])?;
    // `git describe --dirty` rewrites the index, which Cargo watches for staged changes.
    let status = git_output(&["status", "--porcelain=v1", "--untracked-files=no"])?;
    if !status.is_empty() {
        description.push_str("-dirty");
    }
    normalize_version(description)
}

fn watch_tracked_files() {
    let Some(files) = git_bytes(&["ls-files", "--full-name", "-t", "-z", "--", ":/"]) else {
        return;
    };
    // H lists tracked files outside the sparse-checkout skip-worktree set.
    for entry in files.split(|byte| *byte == 0) {
        if let Some(file) = entry.strip_prefix(b"H ")
            && let Ok(file) = std::str::from_utf8(file)
            && !file.contains(['\r', '\n'])
        {
            println!("cargo:rerun-if-changed={WORKSPACE}/{file}");
        }
    }
}

fn watch_git_path(name: &str) {
    if let Some(path) = git_output(&["rev-parse", "--git-path", name])
        && Path::new(&path).exists()
    {
        println!("cargo:rerun-if-changed={path}");
    }
}

fn git_output(args: &[&str]) -> Option<String> {
    Some(String::from_utf8(git_bytes(args)?).ok()?.trim().to_owned())
}

fn git_bytes(args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("--no-optional-locks")
        .args(args)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

fn normalize_version(version: String) -> Option<String> {
    let version = version.trim();
    let version = version.strip_prefix('v').unwrap_or(version);
    (!version.is_empty()).then(|| version.to_owned())
}
