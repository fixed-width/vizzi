//! Resource observation covers the child tree and reports an unsuccessful command.
#![cfg(target_os = "linux")]

#[test]
fn cli_writes_observations_and_rejects_malformed_arguments() {
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("memory.json");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_vizzi-envelope"))
        .arg("--observe-only")
        .arg(&output)
        .args(["--", "true"])
        .status()
        .unwrap();
    assert!(status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
    assert_eq!(report["success"], true);
    assert_eq!(report["mode"], "observe_only");
    for args in [
        vec![],
        vec!["--observe-only"],
        vec!["--observe-only", "/unused", "true"],
    ] {
        assert!(
            !std::process::Command::new(env!("CARGO_BIN_EXE_vizzi-envelope"))
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
}

#[test]
fn observe_only_accepts_successful_work_and_never_claims_a_cap() {
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("memory.json");
    let success = vizzi_perf::envelope::run(
        &output,
        true,
        &["sh".into(), "-c".into(), "sleep 0.3".into()],
    )
    .unwrap();
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert!(success, "{report}");
    assert_eq!(report["success"], true);
    assert_eq!(report["cap_verified"], false);
    assert_eq!(report["requested_cap_bytes"], 12_884_901_888u64);
    assert_eq!(report["group_path"], serde_json::Value::Null);
    assert_eq!(report["errors"], serde_json::json!([]));
    for sample in report["samples"].as_array().unwrap() {
        assert_eq!(sample["errors"], serde_json::json!([]));
        let readings = sample["processes"].as_array().unwrap();
        assert_eq!(
            sample["usage"]["pss_kib"].as_u64().unwrap(),
            readings
                .iter()
                .map(|r| r["usage"]["pss_kib"].as_u64().unwrap())
                .sum::<u64>()
        );
    }
}

#[test]
fn observe_only_records_simultaneous_memory_and_preserves_child_failure() {
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("memory.json");
    let success = vizzi_perf::envelope::run(
        &output,
        true,
        &["sh".into(), "-c".into(), "sleep 0.3; exit 7".into()],
    )
    .unwrap();
    assert!(!success);
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
    assert_eq!(report["mode"], "observe_only");
    assert_eq!(report["cap_verified"], false);
    assert_eq!(report["child_exit_code"], 7);
    assert!(report["peak_pss_kib"].as_u64().unwrap() > 0);
    assert!(
        report["samples"]
            .as_array()
            .unwrap()
            .iter()
            .any(|sample| { sample["processes"].as_array().unwrap().len() >= 3 })
    );
    assert!(vizzi_perf::envelope::run(&output, true, &["true".into()]).is_err());
}
