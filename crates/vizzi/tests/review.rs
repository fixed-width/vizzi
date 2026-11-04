//! Bounded review publication/retention behaviour, without Git or host transports.

use std::{
    io,
    sync::{Arc, Mutex},
};
use vizzi::{
    review::{ReviewCandidate, ReviewError, ReviewStore},
    review_source::{
        CaptureError, CaptureLimits, CaptureNotice, CapturePolicy, ReadEntry, ReviewInput,
    },
};
use vizzi_core::{
    comparison::{EntryKind, ReviewPath, Side},
    model::WorkspaceId,
    projection::WindowQuery,
};

fn path() -> ReviewPath {
    ReviewPath::parse(b"a.rs".to_vec()).unwrap()
}
fn policy() -> CapturePolicy {
    CapturePolicy::scope(None, vec![])
}

struct Memory {
    workspace: WorkspaceId,
    bytes: Vec<u8>,
    failed: bool,
    read: Option<Box<dyn FnMut(usize)>>,
    path: ReviewPath,
    kind: EntryKind,
    notices: Vec<CaptureNotice>,
}
impl Memory {
    fn text(text: &str) -> Self {
        Self {
            workspace: WorkspaceId::derive(&[b"review-workspace"]),
            bytes: text.as_bytes().to_vec(),
            failed: false,
            read: None,
            path: path(),
            kind: EntryKind::Regular { executable: false },
            notices: vec![],
        }
    }
}
impl ReviewInput for Memory {
    fn workspace(&self) -> &WorkspaceId {
        &self.workspace
    }
    fn paths(
        &mut self,
        _: CaptureLimits,
        _: &dyn Fn() -> bool,
    ) -> Result<Vec<ReviewPath>, CaptureError> {
        if self.failed {
            return Err(CaptureError::Io(io::Error::other("fixture")));
        }
        Ok(vec![self.path.clone()])
    }
    fn read(
        &mut self,
        _: &ReviewPath,
        limit: usize,
        _: &dyn Fn() -> bool,
    ) -> Result<ReadEntry, CaptureError> {
        if let Some(read) = &mut self.read {
            read(limit);
        }
        if self.bytes.len() > limit {
            return Err(CaptureError::Limit("content bytes"));
        }
        Ok(ReadEntry::Present {
            kind: self.kind,
            bytes: self.bytes.clone(),
        })
    }
    fn notices(&self) -> Vec<CaptureNotice> {
        self.notices.clone()
    }
}

