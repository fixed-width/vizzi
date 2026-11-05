//! An investigation retains source evidence and reader assertions independently of a host.
use vizzi_core::{investigation::*, model::*};
fn source() -> SourceEvidence {
    SourceEvidence {
        workspace: WorkspaceId::derive(&[b"ws"]),
        snapshot: SnapshotId::derive(&[b"snap"]),
        file: FileId::derive(&[b"file"]),
        version: DocumentVersion::derive(&[b"version"]),
        content: ContentId::derive(&[b"fn example() {}"]),
        path: "src/a.rs".into(),
        line: 1,
        end_line: 1,
        excerpt: "fn example() {}".into(),
        truncated: false,
    }
}
#[test]
fn questions_evidence_and_reader_connections_roundtrip_with_stable_ids() {
    let mut board = Investigation::new(source().workspace);
    board.question("How does fallback work?".into()).unwrap();
    let a = board.add("Entry point".into(), source()).unwrap();
    let b = board.add("Reader contract".into(), source()).unwrap();
    board
        .annotate(a, "Calls the injected reader".into())
        .unwrap();
    board.connect(a, b, "Uses this contract".into()).unwrap();
    let saved = board.encode().unwrap();
    let loaded = Investigation::decode(&saved).unwrap();
    assert_eq!(loaded.data(), board.data());
    assert_eq!(
        loaded.evidence(a).unwrap().note,
        "Calls the injected reader"
    );
    assert_eq!(loaded.data().connections[0].from, a);
    assert_eq!(loaded.data().connections[0].to, b);
}
#[test]
fn removal_cleans_connections_without_reusing_source_ids_or_retargeting_other_evidence() {
    let mut board = Investigation::new(source().workspace);
    let a = board.add("a".into(), source()).unwrap();
    let b = board.add("b".into(), source()).unwrap();
    let retained = board.add("retained".into(), source()).unwrap();
    board.connect(a, b, "relation".into()).unwrap();
    board.connect(b, a, "incoming".into()).unwrap();
    board.connect(b, retained, "unaffected".into()).unwrap();
    board.remove(a).unwrap();
    let c = board.add("c".into(), source()).unwrap();
    assert_ne!(a, c);
    assert!(board.evidence(a).is_none());
    assert_eq!(board.data().connections.len(), 1);
    assert_eq!(board.data().connections[0].from, b);
    assert_eq!(board.data().connections[0].to, retained);
    assert_eq!(board.evidence(b).unwrap().source, source());
}
#[test]
fn invalid_requests_preserve_the_previous_investigation() {
    let mut board = Investigation::new(source().workspace);
    let a = board.add("a".into(), source()).unwrap();
    let before = board.data().clone();
    assert!(board.connect(a, a, "self".into()).is_err());
    assert!(board.connect(a, 999, "missing".into()).is_err());
    assert!(board.question("x".repeat(2049)).is_err());
    assert!(board.annotate(a, "x".repeat(2049)).is_err());
    assert!(board.remove(999).is_err());
    assert_eq!(*board.data(), before);
    let mut wrong = source();
    wrong.workspace = WorkspaceId::derive(&[b"other"]);
    assert!(board.add("wrong workspace".into(), wrong).is_err());
    assert_eq!(*board.data(), before);
}
#[test]
fn bounds_and_untrusted_serialised_data_are_validated() {
    let mut board = Investigation::new(source().workspace);
    for _ in 0..32 {
        board.add("Evidence".into(), source()).unwrap();
    }
    assert!(board.add("Overflow".into(), source()).is_err());
    let raw = board.encode().unwrap();
    for patch in [
        "schema",
        "duplicate",
        "zero_id",
        "next_id",
        "path",
        "line",
        "excerpt",
        "endpoint",
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        match patch {
            "schema" => value["schema"] = 2.into(),
            "duplicate" => value["evidence"][1]["id"] = value["evidence"][0]["id"].clone(),
            "zero_id" => value["evidence"][0]["id"] = 0.into(),
            "next_id" => value["evidence"][0]["id"] = value["next_id"].clone(),
            "path" => value["evidence"][0]["source"]["path"] = "../escape".into(),
            "line" => value["evidence"][0]["source"]["line"] = 0.into(),
            "excerpt" => value["evidence"][0]["source"]["excerpt"] = "x".repeat(4097).into(),
            _ => {
                value["connections"] = serde_json::json!([{"from":1,"to":999,"explanation":"bad"}])
            }
        };
        assert!(
            Investigation::decode(&serde_json::to_vec(&value).unwrap()).is_err(),
            "{patch}"
        );
    }
}

