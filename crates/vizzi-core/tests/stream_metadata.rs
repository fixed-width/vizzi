//! Resident and backing metadata agree with standard UTF-8 and independent line splitting.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use vizzi_core::{
    model::{
        CapturedSource, ContentId, Document, SourceEncoding, SourceReadError, SourceReader,
        WorkspaceId,
    },
    policy::{Classification, Provenance, SourceOnlyReason},
    projection::WindowQuery,
};
#[derive(Debug)]
struct Bytes(Vec<u8>);
impl SourceReader for Bytes {
    fn read_at(&self, offset: usize, target: &mut [u8]) -> Result<(), SourceReadError> {
        let data = self
            .0
            .get(offset..offset + target.len())
            .ok_or(SourceReadError::ShortRead)?;
        target.copy_from_slice(data);
        Ok(())
    }
}
#[test]
fn streamed_metadata_has_exact_content_encoding_and_line_fragments() {
    let mut cases = vec![
        vec![],
        vec![255, 10],
        vec![0xf0, 0x9f],
        "α\r\n🙂\nend\r".as_bytes().to_vec(),
    ];
    for offset in [65533, 65534, 65535] {
        let mut bytes = vec![b'x'; offset];
        bytes.extend_from_slice("🙂\r\nβ".as_bytes());
        cases.push(bytes);
    }
    let mut crlf = String::new();
    for i in 0..100 {
        crlf.push_str(&format!("row{i}\r\n"));
    }
    cases.push(crlf.into_bytes());
    for bytes in cases {
        let source =
            CapturedSource::capture_reader(bytes.len(), Arc::new(Bytes(bytes.clone())), &|| false)
                .unwrap()
                .unwrap();
        assert_eq!(source.content(), &ContentId::derive(&[&bytes]));
        assert_eq!(source.storage().byte_capacity, 0);
        let text = std::str::from_utf8(&bytes).ok();
        assert_eq!(
            source.encoding(),
            if text.is_some() {
                SourceEncoding::Utf8
            } else {
                SourceEncoding::Unsupported
            }
        );
        assert_eq!(
            source.line_count(),
            text.map_or(0, |text| text.split('\n').count())
        );
        let file = Document::capture_source(
            WorkspaceId::derive(&[b"stream fixture"]),
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
        if let Some(text) = text {
            let rows: Vec<_> = text.split('\n').collect();
            for (index, row) in rows.iter().enumerate() {
                let window = file
                    .window(
                        &WindowQuery {
                            line_start: Some(index + 1),
                            line_count: Some(1),
                            ..Default::default()
                        },
                        None,
                    )
                    .unwrap();
                let expected = if index < rows.len() - 1 {
                    row.strip_suffix('\r').unwrap_or(row)
                } else {
                    row
                };
                assert_eq!(window.lines[0].line_bytes, expected.len());
                assert!(expected.starts_with(&window.lines[0].text));
            }
        }
    }
}
#[test]
fn metadata_read_failure_or_observed_cancellation_never_returns_partial_source() {
    let data = vec![b'x'; 100000];
    assert!(matches!(
        CapturedSource::capture_reader(100001, Arc::new(Bytes(data.clone())), &|| false),
        Err(SourceReadError::ShortRead)
    ));
    let calls = AtomicUsize::new(0);
    assert!(
        CapturedSource::capture_reader(data.len(), Arc::new(Bytes(data)), &|| calls
            .fetch_add(1, Ordering::SeqCst)
            > 2)
        .unwrap()
        .is_none()
    );
}