fn populate(candidate: &mut ReviewCandidate, base: &str, working: &str) {
    candidate
        .capture_base(
            &mut Memory::text(base),
            &policy(),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(
            &mut Memory::text(working),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
}
fn create(store: &ReviewStore, base: &str, working: &str) -> String {
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    populate(&mut candidate, base, working);
    candidate.publish().unwrap()
}

#[test]
fn held_queries_keep_old_bytes_and_refuse_a_third_pair_until_release() {
    let store = ReviewStore::open(6).unwrap();
    let accounting = store.accounting();
    let first = create(&store, "ab", "cd");
    let old = store.query(&first).unwrap();
    let ticket = store.request().unwrap();
    let mut refresh = store.begin(&ticket, Some(&first)).unwrap();
    refresh
        .capture_working(&mut Memory::text("ef"), CaptureLimits::default(), &|| false)
        .unwrap();
    assert_eq!(accounting.stats().charged_bytes, 6);
    let second = refresh.publish().unwrap();
    assert!(matches!(store.query(&first), Err(ReviewError::Stale)));
    assert_eq!(
        old.window(Side::Working, &path(), &WindowQuery::default())
            .unwrap()
            .lines[0]
            .text,
        "cd"
    );
    assert_eq!(
        store
            .query(&second)
            .unwrap()
            .window(Side::Working, &path(), &WindowQuery::default())
            .unwrap()
            .lines[0]
            .text,
        "ef"
    );
    let next = store.request().unwrap();
    assert!(matches!(
        store.begin(&next, Some(&second)),
        Err(ReviewError::PairCapacity)
    ));
    assert_eq!(store.active().as_deref(), Some(second.as_str()));
    drop(old);
    assert_eq!(
        (
            accounting.stats().pair_slots,
            accounting.stats().charged_bytes
        ),
        (1, 4)
    );
    drop(store.begin(&next, Some(&second)).unwrap());
    store.close(&second).unwrap();
    assert_eq!(
        (
            accounting.stats().pair_slots,
            accounting.stats().charged_bytes
        ),
        (0, 0)
    );
}

#[test]
fn quota_is_reserved_before_io_and_failures_preserve_the_coherent_pair() {
    let store = ReviewStore::open(5).unwrap();
    let accounting = store.accounting();
    let token = create(&store, "ab", "cd");
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, Some(&token)).unwrap();
    let observed = accounting.clone();
    let mut input = Memory::text("too large");
    input.read = Some(Box::new(move |limit| {
        assert_eq!(limit, 1);
        assert_eq!(observed.stats().charged_bytes, 5);
    }));
    assert!(matches!(
        candidate.capture_working(&mut input, CaptureLimits::default(), &|| false),
        Err(ReviewError::Capture(CaptureError::Limit(_)))
    ));
    assert_eq!(accounting.stats().charged_bytes, 4);
    assert_eq!(store.active().as_deref(), Some(token.as_str()));
    drop(candidate);
    assert_eq!(accounting.stats().pair_slots, 1);
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    let mut fail = Memory::text("unused");
    fail.failed = true;
    assert!(
        candidate
            .capture_base(&mut fail, &policy(), CaptureLimits::default(), &|| false)
            .is_err()
    );
    assert_eq!(accounting.stats().charged_bytes, 4);
}

#[test]
fn identical_content_refresh_gets_new_lifetime_token_and_counts_distinct_storage() {
    let store = ReviewStore::open(6).unwrap();
    let account = store.accounting();
    let first = create(&store, "ab", "ab");
    let held = store.query(&first).unwrap();
    assert_eq!(account.stats().retained_bytes, 4);
    let ticket = store.request().unwrap();
    let mut next = store.begin(&ticket, Some(&first)).unwrap();
    assert!(matches!(
        next.capture_base(
            &mut Memory::text("new"),
            &policy(),
            CaptureLimits::default(),
            &|| false
        ),
        Err(ReviewError::Stage)
    ));
    next.capture_working(&mut Memory::text("ab"), CaptureLimits::default(), &|| false)
        .unwrap();
    let second = next.publish().unwrap();
    let current = store.query(&second).unwrap();
    assert_ne!(first, second);
    assert_eq!(held.comparison_id(), current.comparison_id());
    assert_eq!(
        (account.stats().pair_slots, account.stats().retained_bytes),
        (2, 6)
    );
    drop(held);
    assert_eq!(account.stats().retained_bytes, 4);
    store.close(&second).unwrap();
    assert_eq!(account.stats().retained_bytes, 4);
    drop(current);
    assert_eq!(account.stats().retained_bytes, 0);
    let other = ReviewStore::open(4).unwrap();
    let other_token = create(&other, "ab", "ab");
    assert_ne!(second, other_token);
    assert!(matches!(other.query(&second), Err(ReviewError::Stale)));
}

#[test]
fn stale_candidates_and_shutdown_release_source_after_the_last_query() {
    let store = ReviewStore::open(20).unwrap();
    let accounting = store.accounting();
    let token = create(&store, "old", "current");
    let lease = store.query(&token).unwrap();
    let cloned = lease.clone();
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, Some(&token)).unwrap();
    candidate
        .capture_working(&mut Memory::text("next"), CaptureLimits::default(), &|| {
            false
        })
        .unwrap();
    let newer = store.request().unwrap();
    assert!(matches!(candidate.publish(), Err(ReviewError::Stale)));
    assert_eq!(accounting.stats().pair_slots, 1);
    let pending = store.begin(&newer, None).unwrap();
    drop(store);
    assert!(matches!(
        pending.publish(),
        Err(ReviewError::Stale) | Err(ReviewError::Stage)
    ));
    assert_eq!(
        lease
            .window(Side::Base, &path(), &WindowQuery::default())
            .unwrap()
            .lines[0]
            .text,
        "old"
    );
    drop(lease);
    assert_eq!(accounting.stats().pair_slots, 1);
    drop(cloned);
    assert_eq!(
        (
            accounting.stats().pair_slots,
            accounting.stats().charged_bytes
        ),
        (0, 0)
    );
}

