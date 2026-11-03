//! Streaming private capture preserves byte identity while bounding source read requests.
use std::{
    io::{self, Cursor, Read},
    sync::Arc,
};
use vizzi::source_spool::SourceSpool;
use vizzi_core::model::CapturedSource;
struct Fragmented {
    inner: Cursor<Vec<u8>>,
    fragment: usize,
    max_request: usize,
}
impl Read for Fragmented {
    fn read(&mut self, target: &mut [u8]) -> io::Result<usize> {
        self.max_request = self.max_request.max(target.len());
        let n = target.len().min(self.fragment);
        self.inner.read(&mut target[..n])
    }
}
#[test]
fn fragmented_streams_match_resident_capture_and_reuse_at_full_committed_capacity() {
    let bytes = format!("{}🙂\r\nα\r\nend\r", "x".repeat(65534)).into_bytes();
    for fragment in [1, 3, 65536] {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), bytes.len()).unwrap();
        let mut input = Fragmented {
            inner: Cursor::new(bytes.clone()),
            fragment,
            max_request: 0,
        };
        let source = spool.capture(&mut input, None, &|| false).unwrap().unwrap();
        let resident = CapturedSource::capture(bytes.clone(), None);
        assert_eq!(source.content(), resident.content());
        assert_eq!(source.encoding(), resident.encoding());
        assert_eq!(source.line_count(), resident.line_count());
        assert_eq!(source.read_all(bytes.len()).unwrap().bytes(), bytes);
        assert_eq!(source.storage().byte_capacity, 0);
        assert_eq!(input.max_request, 65536);
        assert_eq!(spool.stats().written_bytes, bytes.len());
        assert_eq!(spool.stats().staging_bytes, 0);
        let mut again = Cursor::new(bytes.clone());
        let reused = spool
            .capture(&mut again, Some(&source), &|| false)
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&source, &reused));
        assert_eq!(spool.stats().written_bytes, bytes.len());
        assert_eq!(spool.stats().staging_bytes, 0);
    }
}
#[test]
fn cancellation_and_read_failure_leave_committed_pins_and_quota_unchanged() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let root = tempfile::tempdir().unwrap();
    let spool = SourceSpool::new(root.path(), 200000).unwrap();
    let first = spool
        .capture(&mut Cursor::new(b"original"), None, &|| false)
        .unwrap()
        .unwrap();
    let calls = AtomicUsize::new(0);
    assert!(
        spool
            .capture(&mut Cursor::new(vec![b'x'; 100000]), None, &|| calls
                .fetch_add(1, Ordering::SeqCst)
                > 3)
            .unwrap()
            .is_none()
    );
    assert_eq!(spool.stats().written_bytes, 8);
    assert_eq!(spool.stats().staging_bytes, 0);
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("fixture"))
        }
    }
    assert!(spool.capture(&mut Broken, None, &|| false).is_err());
    assert_eq!(spool.stats().written_bytes, 8);
    assert_eq!(spool.stats().staging_bytes, 0);
    assert_eq!(first.read_text(100).unwrap().as_deref(), Some("original"));
}

#[test]
fn invalid_and_truncated_unicode_follow_standard_utf8_and_source_identity_rules() {
    use vizzi_core::model::{ContentId, SourceEncoding};
    let mut cases = vec![
        vec![],
        b"plain\r\nlast\r".to_vec(),
        vec![0xff],
        vec![0xf0, 0x9f, 0x99],
        vec![0xe0, 0x80, 0x80],
    ];
    for prefix in [65533, 65534, 65535, 65536] {
        let mut bytes = vec![b'a'; prefix];
        bytes.extend_from_slice("🙂\r\nβ".as_bytes());
        cases.push(bytes);
    }
    for bytes in cases {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 200000).unwrap();
        let source = spool
            .capture(&mut Cursor::new(bytes.clone()), None, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(source.content(), &ContentId::derive(&[&bytes]));
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
        assert_eq!(source.read_all(bytes.len()).unwrap().bytes(), bytes);
        assert_eq!(spool.stats().staging_bytes, 0);
    }
}

