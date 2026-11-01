//! End-to-end checks for measurement output and source preservation.

use std::{fs, process::Command};

#[test]
fn inventory_reports_known_contents_without_modifying_source() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("code.rs"), b"abc").unwrap();
    fs::write(root.path().join("binary"), [0, 255]).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_vizzi-perf"))
        .arg("inventory")
        .arg(root.path())
        .arg("1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        (report["files"].as_u64(), report["content_bytes"].as_u64()),
        (Some(2), Some(5))
    );
    assert_eq!(
        (
            fs::read(root.path().join("code.rs")).unwrap(),
            fs::read(root.path().join("binary")).unwrap()
        ),
        (b"abc".to_vec(), vec![0, 255])
    );
}

#[test]
fn zero_samples_fail_instead_of_producing_empty_measurements() {
    let root = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_vizzi-perf"))
        .arg("inventory")
        .arg(root.path())
        .arg("0")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn generator_creates_the_requested_workload_and_refuses_to_overwrite_it() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("fixture");
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_vizzi-perf"))
            .arg("generate")
            .arg(&root)
            .arg("3")
            .arg("128")
            .output()
            .unwrap()
    };
    assert!(run().status.success());
    let captured = vizzi_perf::source::capture(&root).unwrap();
    assert_eq!(
        (
            captured.files().len(),
            captured.files().values().map(|f| f.bytes).sum::<u64>()
        ),
        (3, 384)
    );
    assert!(!run().status.success());
    assert_eq!(vizzi_perf::source::capture(&root).unwrap(), captured);
}

#[cfg(feature = "syntax")]
#[test]
fn syntax_command_scopes_files_and_verifies_incremental_results() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("code.rs"), b"fn f() {}").unwrap();
    fs::write(root.path().join("fixture.c"), b"not Rust source").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_vizzi-syntax"))
        .arg("rust")
        .arg(root.path())
        .arg("1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        (
            report["matching_files"].as_u64(),
            report["measured_files"].as_u64()
        ),
        (Some(1), Some(1))
    );
    let edits = report["incremental_edits"].as_array().unwrap();
    assert_eq!(edits.len(), 3);
    assert!(edits.iter().all(|edit| edit["validation_passed"] == true));
    assert_eq!(fs::read(root.path().join("code.rs")).unwrap(), b"fn f() {}");
}

#[cfg(feature = "watch")]
#[test]
fn watcher_command_rejects_insufficient_distinct_files() {
    let output = Command::new(env!("CARGO_BIN_EXE_vizzi-watch"))
        .arg("2")
        .arg("2")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[cfg(feature = "syntax")]
#[test]
fn parallel_command_validates_results_and_does_not_overwrite_baselines() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("source");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("a.rs"), b"fn a() {}").unwrap();
    fs::write(root.join("b.rs"), b"@").unwrap();
    let baseline = parent.path().join("baseline.json");
    let run = |workers: &str, mode: &str| {
        Command::new(env!("CARGO_BIN_EXE_vizzi-parallel"))
            .arg("rust")
            .arg(&root)
            .arg(workers)
            .arg("1")
            .arg(mode)
            .arg(&baseline)
            .output()
            .unwrap()
    };
    let sequential = run("1", "--write-baseline");
    assert!(
        sequential.status.success(),
        "{}",
        String::from_utf8_lossy(&sequential.stderr)
    );
    let before = fs::read(&baseline).unwrap();
    let parallel = run("4", "--baseline");
    assert!(
        parallel.status.success(),
        "{}",
        String::from_utf8_lossy(&parallel.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&parallel.stdout).unwrap();
    assert_eq!(
        (report["files"].as_u64(), report["actual_workers"].as_u64()),
        (Some(2), Some(2))
    );
    assert_eq!(report["validation"], "sequential_baseline");
    assert!(!run("1", "--write-baseline").status.success());
    assert_eq!(fs::read(&baseline).unwrap(), before);
    assert_eq!(fs::read(root.join("a.rs")).unwrap(), b"fn a() {}");
    fs::write(root.join("a.rs"), b"fn z() {}").unwrap();
    let changed = run("2", "--baseline");
    assert!(!changed.status.success());
    assert!(String::from_utf8_lossy(&changed.stderr).contains("differ from baseline"));
}
