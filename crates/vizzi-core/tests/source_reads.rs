//! Bounded, fallible source access preserves verified captured identity.
use std::{
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use vizzi_core::model::{CapturedSource, SourceReadError, SourceReader};
#[derive(Debug)]
struct Reader {
    bytes: Vec<u8>,
    fail: AtomicBool,
    reads: AtomicUsize,
}
impl SourceReader for Reader {
    fn read_at(&self, offset: usize, target: &mut [u8]) -> Result<(), SourceReadError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.fail.load(Ordering::SeqCst) {
            return Err(SourceReadError::Io("fixture".into()));
        }
        let end = offset
            .checked_add(target.len())
            .ok_or(SourceReadError::InvalidRange)?;
        let data = self
            .bytes
            .get(offset..end)
            .ok_or(SourceReadError::ShortRead)?;
        target.copy_from_slice(data);
        Ok(())
    }
}
#[test]
fn resident_and_backed_reads_bind_to_the_same_bytes_and_enforce_limits() {
    let bytes = "α\r\n🙂 second\n".as_bytes();
    let resident = CapturedSource::capture(bytes.to_vec(), None);
    let reader = Arc::new(Reader {
        bytes: bytes.to_vec(),
        fail: AtomicBool::new(false),
        reads: AtomicUsize::new(0),
    });
    let backed = resident.clone().with_backing(reader.clone());
    assert_eq!(backed.content(), resident.content());
    assert!(!backed.is_empty());
    assert_eq!(backed.encoding(), resident.encoding());
    assert_eq!(backed.line_count(), 3);
    assert_eq!(backed.storage().byte_capacity, 0);
    assert_eq!(reader.reads.load(Ordering::SeqCst), 0);
    assert_eq!(backed.read_all(bytes.len()).unwrap().bytes(), bytes);
    assert_eq!(
        resident.read_all(bytes.len()).unwrap().bytes().as_ptr(),
        resident.resident_bytes().unwrap().as_ptr()
    );
    assert!(matches!(
        backed.read_all(bytes.len() - 1),
        Err(SourceReadError::Capacity)
    ));
    let mut session = backed.reader();
    assert_eq!(session.read(0..2, 2).unwrap().as_ref(), "α".as_bytes());
    assert!(matches!(
        session.read(Range { start: 2, end: 1 }, 4),
        Err(SourceReadError::InvalidRange)
    ));
    assert!(matches!(
        session.read(0..bytes.len() + 1, 100),
        Err(SourceReadError::InvalidRange)
    ));
    assert!(matches!(
        session.read(0..3, 2),
        Err(SourceReadError::Capacity)
    ));
    assert_eq!(session.read(bytes.len()..bytes.len(), 0).unwrap().len(), 0);
    reader.fail.store(true, Ordering::SeqCst);
    assert!(matches!(
        backed.reader().read(0..2, 2),
        Err(SourceReadError::Io(_))
    ));
}
#[test]
fn corrupt_or_short_backing_is_an_error_instead_of_empty_or_unsupported_source() {
    for data in [b"incorrect".to_vec(), b"x".to_vec()] {
        let source =
            CapturedSource::capture(b"correct!!".to_vec(), None).with_backing(Arc::new(Reader {
                bytes: data,
                fail: AtomicBool::new(false),
                reads: AtomicUsize::new(0),
            }));
        assert!(source.read_all(100).is_err());
        assert!(source.reader().read(Range { start: 0, end: 1 }, 1).is_err());
    }
}