#[test]
fn oversize_staging_is_bounded_and_interrupted_reads_retry() {
    use vizzi_core::model::SourceReadError;
    let root = tempfile::tempdir().unwrap();
    let spool = SourceSpool::new(root.path(), 65536).unwrap();
    let mut oversized = Cursor::new(vec![b'x'; 65537]);
    assert_eq!(
        spool.capture(&mut oversized, None, &|| false).unwrap_err(),
        SourceReadError::Capacity
    );
    assert_eq!(spool.stats().written_bytes, 0);
    assert_eq!(spool.stats().staging_bytes, 0);
    assert!(spool.stats().peak_staging_bytes <= 65536);
    // A finite reader yields one interruption, data, and EOF.
    struct Finite(u8);
    impl Read for Finite {
        fn read(&mut self, target: &mut [u8]) -> io::Result<usize> {
            self.0 += 1;
            match self.0 {
                1 => Err(io::ErrorKind::Interrupted.into()),
                2 => {
                    target[0] = b'a';
                    Ok(1)
                }
                _ => Ok(0),
            }
        }
    }
    let source = spool
        .capture(&mut Finite(0), None, &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(source.read_text(1).unwrap().as_deref(), Some("a"));
}

#[test]
fn old_readers_continue_while_a_capture_waits_on_project_io() {
    use std::{
        sync::atomic::{AtomicBool, Ordering},
        time::{Duration, Instant},
    };
    struct Held {
        entered: Arc<AtomicBool>,
        release: Arc<AtomicBool>,
        inner: Cursor<Vec<u8>>,
    }
    impl Read for Held {
        fn read(&mut self, target: &mut [u8]) -> io::Result<usize> {
            self.entered.store(true, Ordering::Release);
            while !self.release.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(1));
            }
            self.inner.read(target)
        }
    }
    let root = tempfile::tempdir().unwrap();
    let spool = Arc::new(SourceSpool::new(root.path(), 200000).unwrap());
    let first = spool
        .capture(&mut Cursor::new(b"original"), None, &|| false)
        .unwrap()
        .unwrap();
    let entered = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let worker_spool = spool.clone();
    let mut input = Held {
        entered: entered.clone(),
        release: release.clone(),
        inner: Cursor::new(b"changed".to_vec()),
    };
    let worker = std::thread::spawn(move || worker_spool.capture(&mut input, None, &|| false));
    let started = Instant::now();
    while !entered.load(Ordering::Acquire) && started.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(entered.load(Ordering::Acquire));
    let observed = first.read_text(8);
    release.store(true, Ordering::Release);
    assert_eq!(observed.unwrap().as_deref(), Some("original"));
    assert_eq!(
        worker
            .join()
            .unwrap()
            .unwrap()
            .unwrap()
            .read_text(8)
            .unwrap()
            .as_deref(),
        Some("changed")
    );
}

#[test]
fn changed_stream_cannot_append_beyond_a_full_committed_quota() {
    use vizzi_core::model::SourceReadError;
    let root = tempfile::tempdir().unwrap();
    let spool = SourceSpool::new(root.path(), 100).unwrap();
    let original = spool
        .capture(&mut Cursor::new(vec![b'a'; 100]), None, &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(
        spool
            .capture(&mut Cursor::new(vec![b'b'; 100]), Some(&original), &|| {
                false
            })
            .unwrap_err(),
        SourceReadError::Capacity
    );
    assert_eq!(spool.stats().written_bytes, 100);
    assert_eq!(spool.stats().staging_bytes, 0);
    assert_eq!(original.read_all(100).unwrap().bytes(), vec![b'a'; 100]);
}

#[test]
fn previous_capture_does_not_hide_changed_shorter_or_longer_input() {
    use vizzi_core::model::{ContentId, SourceEncoding};
    let original = b"first\r\nlast\n";
    for bytes in [
        b"first\r\nlast!".as_slice(),
        b"first\r\n".as_slice(),
        b"first\r\nlast\nextra".as_slice(),
        &[255; 12],
        &[],
    ] {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 1000).unwrap();
        let old = spool
            .capture(&mut Cursor::new(original), None, &|| false)
            .unwrap()
            .unwrap();
        let mut input = Fragmented {
            inner: Cursor::new(bytes.to_vec()),
            fragment: original.len(),
            max_request: 0,
        };
        let changed = spool
            .capture(&mut input, Some(&old), &|| false)
            .unwrap()
            .unwrap();
        assert!(!Arc::ptr_eq(&old, &changed));
        assert_eq!(changed.content(), &ContentId::derive(&[bytes]));
        assert_eq!(changed.read_all(1000).unwrap().bytes(), bytes);
        let text = std::str::from_utf8(bytes).ok();
        assert_eq!(
            changed.encoding(),
            if text.is_some() {
                SourceEncoding::Utf8
            } else {
                SourceEncoding::Unsupported
            }
        );
        assert_eq!(
            changed.line_count(),
            text.map_or(0, |t| t.split('\n').count())
        );
        assert_eq!(old.read_all(1000).unwrap().bytes(), original);
        assert_eq!(spool.stats().written_bytes, original.len() + bytes.len());
        assert_eq!(spool.stats().staging_bytes, 0);
    }
}

#[test]
fn matching_bytes_before_eof_do_not_hide_a_read_failure_or_cancellation() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct End {
        data: Cursor<&'static [u8]>,
        cancelled: Arc<AtomicBool>,
        fail: bool,
    }
    impl Read for End {
        fn read(&mut self, target: &mut [u8]) -> io::Result<usize> {
            let count = self.data.read(target)?;
            if count == 0 {
                if self.fail {
                    return Err(io::Error::other("EOF fixture"));
                }
                self.cancelled.store(true, Ordering::SeqCst);
            }
            Ok(count)
        }
    }
    for fail in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let spool = SourceSpool::new(root.path(), 8).unwrap();
        let old = spool
            .capture(&mut Cursor::new(b"original"), None, &|| false)
            .unwrap()
            .unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut input = End {
            data: Cursor::new(b"original"),
            cancelled: cancelled.clone(),
            fail,
        };
        let result = spool.capture(&mut input, Some(&old), &|| cancelled.load(Ordering::SeqCst));
        if fail {
            assert!(result.is_err());
        } else {
            assert!(result.unwrap().is_none());
        }
        assert_eq!(spool.stats().written_bytes, 8);
        assert_eq!(spool.stats().staging_bytes, 0);
        assert_eq!(old.read_text(8).unwrap().as_deref(), Some("original"));
    }
}
