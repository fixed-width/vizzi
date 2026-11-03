//! The large-file capture fixture uses bounded verification and restores no project data.
#![cfg(feature = "qualification")]
#[test]
fn large_fixture_probe_covers_whole_and_streamed_capture() {
    for mode in ["whole", "stream"] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(".vizzi-stream-owned"), "owner").unwrap();
        std::fs::write(root.path().join("large.rs"), vec![b' '; 100000]).unwrap();
        let config = serde_json::json!({"root":root.path(),"owner":"owner","mode":mode});
        let report =
            vizzi::stream_probe::run(&serde_json::to_vec(&config).unwrap(), &mut |_| {}).unwrap();
        assert_eq!(report["bounded_bytes_match"], true);
        assert_eq!(report["unchanged_shared"], true);
        assert_eq!(report["final_spool"]["written_bytes"], 100000);
        assert_eq!(report["phases"].as_array().unwrap().len(), 2);
    }
}
