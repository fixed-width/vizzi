//! Controlled mixed working sets through the optional production-engine probe.
#![cfg(feature = "qualification")]

#[test]
fn working_set_probe_reports_each_transition_and_restores_owned_inputs() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join(".vizzi-working-set-owned"), "owner").unwrap();
    for (path, source) in [("a.rs", "fn a() {}"), ("b.py", "def b(): pass\n")] {
        std::fs::write(root.path().join(path), source).unwrap();
    }
    let config = serde_json::json!({"root":root.path(),"cache":cache.path(),"policy":vizzi_core::policy::PolicyConfig::default(),"workers":1,
        "targets":["a.rs","b.py"],"steps":[{"kind":"edit","paths":["a.rs"]},{"kind":"edit","paths":["b.py"]},{"kind":"edit","paths":["a.rs","b.py"]},{"kind":"reconcile"}],
        "budget_mib":1,"probe":"fresh-working-set","owner":"owner"});
    let report =
        vizzi::working_set::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {}).unwrap();
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 4);
    assert_eq!(cases[0]["entries_before"]["a.rs"], serde_json::Value::Null);
    assert_eq!(cases[0]["entries_after"]["a.rs"]["state"], "idle");
    assert_eq!(cases[0]["measurements"]["tree_reuses"], 0);
    assert_eq!(cases[1]["entries_before"]["b.py"], serde_json::Value::Null);
    assert_eq!(cases[2]["measurements"]["tree_reuses"], 2);
    assert_eq!(cases[2]["measurements"]["parsed"], 2);
    assert_eq!(cases[3]["measurements"]["parsed"], 0);
    assert_eq!(cases[3]["measurements"]["reconciled"], true);
    assert_eq!(cases[3]["entries_before"], cases[3]["entries_after"]);
    for case in cases {
        assert_eq!(case["clean_snapshot_matches"], true);
        assert_eq!(case["independent_facts_match"], true);
        assert_eq!(case["syntax_cache_after"]["active_leases"], 0);
        assert!(
            case["syntax_cache_after"]["peak_charged_weight"]
                .as_u64()
                .unwrap()
                <= 1048576
        );
    }
    assert_eq!(
        std::fs::read_to_string(root.path().join("a.rs")).unwrap(),
        "fn a() {}"
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("b.py")).unwrap(),
        "def b(): pass\n"
    );
    let mut disabled = config.clone();
    disabled["budget_mib"] = 0.into();
    disabled["probe"] = "fresh-disabled".into();
    let report =
        vizzi::working_set::run(&serde_json::to_vec(&disabled).unwrap(), &mut |_| {}).unwrap();
    for case in report["cases"].as_array().unwrap() {
        assert_eq!(case["syntax_cache_after"], serde_json::Value::Null);
        assert_eq!(case["measurements"]["tree_reuses"], 0);
    }
    for (field, value) in [
        ("owner", serde_json::json!("wrong")),
        ("owner", serde_json::json!("")),
        ("probe", serde_json::json!("")),
        ("targets", serde_json::json!([])),
        ("targets", serde_json::json!(["../a.rs"])),
        ("targets", serde_json::json!(["a.rs", "a.rs"])),
        ("steps", serde_json::json!([])),
        ("steps", serde_json::json!([{"kind":"edit","paths":[]}])),
        (
            "steps",
            serde_json::json!([{"kind":"edit","paths":["a.rs","a.rs"]}]),
        ),
        (
            "steps",
            serde_json::json!([{"kind":"edit","paths":["missing.rs"]}]),
        ),
        ("budget_mib", serde_json::json!(usize::MAX)),
    ] {
        let mut bad = config.clone();
        bad[field] = value;
        assert!(
            vizzi::working_set::run(&serde_json::to_vec(&bad).unwrap(), &mut |_| {}).is_err(),
            "accepted invalid {field}"
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("a.rs")).unwrap(),
            "fn a() {}"
        );
    }
}
