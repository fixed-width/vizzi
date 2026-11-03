//! Combined qualification requires native watching and uses no manual hints for edits.
#![cfg(feature = "qualification")]

#[test]
fn native_watch_recovery_and_cleanup_preserve_source_facts_and_pins() {
    let owned = tempfile::tempdir().unwrap();
    let root = owned.path().join("source");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(".vizzi-foreground-owned"), "owner").unwrap();
    std::fs::write(root.join("a.rs"), "fn original() {}\n").unwrap();
    let config = serde_json::json!({"root":root,"cache":owned.path().join("cache"),"policy":vizzi_core::policy::PolicyConfig::default(),
        "target":"a.rs","owner":"owner","probe":"interactive-fixture","spool_mib":4,"tail_mib":1,"pairs":2,
        "watch":true,"recovery_ms":400,"recovery_samples":1});
    let report =
        vizzi::idle_probe::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {}).unwrap();
    assert_eq!(report["watching"], true);
    assert_eq!(report["explicit_hints"], false);
    assert_eq!(report["cases"].as_array().unwrap().len(), 5);
    assert_eq!(
        report["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|case| case["condition"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "after_cleanup",
            "during_cleanup",
            "during_cleanup",
            "after_cleanup",
            "recovery_observed"
        ]
    );
    assert_eq!(
        report["cases"][4]["status_before"]["recovery"]["running"],
        true
    );
    for case in report["cases"].as_array().unwrap() {
        assert_eq!(case["explicit_hint"], false);
        if case["condition"] != "recovery_observed" {
            assert_eq!(case["capture_marks"], serde_json::json!([]));
            assert!(case["capture_overlaps_cleanup"].is_null());
        }
        for key in [
            "source_matches",
            "fresh_facts_match",
            "expected_declaration",
            "old_pin_matches",
        ] {
            assert_eq!(case[key], true);
        }
    }
    assert_eq!(report["final_clean_matches"], true);
    assert_eq!(report["spool"]["file_bytes"], 0);
    assert_eq!(
        std::fs::read_to_string(root.join("a.rs")).unwrap(),
        "fn original() {}\n"
    );
}
