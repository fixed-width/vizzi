//! Owned-buffer accounting does not alter content or sharing semantics.

use std::sync::Arc;
use vizzi_core::model::CapturedSource;

#[test]
fn source_storage_reports_capacity_and_index_buffers_without_copying_or_touching_source() {
    let mut bytes = Vec::with_capacity(1024);
    bytes.extend_from_slice("α\r\nb\n".as_bytes());
    let capacity = bytes.capacity();
    let source = CapturedSource::capture(bytes, None);
    let observed = source.storage();
    assert_eq!(observed.source_bytes, "α\r\nb\n".len());
    assert_eq!(observed.byte_capacity, capacity);
    assert_eq!(observed.index_entries, 3);
    assert!(observed.index_capacity >= observed.index_entries);
    assert_eq!(
        observed.index_bytes,
        observed.index_capacity * std::mem::size_of::<usize>() + observed.terminator_bytes
    );
    assert_eq!(source.resident_text(), Some("α\r\nb\n"));
    assert_eq!(source.storage(), observed);
    assert!(observed.integrity_bytes >= 32);
    assert_eq!(observed.integrity_bytes % 32, 0);
    let shared = CapturedSource::capture(source.resident_bytes().unwrap().to_vec(), Some(&source));
    assert!(Arc::ptr_eq(&source, &shared));
    assert_eq!(shared.storage(), observed);
    let distinct = CapturedSource::capture(source.resident_bytes().unwrap().to_vec(), None);
    assert!(!Arc::ptr_eq(&source, &distinct));
    assert_eq!(source.content(), distinct.content());
}

#[test]
fn unsupported_and_empty_buffers_keep_their_existing_line_semantics() {
    let mut bytes = Vec::with_capacity(512);
    bytes.push(255);
    let capacity = bytes.capacity();
    let source = CapturedSource::capture(bytes, None);
    assert_eq!(source.storage().source_bytes, 1);
    assert_eq!(source.storage().byte_capacity, capacity);
    assert_eq!(source.storage().index_entries, 1);
    assert_eq!(source.line_count(), 0);
    let empty = CapturedSource::capture(Vec::new(), None);
    assert!(empty.is_empty());
    assert_eq!(empty.storage().source_bytes, 0);
    assert_eq!(empty.storage().byte_capacity, 0);
    assert_eq!(empty.storage().index_entries, 1);
    assert_eq!(empty.line_count(), 1);
}