#[test]
fn newer_intent_during_capture_cancels_without_holding_publication_locks() {
    let store = Arc::new(ReviewStore::open(8).unwrap());
    let account = store.accounting();
    let token = create(&store, "ab", "cd");
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, Some(&token)).unwrap();
    let newer = Arc::new(Mutex::new(None));
    let record = newer.clone();
    let callback_store = store.clone();
    let observed = account.clone();
    let mut source = Memory::text("ef");
    source.read = Some(Box::new(move |_| {
        assert_eq!(callback_store.active().as_deref(), Some(token.as_str()));
        assert_eq!(observed.stats().charged_bytes, 8);
        *record.lock().unwrap() = Some(callback_store.request().unwrap());
    }));
    assert!(matches!(
        candidate.capture_working(&mut source, CaptureLimits::default(), &|| false),
        Err(ReviewError::Stale)
    ));
    assert_eq!(account.stats().charged_bytes, 4);
    let latest = newer.lock().unwrap().clone().unwrap();
    assert!(matches!(store.begin(&latest, None), Err(ReviewError::Busy)));
    drop(candidate);
    drop(store.begin(&latest, None).unwrap());
    assert_eq!(account.stats().pair_slots, 1);
}

#[test]
fn invalid_stages_foreign_tickets_and_close_cannot_publish_or_release_the_active_pair() {
    let store = ReviewStore::open(8).unwrap();
    let other = ReviewStore::open(8).unwrap();
    let foreign = other.request().unwrap();
    assert!(matches!(
        store.begin(&foreign, None),
        Err(ReviewError::Stale)
    ));
    let token = create(&store, "ab", "cd");
    let ticket = store.request().unwrap();
    assert!(matches!(
        store.begin(&ticket, Some("unknown")),
        Err(ReviewError::Stale)
    ));
    let mut candidate = store.begin(&ticket, None).unwrap();
    assert!(matches!(store.begin(&ticket, None), Err(ReviewError::Busy)));
    assert!(matches!(
        candidate.capture_working(&mut Memory::text("x"), CaptureLimits::default(), &|| false),
        Err(ReviewError::Stage)
    ));
    assert!(matches!(candidate.publish(), Err(ReviewError::Stage)));
    let mut refresh = store.begin(&ticket, Some(&token)).unwrap();
    refresh
        .capture_working(&mut Memory::text("xy"), CaptureLimits::default(), &|| false)
        .unwrap();
    assert!(matches!(
        refresh.capture_working(&mut Memory::text("zz"), CaptureLimits::default(), &|| false),
        Err(ReviewError::Stage)
    ));
    assert!(matches!(store.close("wrong"), Err(ReviewError::Stale)));
    assert_eq!(store.active().as_deref(), Some(token.as_str()));
    store.close(&token).unwrap();
    assert!(matches!(refresh.publish(), Err(ReviewError::Stale)));
    assert!(store.active().is_none());
    assert_eq!(store.accounting().stats().charged_bytes, 0);
}

