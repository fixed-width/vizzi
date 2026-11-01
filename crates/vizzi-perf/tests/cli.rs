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
