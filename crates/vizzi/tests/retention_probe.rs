//! The owned-copy probe must measure actual parsing and preserve its source inputs.
#![cfg(feature = "qualification")]

#[test]
fn repeated_edit_probe_checks_fresh_equivalence_and_rejects_unowned_roots() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join(".vizzi-retention-owned"), "owner").unwrap();
    for (path, text) in [
        ("a.rs", "fn example() {}"),
        ("b.py", "def example(): pass\n"),
    ] {
        std::fs::write(root.path().join(path), text).unwrap();
    }
    let mut config = serde_json::json!({"root":root.path(),"cache":cache.path(),"policy":vizzi_core::policy::PolicyConfig::default(),"workers":2,"targets":["a.rs","b.py"],"samples":2,"budget_mib":1,"probe":"unique probe","owner":"wrong"});
    assert!(vizzi::retention::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {}).is_err());
    config["owner"] = "owner".into();
    let report = vizzi::retention::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {}).unwrap();
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 18);
    for case in cases {
        assert_eq!(case["measurements"]["parsed"], 1);
        assert_eq!(case["measurements"]["read_files"], 1);
        assert_eq!(case["measurements"]["cache_hits"], 0);
        assert_eq!(case["clean_snapshot_matches"], true);
        assert_eq!(case["independent_facts_match"], true);
        assert_eq!(
            case["comment_prefix"],
            if case["path"] == "b.py" { "#" } else { "//" }
        );
        assert_eq!(case["syntax_cache"]["budget_weight"], 1_048_576);
        assert_eq!(case["syntax_cache"]["entry_weight_limit"], 524_288);
        #[cfg(target_os = "linux")]
        assert!(case["rss_before_verification_kib"].as_u64().unwrap() > 1024);
        assert_eq!(
            case["measurements"]["tree_reuses"],
            if case["priming"] == true { 0 } else { 1 }
        );
    }
    assert_eq!(
        std::fs::read_to_string(root.path().join("a.rs")).unwrap(),
        "fn example() {}"
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("b.py")).unwrap(),
        "def example(): pass\n"
    );
    for (field, value) in [
        ("samples", serde_json::json!(0)),
        ("samples", serde_json::json!(9)),
        ("targets", serde_json::json!([])),
        ("targets", serde_json::json!(["../a.rs"])),
        ("probe", serde_json::json!("")),
        ("owner", serde_json::json!("")),
        ("budget_mib", serde_json::json!(usize::MAX)),
    ] {
        let mut bad = config.clone();
        bad[field] = value;
        assert!(vizzi::retention::run(&serde_json::to_vec(&bad).unwrap(), &mut |_| {}).is_err());
    }
}