#[test]
fn cancellation_and_incompatible_workspaces_drop_candidate_charges_only() {
    let store = ReviewStore::open(20).unwrap();
    let account = store.accounting();
    let token = create(&store, "ab", "cd");
    let ticket = store.request().unwrap();
    let mut refresh = store.begin(&ticket, Some(&token)).unwrap();
    assert!(matches!(
        refresh.capture_working(&mut Memory::text("next"), CaptureLimits::default(), &|| {
            true
        }),
        Err(ReviewError::Capture(CaptureError::Cancelled))
    ));
    assert_eq!(account.stats().charged_bytes, 4);
    let mut source = Memory::text("xy");
    source.workspace = WorkspaceId::derive(&[b"wrong-workspace"]);
    refresh
        .capture_working(&mut source, CaptureLimits::default(), &|| false)
        .unwrap();
    assert!(matches!(refresh.publish(), Err(ReviewError::Model(_))));
    assert_eq!(
        (account.stats().charged_bytes, account.stats().pair_slots),
        (4, 1)
    );
    assert_eq!(store.active().as_deref(), Some(token.as_str()));
}

#[test]
fn refresh_keeps_scope_and_bounded_query_outputs_do_not_keep_source_owners() {
    let store = ReviewStore::open(4).unwrap();
    let account = store.accounting();
    let excluded = CapturePolicy::scope(None, vec![path()]);
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    candidate
        .capture_base(
            &mut Memory::text("too large"),
            &excluded,
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(
            &mut Memory::text("too large"),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    let token = candidate.publish().unwrap();
    assert_eq!(account.stats().retained_bytes, 0);
    let ticket = store.request().unwrap();
    let mut refresh = store.begin(&ticket, Some(&token)).unwrap();
    refresh
        .capture_working(
            &mut Memory::text("still excluded"),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    let current = refresh.publish().unwrap();
    let lease = store.query(&current).unwrap();
    assert!(matches!(
        lease.window(Side::Base, &path(), &WindowQuery::default()),
        Err(ReviewError::SourceUnavailable)
    ));
    assert!(lease.notices(Side::Base).is_empty());
    assert!(lease.notices(Side::Working).is_empty());
    drop(lease);
    store.close(&current).unwrap();
    let token = create(&store, "ab", "cd");
    let lease = store.query(&token).unwrap();
    let window = lease
        .window(
            Side::Base,
            &path(),
            &WindowQuery {
                line_count: Some(usize::MAX),
                ..WindowQuery::default()
            },
        )
        .unwrap();
    store.close(&token).unwrap();
    drop(lease);
    assert_eq!(account.stats().retained_bytes, 0);
    assert_eq!(window.lines[0].text, "ab");
}

#[test]
fn zero_byte_reviews_and_retired_query_clones_have_no_reference_cycle() {
    let store = ReviewStore::open(0).unwrap();
    let account = store.accounting();
    let token = create(&store, "", "");
    let lease = store.query(&token).unwrap();
    let copy = lease.clone();
    assert_eq!(
        (account.stats().pair_slots, account.stats().retained_bytes),
        (1, 0)
    );
    drop(store);
    std::thread::spawn(move || {
        assert_eq!(copy.token(), token);
        let window = copy
            .window(Side::Base, &path(), &WindowQuery::default())
            .unwrap();
        assert_eq!(window.line_total, 1);
        assert_eq!(window.lines.len(), 1);
        assert_eq!(window.lines[0].text, "");
    })
    .join()
    .unwrap();
    assert_eq!(account.stats().pair_slots, 1);
    drop(lease);
    assert_eq!(account.stats().pair_slots, 0);
}

#[test]
fn comparison_query_runs_hunks_without_transferring_snapshot_ownership() {
    let store = ReviewStore::open(64).unwrap();
    let token = create(&store, "before\n", "after\n");
    let lease = store.query(&token).unwrap();
    let diff = lease
        .text_diff(
            &path(),
            vizzi_core::text_diff::DiffLimits::default(),
            &|| false,
        )
        .unwrap();
    assert_eq!(&diff.comparison, lease.comparison_id());
    assert_eq!(diff.hunks.len(), 1);
    store.close(&token).unwrap();
    drop(lease);
    assert_eq!(store.accounting().stats().pair_slots, 0);
    assert_eq!(store.accounting().stats().charged_bytes, 0);
    assert_eq!(diff.edits.len(), 1);
}

#[test]
fn incomplete_candidates_release_captured_baselines_and_allow_retry() {
    let store = ReviewStore::open(4).unwrap();
    let account = store.accounting();
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    candidate
        .capture_base(
            &mut Memory::text("ab"),
            &policy(),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    assert_eq!(account.stats().charged_bytes, 2);
    assert!(matches!(candidate.publish(), Err(ReviewError::Stage)));
    assert_eq!(
        (account.stats().pair_slots, account.stats().charged_bytes),
        (0, 0)
    );
    let mut retry = store.begin(&ticket, None).unwrap();
    populate(&mut retry, "ab", "cd");
    let token = retry.publish().unwrap();
    assert_eq!(store.active().as_deref(), Some(token.as_str()));
}

#[test]
fn literal_values_are_charged_but_discarded_unrepresentable_bodies_are_not_retained() {
    use vizzi::review_source::NoticeKind;
    let store = ReviewStore::open(6).unwrap();
    let account = store.accounting();
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    let mut base = Memory::text("");
    base.notices = vec![CaptureNotice {
        path: path(),
        kind: NoticeKind::UnmergedIndex,
    }];
    candidate
        .capture_base(&mut base, &policy(), CaptureLimits::default(), &|| false)
        .unwrap();
    let mut working = Memory::text("target");
    working.kind = EntryKind::Symlink;
    working.notices = vec![CaptureNotice {
        path: path(),
        kind: NoticeKind::NestedContentsNotCaptured,
    }];
    candidate
        .capture_working(&mut working, CaptureLimits::default(), &|| false)
        .unwrap();
    let token = candidate.publish().unwrap();
    let lease = store.query(&token).unwrap();
    assert_eq!(account.stats().retained_bytes, 6);
    assert_eq!(lease.notices(Side::Base)[0].kind, NoticeKind::UnmergedIndex);
    assert_eq!(
        lease.notices(Side::Working)[0].kind,
        NoticeKind::NestedContentsNotCaptured
    );
    assert!(matches!(
        lease.window(Side::Working, &path(), &WindowQuery::default()),
        Err(ReviewError::SourceUnavailable)
    ));
    store.close(&token).unwrap();
    drop(lease);
    assert_eq!(account.stats().charged_bytes, 0);
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    let mut opaque = Memory::text("abc");
    opaque.path = ReviewPath::parse(b"non-utf8-\xff".to_vec()).unwrap();
    candidate
        .capture_base(&mut opaque, &policy(), CaptureLimits::default(), &|| false)
        .unwrap();
    assert_eq!(account.stats().retained_bytes, 0);
    candidate
        .capture_working(&mut opaque, CaptureLimits::default(), &|| false)
        .unwrap();
    let token = candidate.publish().unwrap();
    assert_eq!(account.stats().retained_bytes, 0);
    assert_eq!(store.query(&token).unwrap().entries().count(), 1);
}

#[test]
fn a_panicking_input_releases_reservation_and_leaves_metadata_locks_usable() {
    let store = ReviewStore::open(8).unwrap();
    let account = store.accounting();
    let token = create(&store, "ab", "cd");
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, Some(&token)).unwrap();
    let mut input = Memory::text("ef");
    input.read = Some(Box::new(|_| panic!("fixture input failed")));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| candidate.capture_working(
            &mut input,
            CaptureLimits::default(),
            &|| false
        )))
        .is_err()
    );
    assert_eq!(
        (
            account.stats().charged_bytes,
            account.stats().retained_bytes
        ),
        (4, 4)
    );
    candidate
        .capture_working(&mut Memory::text("ef"), CaptureLimits::default(), &|| false)
        .unwrap();
    let next = candidate.publish().unwrap();
    assert_eq!(store.active().as_deref(), Some(next.as_str()));
    assert_eq!(account.stats().retained_bytes, 4);
}
