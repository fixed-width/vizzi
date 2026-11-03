//! The owned-copy resource workflow checks source, facts, pins and restoration.
#![cfg(feature = "qualification")]

#[test]
fn admission_workflow_covers_edits_new_files_and_recovery_without_running_projects() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join(".vizzi-admission-owned"), "owner").unwrap();
    let targets = ["a.rs", "b.rs", "c.rs", "d.rs"];
    for path in targets {
        std::fs::write(root.path().join(path), "fn example() {}\n").unwrap();
    }
    let mut config = serde_json::json!({"root":root.path(),"cache":cache.path(),"policy":vizzi_core::policy::PolicyConfig::default(),
        "targets":targets,"owner":"wrong","probe":"fixture-admission","input_mib":1,"recovery_ms":10});
    assert!(
        vizzi::admission_probe::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {}).is_err()
    );
    config["owner"] = "owner".into();
    let report =
        vizzi::admission_probe::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {}).unwrap();
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 4);
    assert_eq!(cases[1]["documents"][0]["coverage"], "deferred");
    for case in cases {
        assert_eq!(case["source_scope_matches"], true);
        assert_eq!(case["completed_facts_match"], true);
        assert_eq!(case["status_before"]["recovery"]["running"], true);
        assert!(case["admission"]["peak_bytes"].as_u64().unwrap() <= 1048576);
    }
    assert_eq!(report["final_clean_matches"], true);
    assert_eq!(report["old_pin_source_matches"], true);
    assert_eq!(report["admission_final"]["active_jobs"], 0);
    for path in targets {
        assert_eq!(
            std::fs::read_to_string(root.path().join(path)).unwrap(),
            "fn example() {}\n"
        );
    }
    assert!(!root.path().join("vizzi_admission_probe.rs").exists());
}
