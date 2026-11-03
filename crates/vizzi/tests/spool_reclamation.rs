//! Retired backing ranges are reusable only after the final owner releases them.
use std::{io::Cursor, sync::Arc};
use vizzi::source_spool::SourceSpool;
use vizzi_core::model::{CapturedSource, SourceReadError};

#[test]
fn final_source_owner_retires_a_range_and_full_backing_can_reuse_it() {
    let root = tempfile::tempdir().unwrap();
    let spool = SourceSpool::new(root.path(), 12).unwrap();
    let first = spool
        .persist(CapturedSource::capture(b"aaaa".to_vec(), None))
        .unwrap();
    let held = first.clone();
    let last = spool
        .persist(CapturedSource::capture(b"bbbbbbbb".to_vec(), None))
        .unwrap();
    drop(first);
    spool.reclaim().unwrap();
    assert!(matches!(
        spool.persist(CapturedSource::capture(b"xxxx".to_vec(), None)),
        Err(SourceReadError::Capacity)
    ));
    assert_eq!(held.read_all(4).unwrap().bytes(), b"aaaa");
    drop(held);
    spool.reclaim().unwrap();
    assert_eq!(spool.stats().allocated_bytes, 8);
    assert_eq!(spool.stats().free_bytes, 4);
    assert_eq!(spool.stats().file_bytes, Some(12));
    let replacement = spool
        .capture(&mut Cursor::new(b"xxxx"), None, &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(replacement.read_all(4).unwrap().bytes(), b"xxxx");
    assert_eq!(last.read_all(8).unwrap().bytes(), b"bbbbbbbb");
    assert_eq!(spool.stats().written_bytes, 16);
    assert_eq!(spool.stats().extent_bytes, 12);
    drop(last);
    spool.reclaim().unwrap();
    assert_eq!(spool.stats().file_bytes, Some(4));
    drop(replacement);
    spool.reclaim().unwrap();
    assert_eq!(spool.stats().allocated_bytes, 0);
    assert_eq!(spool.stats().file_bytes, Some(0));
}

#[test]
fn split_and_adjacent_retirements_make_a_larger_contiguous_capture_possible() {
    let root = tempfile::tempdir().unwrap();
    let spool = SourceSpool::new(root.path(), 12).unwrap();
    let a = spool
        .persist(CapturedSource::capture(vec![b'a'; 4], None))
        .unwrap();
    let b = spool
        .persist(CapturedSource::capture(vec![b'b'; 4], None))
        .unwrap();
    let c = spool
        .persist(CapturedSource::capture(vec![b'c'; 4], None))
        .unwrap();
    drop(a);
    drop(b);
    let d = spool
        .persist(CapturedSource::capture(vec![b'd'; 6], None))
        .unwrap();
    let e = spool
        .persist(CapturedSource::capture(vec![b'e'; 2], None))
        .unwrap();
    assert_eq!(c.read_all(4).unwrap().bytes(), b"cccc");
    assert_eq!(d.read_all(6).unwrap().bytes(), b"dddddd");
    assert_eq!(e.read_all(2).unwrap().bytes(), b"ee");
    assert_eq!(spool.stats().free_bytes, 0);
}

#[test]
fn repeated_stream_churn_exceeds_cumulative_capacity_while_a_pin_stays_readable() {
    let root = tempfile::tempdir().unwrap();
    let spool = SourceSpool::new(root.path(), 32).unwrap();
    let pin = spool
        .capture(&mut Cursor::new(vec![b'p'; 8]), None, &|| false)
        .unwrap()
        .unwrap();
    for byte in 0..100 {
        let source = spool
            .capture(&mut Cursor::new(vec![byte; 16]), None, &|| false)
            .unwrap()
            .unwrap();
        let clone = source.clone();
        drop(source);
        spool.reclaim().unwrap();
        assert_eq!(clone.read_all(16).unwrap().bytes(), vec![byte; 16]);
        assert_eq!(pin.read_all(8).unwrap().bytes(), vec![b'p'; 8]);
        drop(clone);
    }
    spool.reclaim().unwrap();
    assert!(spool.stats().written_bytes > spool.stats().capacity_bytes);
    assert_eq!(spool.stats().allocated_bytes, 8);
    assert_eq!(spool.stats().file_bytes, Some(8));
    assert!(spool.stats().peak_extent_bytes <= 32);
    drop(pin);
    spool.reclaim().unwrap();
    assert_eq!(spool.stats().file_bytes, Some(0));
}

#[test]
fn empty_captures_need_no_extent_and_a_reader_survives_store_release() {
    let root = tempfile::tempdir().unwrap();
    let spool = Arc::new(SourceSpool::new(root.path(), 8).unwrap());
    let empty = spool
        .persist(CapturedSource::capture(vec![], None))
        .unwrap();
    let source = spool
        .persist(CapturedSource::capture(b"original".to_vec(), None))
        .unwrap();
    assert!(empty.read_all(0).unwrap().bytes().is_empty());
    drop(empty);
    spool.reclaim().unwrap();
    assert_eq!(spool.stats().allocated_bytes, 8);
    drop(spool);
    assert_eq!(source.read_text(8).unwrap().as_deref(), Some("original"));
}

#[test]
fn fragmentation_refuses_a_contiguous_capture_without_overwriting_a_pin() {
    let root = tempfile::tempdir().unwrap();
    let spool = SourceSpool::new(root.path(), 12).unwrap();
    let a = spool
        .persist(CapturedSource::capture(vec![b'a'; 4], None))
        .unwrap();
    let b = spool
        .persist(CapturedSource::capture(vec![b'b'; 4], None))
        .unwrap();
    let c = spool
        .persist(CapturedSource::capture(vec![b'c'; 4], None))
        .unwrap();
    drop(a);
    drop(c);
    assert!(matches!(
        spool.persist(CapturedSource::capture(vec![b'd'; 5], None)),
        Err(SourceReadError::Capacity)
    ));
    assert_eq!(b.read_all(4).unwrap().bytes(), b"bbbb");
    drop(b);
    let d = spool
        .persist(CapturedSource::capture(vec![b'd'; 5], None))
        .unwrap();
    assert_eq!(d.read_all(5).unwrap().bytes(), b"ddddd");
}