#[test]
fn encoded_input_limit_accepts_the_boundary_and_refuses_one_more_byte() {
    let board = Investigation::new(source().workspace);
    let mut bytes = board.encode().unwrap();
    bytes.resize(ENCODED_BYTES, b' ');
    assert_eq!(Investigation::decode(&bytes).unwrap().data(), board.data());
    bytes.push(b' ');
    assert!(Investigation::decode(&bytes).is_err());
}

#[test]
fn connection_capacity_and_removal_keep_reader_assertions_editable() {
    let mut board = Investigation::new(source().workspace);
    let ids: Vec<_> = (0..12)
        .map(|i| board.add(format!("Evidence {i}"), source()).unwrap())
        .collect();
    let mut count = 0;
    for &a in &ids {
        for &b in &ids {
            if a != b && count < 64 {
                board.connect(a, b, "relation".into()).unwrap();
                count += 1;
            }
        }
    }
    assert_eq!(board.data().connections.len(), 64);
    assert!(board.connect(ids[11], ids[10], "overflow".into()).is_err());
    let link = board.data().connections[0].clone();
    board.disconnect(link.from, link.to).unwrap();
    assert_eq!(board.data().connections.len(), 63);
    assert!(board.disconnect(link.from, link.to).is_err());
    assert_eq!(board.data().evidence.len(), 12);
}
#[test]
fn encoded_budget_refusals_are_atomic_even_for_heavily_escaped_source() {
    let mut board = Investigation::new(source().workspace);
    let mut captured = source();
    captured.excerpt = "\u{1}".repeat(4096);
    let mut refused = false;
    for _ in 0..32 {
        let before = board.data().clone();
        match board.add("Escaped source".into(), captured.clone()) {
            Ok(_) => {}
            Err(_) => {
                assert_eq!(*board.data(), before);
                refused = true;
                break;
            }
        }
    }
    assert!(refused);
    let bytes = board.encode().unwrap();
    assert!(bytes.len() <= ENCODED_BYTES);
    assert_eq!(Investigation::decode(&bytes).unwrap().data(), board.data());
}
#[test]
fn unicode_text_limits_and_invalid_paths_are_checked_before_mutation() {
    let mut board = Investigation::new(source().workspace);
    let id = board.add("界".repeat(42), source()).unwrap();
    board.annotate(id, "x".repeat(2048)).unwrap();
    board.question("x".repeat(2048)).unwrap();
    for path in ["", "/absolute", "../parent", "a//b", "a\\b", "a\0b"] {
        let mut e = source();
        e.path = path.into();
        let before = board.data().clone();
        assert!(board.add("bad".into(), e).is_err(), "{path:?}");
        assert_eq!(*board.data(), before);
    }
    assert!(board.add("界".repeat(43), source()).is_err());
    assert!(board.add("  ".into(), source()).is_err());
}
#[test]
fn exhausted_revision_and_identity_counters_cannot_partially_change_data() {
    let board = Investigation::new(source().workspace);
    let mut invalid = serde_json::to_value(board.data()).unwrap();
    invalid["next_id"] = 0.into();
    assert!(Investigation::decode(&serde_json::to_vec(&invalid).unwrap()).is_err());
    for field in ["revision", "next_id"] {
        let mut value = serde_json::to_value(board.data()).unwrap();
        value[field] = u32::MAX.into();
        let mut loaded = Investigation::decode(&serde_json::to_vec(&value).unwrap()).unwrap();
        let before = loaded.data().clone();
        assert!(loaded.add("refused".into(), source()).is_err());
        assert_eq!(*loaded.data(), before);
    }
}