#[test]
fn window_reads_preserve_utf8_crlf_and_never_need_a_whole_backing_load() {
    use vizzi_core::{
        model::{Document, WorkspaceId},
        policy::{Classification, Provenance, SourceOnlyReason},
        projection::WindowQuery,
        query::{Projection, ViewQuery},
    };
    let text = format!("{}🙂\r\n{}\nlast", "a".repeat(65534), "λ".repeat(40000));
    let resident = CapturedSource::capture(text.as_bytes().to_vec(), None);
    let reader = Arc::new(Reader {
        bytes: text.as_bytes().to_vec(),
        fail: AtomicBool::new(false),
        reads: AtomicUsize::new(0),
    });
    let backed = resident.clone().with_backing(reader.clone());
    let workspace = WorkspaceId::derive(&[b"reader fixture"]);
    let classification = Classification::SourceOnly {
        language: None,
        provenance: Provenance::Extension {
            extension: "h".into(),
        },
        reason: SourceOnlyReason::LanguageUnassigned,
    };
    let make = |source| {
        Document::capture_source(
            workspace.clone(),
            "a.h".into(),
            source,
            vec![],
            classification.clone(),
        )
        .unwrap()
    };
    let a = make(resident);
    let b = make(backed);
    let query = WindowQuery {
        column_line: Some(1),
        column: Some(65535),
        ..Default::default()
    };
    let expected = serde_json::to_value(a.window(&query, None).unwrap()).unwrap();
    let observed = serde_json::to_value(b.window(&query, None).unwrap()).unwrap();
    assert_eq!(expected, observed);
    assert_eq!(observed["lines"][0]["text"], "🙂");
    assert_eq!(observed["lines"][0]["column"], 65534);
    assert!(reader.reads.load(Ordering::SeqCst) <= 3);
    reader.fail.store(true, Ordering::SeqCst);
    assert!(matches!(
        b.window(&query, None),
        Err(SourceReadError::Io(_))
    ));
    let snapshot = vizzi_core::model::Snapshot::new(
        workspace,
        vec![Arc::new(b)],
        vizzi_core::model::CapturePolicyId::derive(&[b"fixture"]),
    )
    .unwrap();
    let summary = snapshot
        .view(&ViewQuery {
            projection: Some(Projection::Summary),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(summary.source.unwrap().text, None);
    assert!(matches!(
        snapshot.view(&ViewQuery::default()),
        Err(SourceReadError::Io(_))
    ));
}

#[test]
fn backed_fact_validation_uses_the_verified_loaded_input_and_rejects_a_different_one() {
    use vizzi_core::{
        analysis::Languages,
        model::{Document, WorkspaceId},
        policy::{Classification, Provenance},
    };
    let bytes = b"fn example() {}";
    let source = CapturedSource::capture(bytes.to_vec(), None).with_backing(Arc::new(Reader {
        bytes: bytes.to_vec(),
        fail: AtomicBool::new(false),
        reads: AtomicUsize::new(0),
    }));
    let analysis = Languages::builtin()
        .unwrap()
        .select("rust")
        .unwrap()
        .create()
        .unwrap()
        .analyze(bytes)
        .unwrap();
    let class = Classification::Assigned {
        language: "rust".into(),
        provenance: Provenance::Extension {
            extension: "rs".into(),
        },
    };
    let workspace = WorkspaceId::derive(&[b"loaded fixture"]);
    assert!(
        Document::capture_source(
            workspace.clone(),
            "a.rs".into(),
            source.clone(),
            vec![analysis.clone()],
            class.clone()
        )
        .is_err()
    );
    let input = source.read_all(100).unwrap();
    let document = Document::capture_loaded(
        workspace.clone(),
        "a.rs".into(),
        source.clone(),
        &input,
        vec![analysis.clone()],
        class.clone(),
    )
    .unwrap();
    assert_eq!(document.analyses(), std::slice::from_ref(&analysis));
    let other = CapturedSource::capture(b"different".to_vec(), None);
    let wrong = other.read_all(100).unwrap();
    assert!(
        Document::capture_loaded(
            workspace,
            "a.rs".into(),
            source.clone(),
            &wrong,
            vec![analysis],
            class
        )
        .is_err()
    );
}

#[test]
fn read_sessions_reuse_two_blocks_and_do_not_cache_an_entire_file() {
    let mut bytes = vec![b'a'; 65536];
    bytes.extend(std::iter::repeat_n(b'b', 65536));
    bytes.extend(std::iter::repeat_n(b'c', 65536));
    let reader = Arc::new(Reader {
        bytes: bytes.clone(),
        fail: AtomicBool::new(false),
        reads: AtomicUsize::new(0),
    });
    let source = CapturedSource::capture(bytes, None).with_backing(reader.clone());
    let mut session = source.reader();
    assert_eq!(session.read(0..1, 1).unwrap().as_ref(), b"a");
    assert_eq!(session.read(65536..65537, 1).unwrap().as_ref(), b"b");
    assert_eq!(session.read(0..1, 1).unwrap().as_ref(), b"a");
    assert_eq!(
        reader.reads.load(Ordering::SeqCst),
        2,
        "both recent blocks should be reusable"
    );
    assert_eq!(session.read(131072..131073, 1).unwrap().as_ref(), b"c");
    for (offset, expected) in [(0, b'a'), (65536, b'b'), (131072, b'c')] {
        assert_eq!(
            session.read(offset..offset + 1, 1).unwrap().as_ref(),
            &[expected]
        );
    }
    assert!(
        reader.reads.load(Ordering::SeqCst) >= 4,
        "three distinct blocks must not all remain cached"
    );
}

#[test]
fn crlf_metadata_covers_many_lines_and_preserves_a_bare_final_carriage_return() {
    use vizzi_core::{
        model::{Document, WorkspaceId},
        policy::{Classification, Provenance, SourceOnlyReason},
        projection::WindowQuery,
    };
    let mut text = String::new();
    let mut expected = Vec::new();
    for index in 0..80 {
        let row = format!("row{index}");
        text.push_str(&row);
        text.push_str(if index % 3 == 1 { "\r\n" } else { "\n" });
        expected.push(row);
    }
    text.push_str("tail\r");
    expected.push("tail\r".into());
    let source =
        CapturedSource::capture(text.as_bytes().to_vec(), None).with_backing(Arc::new(Reader {
            bytes: text.as_bytes().to_vec(),
            fail: AtomicBool::new(false),
            reads: AtomicUsize::new(0),
        }));
    let file = Document::capture_source(
        WorkspaceId::derive(&[b"crlf fixture"]),
        "a.h".into(),
        source,
        vec![],
        Classification::SourceOnly {
            language: None,
            provenance: Provenance::Extension {
                extension: "h".into(),
            },
            reason: SourceOnlyReason::LanguageUnassigned,
        },
    )
    .unwrap();
    assert!(
        file.captured().storage().terminator_bytes
            <= 2 * file.captured().line_count().div_ceil(8).max(8),
        "CRLF flags must remain compact"
    );
    let window = file.window(&WindowQuery::default(), None).unwrap();
    assert_eq!(
        window
            .lines
            .iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>(),
        expected.iter().map(String::as_str).collect::<Vec<_>>()
    );
    for (row, text) in window.lines.iter().zip(expected) {
        assert_eq!(row.line_bytes, text.len());
    }
}

#[test]
fn default_bulk_allowance_accepts_32_mib_and_rejects_the_next_byte() {
    use vizzi_core::model::BULK_SOURCE_LIMIT;
    let source = CapturedSource::capture(vec![b'x'; 32 * 1024 * 1024], None);
    assert_eq!(
        source.read_text(BULK_SOURCE_LIMIT).unwrap().unwrap().len(),
        32 * 1024 * 1024
    );
    let larger = CapturedSource::capture(vec![b'x'; 32 * 1024 * 1024 + 1], None);
    assert!(matches!(
        larger.read_text(BULK_SOURCE_LIMIT),
        Err(SourceReadError::Capacity)
    ));
}
