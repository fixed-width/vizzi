//! Automatic maintenance qualification preserves source and observation identity.
#![cfg(feature = "qualification")]
#[test]
fn idle_probe_restores_source_facts_and_all_backing() {
    let owned = tempfile::tempdir().unwrap();
    let root = owned.path().join("source");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(".vizzi-foreground-owned"), "owner").unwrap();
    std::fs::write(root.join("a.rs"), "fn original() {}\n").unwrap();
    let config = serde_json::json!({"root":root,"cache":owned.path().join("cache"),"policy":vizzi_core::policy::PolicyConfig::default(),
        "target":"a.rs","owner":"owner","probe":"idle-fixture","spool_mib":4,"tail_mib":1,"pairs":1});
    let report =
        vizzi::idle_probe::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {}).unwrap();
    assert_eq!(report["cases"].as_array().unwrap().len(), 2);
    assert_eq!(report["final_clean_matches"], true);
    assert_eq!(report["spool"]["file_bytes"], 0);
    for case in report["cases"].as_array().unwrap() {
        for key in [
            "source_matches",
            "fresh_facts_match",
            "expected_declaration",
            "old_pin_matches",
        ] {
            assert_eq!(case[key], true);
        }
        assert_eq!(case["measurements"]["read_files"], 1);
        assert!(
            case["cleanup_end_ms"].as_f64().unwrap() >= case["cleanup_start_ms"].as_f64().unwrap()
        );
    }
    assert_eq!(
        std::fs::read_to_string(root.join("a.rs")).unwrap(),
        "fn original() {}\n"
    );
}
