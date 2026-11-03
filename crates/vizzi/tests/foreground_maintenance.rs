//! The foreground probe uses real host publication and restores owned source.
#![cfg(feature = "qualification")]

#[test]
fn foreground_edits_during_cleanup_keep_facts_pins_and_restoration_correct() {
    let owned = tempfile::tempdir().unwrap();
    let root = owned.path().join("source");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(".vizzi-foreground-owned"), "owner").unwrap();
    std::fs::write(root.join("a.rs"), "fn original() {}\n").unwrap();
    let mut config = serde_json::json!({"root":root,"cache":owned.path().join("cache"),
        "policy":vizzi_core::policy::PolicyConfig::default(),"target":"a.rs","owner":"wrong",
        "probe":"foreground-fixture","spool_mib":4,"tail_mib":1,"pairs":8});
    assert!(
        vizzi::foreground_maintenance::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {})
            .is_err()
    );
    config["owner"] = "".into();
    std::fs::write(root.join(".vizzi-foreground-owned"), "").unwrap();
    assert!(
        vizzi::foreground_maintenance::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {})
            .is_err()
    );
    std::fs::write(root.join(".vizzi-foreground-owned"), "owner").unwrap();
    config["owner"] = "owner".into();
    let report =
        vizzi::foreground_maintenance::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {})
            .unwrap();
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 16);
    for (index, case) in cases.iter().enumerate() {
        assert_eq!(case["measurements"]["read_files"], 1);
        assert_eq!(case["source_matches"], true);
        assert_eq!(case["fresh_facts_match"], true);
        assert_eq!(case["expected_declaration"], true);
        assert_eq!(case["old_pin_matches"], true);
        assert_eq!(case["capture_marks"].as_array().unwrap().len(), 1);
        let expected = if [0, 3].contains(&(index % 4)) {
            "after_cleanup"
        } else {
            "during_cleanup"
        };
        assert_eq!(case["condition"], expected);
        let number = |key: &str| case[key].as_f64().unwrap();
        let entered = case["capture_marks"][0]["entered_ms"].as_f64().unwrap();
        let ready = case["capture_marks"][0]["ready_ms"].as_f64().unwrap();
        let gate = case["capture_marks"][0]["gate_ms"].as_f64().unwrap();
        assert!((ready - entered - gate).abs() < 0.000001);
        assert!(number("write_start_ms") <= number("hint_ms"));
        assert!(
            number("hint_ms") <= entered && entered <= ready && ready <= number("published_ms")
        );
        assert!(
            (number("published_ms") - number("write_start_ms") - number("write_to_published_ms"))
                .abs()
                < 0.000001
        );
        assert!(
            (number("cleanup_end_ms") - number("cleanup_start_ms") - number("cleanup_ms")).abs()
                < 0.000001
        );
        if expected == "after_cleanup" {
            assert!(!case["write_overlaps_cleanup"].as_bool().unwrap());
        }
    }
    assert_eq!(report["restored_matches"], true);
    assert_eq!(report["final_clean_matches"], true);
    assert_eq!(report["spool"]["file_bytes"], 0);
    assert_eq!(
        std::fs::read_to_string(root.join("a.rs")).unwrap(),
        "fn original() {}\n"
    );
    for (field, value) in [
        ("owner", serde_json::json!("")),
        ("probe", serde_json::json!("")),
        ("pairs", serde_json::json!(0)),
        ("pairs", serde_json::json!(9)),
        ("tail_mib", serde_json::json!(0)),
        ("spool_mib", serde_json::json!(0)),
        ("tail_mib", serde_json::json!(usize::MAX)),
        ("spool_mib", serde_json::json!(usize::MAX)),
    ] {
        let mut invalid = config.clone();
        invalid[field] = value;
        assert!(
            vizzi::foreground_maintenance::run(&serde_json::to_vec(&invalid).unwrap(), &mut |_| {})
                .is_err()
        );
    }
    config["tail_mib"] = 4.into();
    assert!(
        vizzi::foreground_maintenance::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {})
            .unwrap_err()
            .to_string()
            .contains("tail must leave space")
    );
}
