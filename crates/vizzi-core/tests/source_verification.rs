//! Stream verification uses actual bytes and exact length independently of backing or chunking.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use vizzi_core::model::{CapturedSource, ContentId, SourceReadError, SourceReader};

#[test]
fn verification_matches_independent_content_for_every_partition_and_length() {
    let cases = [
        vec![],
        b"a\0b\r\n".to_vec(),
        vec![255, 0xf0, 0x9f],
        format!("{}🙂\r\nend", "x".repeat(65535)).into_bytes(),
    ];
    for bytes in cases {
        let source = CapturedSource::capture(bytes.clone(), None);
        let mut changed = bytes.clone();
        if let Some(last) = changed.last_mut() {
            *last ^= 1;
        } else {
            changed.push(0);
        }
        let mut extra = bytes.clone();
        extra.push(0);
        let mut candidates = vec![bytes.clone(), changed, extra];
        if !bytes.is_empty() {
            candidates.push(bytes[..bytes.len() - 1].to_vec());
        }
        for candidate in candidates {
            for partition in [1, 3, 65536] {
                let mut verifier = source.verifier();
                for chunk in candidate.chunks(partition) {
                    verifier.update(chunk);
                    verifier.update(&[]);
                }
                assert_eq!(
                    verifier.matches(),
                    source.content() == &ContentId::derive(&[&candidate])
                );
            }
        }
        let mut verifier = source.verifier();
        verifier.update(&bytes);
        verifier.update(b"extra");
        verifier.update(&[]);
        assert!(!verifier.matches());
    }
}

#[derive(Debug)]
struct Unavailable(Arc<AtomicUsize>);
impl SourceReader for Unavailable {
    fn read_at(&self, _: usize, _: &mut [u8]) -> Result<(), SourceReadError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(SourceReadError::Io("unavailable fixture".into()))
    }
}
#[test]
fn verifying_previous_identity_needs_no_backing_reads() {
    let reads = Arc::new(AtomicUsize::new(0));
    let source = CapturedSource::capture(b"original".to_vec(), None)
        .with_backing(Arc::new(Unavailable(reads.clone())));
    let mut verifier = source.verifier();
    verifier.update(b"ori");
    verifier.update(b"ginal");
    assert!(verifier.matches());
    assert_eq!(reads.load(Ordering::SeqCst), 0);
}
