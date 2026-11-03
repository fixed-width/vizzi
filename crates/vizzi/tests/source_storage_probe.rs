//! Source-only ownership accounting across real publication history and external pins.
#![cfg(feature = "qualification")]

#[test]
fn source_storage_probe_deduplicates_shared_handles_and_releases_expired_pins() {
    for mode in ["headers", "workspace"] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(".vizzi-source-owned"), "owner").unwrap();
        for (path, text) in [("a.rs", "fn a() {}\n"), ("b.rs", "fn a() {}\n")] {
            std::fs::write(root.path().join(path), text).unwrap();
        }
        let mut config = serde_json::json!({"root":root.path(),"policy":vizzi_core::policy::PolicyConfig::default(),
            "targets":["a.rs"],"owner":"wrong","probe":"storage-fixture","mode":mode});
        assert!(
            vizzi::source_storage_probe::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {})
                .is_err()
        );
        config["owner"] = "owner".into();
        let report =
            vizzi::source_storage_probe::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {})
                .unwrap();
        let phases = report["phases"].as_array().unwrap();
        assert_eq!(phases.len(), 9);
        assert_eq!(phases[0]["current"]["unique_sources"], 2);
        assert_eq!(phases[1]["current"], phases[0]["current"]);
        assert_eq!(phases[1]["shared_reopened_sources"], 2);
        assert_eq!(
            phases[5]["publication"]["unique_sources"],
            if mode == "headers" { 5 } else { 8 }
        );
        assert_eq!(
            phases[5]["including_external"]["unique_sources"],
            if mode == "headers" { 6 } else { 10 }
        );
        assert_eq!(phases[6]["publication"], phases[6]["including_external"]);
        assert_eq!(phases[8]["publication"]["unique_sources"], 0);
        assert_eq!(report["expired_from_publication"], true);
        assert_eq!(report["external_pin_bytes_match"], true);
        assert_eq!(report["restored_snapshot_matches"], true);
        assert_eq!(report["released_handles_dead"], true);
        assert_eq!(
            std::fs::read_to_string(root.path().join("a.rs")).unwrap(),
            "fn a() {}\n"
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("b.rs")).unwrap(),
            "fn a() {}\n"
        );
    }
}

#[test]
fn source_storage_maintenance_probe_reads_and_releases_an_independent_prefix_pin() {
    let owned = tempfile::tempdir().unwrap();
    let root = owned.path().join("source");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(".vizzi-source-owned"), "owner").unwrap();
    std::fs::write(root.join("a.rs"), "fn original() {}\n").unwrap();
    let config = serde_json::json!({"root":root,"policy":vizzi_core::policy::PolicyConfig::default(),
        "targets":["a.rs"],"owner":"owner","probe":"maintenance-fixture","mode":"workspace",
        "spool_mib":1,"maintenance_reads":true});
    let report =
        vizzi::source_storage_probe::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {})
            .unwrap();
    assert_eq!(report["reader_probe_bytes"], 65536);
    assert_eq!(report["reader_released"], true);
    assert_eq!(report["spool"]["file_bytes"], 0);
    assert_eq!(report["spool"]["allocated_bytes"], 0);
    assert_eq!(report["phases"][8]["spool"]["allocated_bytes"], 65536);
    assert!(
        !report["phases"][8]["maintenance_reads"]["samples"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(report["phases"][9]["phase"], "reader_released");
}
