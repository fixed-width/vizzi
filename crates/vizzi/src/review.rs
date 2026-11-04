//! Bounded review ownership and publication, independent of transports and SCMs.

use crate::review_source::{Acquired, CaptureError, CaptureLimits, CapturePolicy, ReviewInput};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use vizzi_core::{
    comparison::{ComparedEntry, Comparison, ReviewPath, Side},
    model::{ComparisonId, ModelError, SourceReadError},
    projection::{Window, WindowQuery},
    text_diff::{DiffError, DiffLimits, TextDiff},
};

/// Explicit admission/publication failure; an existing active pair is preserved.
#[derive(Debug, thiserror::Error)]
pub enum ReviewError {
    /// Ticket/token belongs to another request, closed pair or store lifetime.
    #[error("review request or token expired")]
    Stale,
    /// Another candidate still owns capture admission.
    #[error("a review capture is still outstanding")]
    Busy,
    /// Active, private or query-held retired pairs occupy both slots.
    #[error("both review pair slots are occupied")]
    PairCapacity,
    /// Capture/publication operations were called out of order.
    #[error("review capture stage is invalid")]
    Stage,
    /// Request sequence cannot wrap and revive old tokens.
    #[error("review request sequence exhausted")]
    Exhausted,
    /// The native input failed within the reserved capture allowance.
    #[error(transparent)]
    Capture(#[from] CaptureError),
    /// Captured sides cannot form one coherent comparison.
    #[error(transparent)]
    Model(#[from] ModelError),
    /// No captured source document is available on the selected side.
    #[error("review source is absent or not retained")]
    SourceUnavailable,
    /// Reading a held immutable document failed.
    #[error(transparent)]
    Source(#[from] SourceReadError),
}

/// Request intent scoped to one store lifetime; it never owns captured source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureTicket {
    token: String,
}

/// Metadata counters; byte counts exclude facts, source indexes and allocator overhead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewStats {
    /// Configured logical source-byte allowance.
    pub capacity_bytes: usize,
    /// Retained bytes plus outstanding pre-capture reservations.
    pub charged_bytes: usize,
    /// Actual logical source bytes retained by admitted input owners.
    pub retained_bytes: usize,
    /// Active, private and externally held retired pair owners, at most two.
    pub pair_slots: usize,
}

#[derive(Default)]
struct Bytes {
    charged: usize,
    retained: usize,
}
struct ByteBudget {
    capacity: usize,
    state: Mutex<Bytes>,
}
#[derive(Default)]
struct Slots {
    used: Mutex<usize>,
}

/// Read-only accounting handle that does not keep captured pairs alive.
#[derive(Clone)]
pub struct ReviewAccounting {
    bytes: Arc<ByteBudget>,
    slots: Arc<Slots>,
}
impl ReviewAccounting {
    /// Observes admission/retention counters without capturing source.
    pub fn stats(&self) -> ReviewStats {
        let bytes = lock(&self.bytes.state);
        ReviewStats {
            capacity_bytes: self.bytes.capacity,
            charged_bytes: bytes.charged,
            retained_bytes: bytes.retained,
            pair_slots: *lock(&self.slots.used),
        }
    }
}

struct State {
    sequence: u64,
    desired: Option<CaptureTicket>,
    building: Option<CaptureTicket>,
    current: Option<Arc<Pair>>,
}

/// One active comparison, with two total pair slots and pre-capture byte admission.
pub struct ReviewStore {
    nonce: String,
    state: Arc<Mutex<State>>,
    accounting: ReviewAccounting,
}

impl ReviewStore {
    /// Opens a fresh token namespace with an explicit source allowance (zero is valid).
    pub fn open(source_bytes: usize) -> Result<Self, getrandom::Error> {
        let mut nonce = [0; 32];
        getrandom::fill(&mut nonce)?;
        Ok(Self {
            nonce: blake3::Hash::from_bytes(nonce).to_hex().to_string(),
            state: Arc::new(Mutex::new(State {
                sequence: 0,
                desired: None,
                building: None,
                current: None,
            })),
            accounting: ReviewAccounting {
                bytes: Arc::new(ByteBudget {
                    capacity: source_bytes,
                    state: Mutex::new(Bytes::default()),
                }),
                slots: Arc::new(Slots::default()),
            },
        })
    }
    /// Creates newer intent, cooperatively invalidating any older capture ticket.
    pub fn request(&self) -> Result<CaptureTicket, ReviewError> {
        let mut state = lock(&self.state);
        state.sequence = state
            .sequence
            .checked_add(1)
            .ok_or(ReviewError::Exhausted)?;
        let ticket = CaptureTicket {
            token: format!("{}:{}", self.nonce, state.sequence),
        };
        state.desired = Some(ticket.clone());
        Ok(ticket)
    }
    /// Admits one candidate; a refresh requires the current token and reuses its base.
    pub fn begin(
        &self,
        ticket: &CaptureTicket,
        refresh: Option<&str>,
    ) -> Result<ReviewCandidate, ReviewError> {
        let mut state = lock(&self.state);
        if state.desired.as_ref() != Some(ticket) {
            return Err(ReviewError::Stale);
        }
        if state.building.is_some() {
            return Err(ReviewError::Busy);
        }
        let base = match refresh {
            Some(token) => Some(
                state
                    .current
                    .as_ref()
                    .filter(|pair| pair.token == token)
                    .ok_or(ReviewError::Stale)?
                    .base
                    .clone(),
            ),
            None => None,
        };
        let slot = Slot::acquire(self.accounting.slots.clone())?;
        state.building = Some(ticket.clone());
        Ok(ReviewCandidate {
            state: Arc::downgrade(&self.state),
            ticket: ticket.clone(),
            accounting: self.accounting.clone(),
            base,
            working: None,
            slot: Some(slot),
        })
    }
    /// Returns the active publication token without pinning source.
    pub fn active(&self) -> Option<String> {
        lock(&self.state)
            .current
            .as_ref()
            .map(|pair| pair.token.clone())
    }
    /// Pins exactly the active token; new requests cannot acquire a retired pair.
    pub fn query(&self, token: &str) -> Result<ReviewLease, ReviewError> {
        let state = lock(&self.state);
        let pair = state
            .current
            .as_ref()
            .filter(|pair| pair.token == token)
            .ok_or(ReviewError::Stale)?;
        Ok(ReviewLease { pair: pair.clone() })
    }
    /// Retires current ownership and invalidates outstanding capture intent.
    pub fn close(&self, token: &str) -> Result<(), ReviewError> {
        let old = {
            let mut state = lock(&self.state);
            if !state
                .current
                .as_ref()
                .is_some_and(|pair| pair.token == token)
            {
                return Err(ReviewError::Stale);
            }
            state.desired = None;
            state.current.take()
        };
        drop(old);
        Ok(())
    }
    /// Observes accounting without creating another source owner.
    pub fn accounting(&self) -> ReviewAccounting {
        self.accounting.clone()
    }
}

impl Drop for ReviewStore {
    fn drop(&mut self) {
        let old = {
            let mut state = lock(&self.state);
            state.desired = None;
            state.current.take()
        };
        drop(old);
    }
}

struct Reservation {
    budget: Arc<ByteBudget>,
    bytes: usize,
    retained: bool,
}
struct Slot {
    pool: Arc<Slots>,
}

impl Slot {
    fn acquire(pool: Arc<Slots>) -> Result<Self, ReviewError> {
        {
            let mut used = lock(&pool.used);
            if *used >= 2 {
                return Err(ReviewError::PairCapacity);
            }
            *used += 1;
        }
        Ok(Self { pool })
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        *lock(&self.pool.used) -= 1;
    }
}

impl Reservation {
    fn reserve(budget: Arc<ByteBudget>, requested: usize) -> Self {
        let bytes = {
            let mut state = lock(&budget.state);
            let bytes = requested.min(budget.capacity - state.charged);
            state.charged += bytes;
            bytes
        };
        Self {
            budget,
            bytes,
            retained: false,
        }
    }
    fn retain(&mut self, actual: usize) -> Result<(), ReviewError> {
        if actual > self.bytes {
            return Err(ReviewError::Model(ModelError(
                "retained source exceeds its reservation".into(),
            )));
        }
        let mut state = lock(&self.budget.state);
        state.charged -= self.bytes - actual;
        state.retained += actual;
        self.bytes = actual;
        self.retained = true;
        Ok(())
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        let mut state = lock(&self.budget.state);
        state.charged -= self.bytes;
        if self.retained {
            state.retained -= self.bytes;
        }
    }
}

struct InputOwner {
    capture: Acquired,
    policy: CapturePolicy,
    _charge: Reservation,
}

struct Pair {
    token: String,
    comparison: Comparison,
    base: Arc<InputOwner>,
    working: Arc<InputOwner>,
    _slot: Slot,
}

/// Private admitted capture; dropping it abandons its payloads and reservation.
pub struct ReviewCandidate {
    state: Weak<Mutex<State>>,
    ticket: CaptureTicket,
    accounting: ReviewAccounting,
    base: Option<Arc<InputOwner>>,
    working: Option<Arc<InputOwner>>,
    slot: Option<Slot>,
}

impl ReviewCandidate {
    /// Captures a new baseline; refresh candidates already own their exact held base.
    pub fn capture_base(
        &mut self,
        input: &mut dyn ReviewInput,
        policy: &CapturePolicy,
        limits: CaptureLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), ReviewError> {
        if self.base.is_some() {
            return Err(ReviewError::Stage);
        }
        self.base = Some(self.capture(input, policy, limits, cancelled)?);
        Ok(())
    }
    /// Captures working bytes under the baseline's frozen scope/policy.
    pub fn capture_working(
        &mut self,
        input: &mut dyn ReviewInput,
        limits: CaptureLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), ReviewError> {
        if self.working.is_some() {
            return Err(ReviewError::Stage);
        }
        let base = self.base.as_ref().ok_or(ReviewError::Stage)?;
        self.working = Some(self.capture(input, &base.policy, limits, cancelled)?);
        Ok(())
    }
    /// Validates and atomically replaces the pair only for the latest live request.
    pub fn publish(mut self) -> Result<String, ReviewError> {
        self.check()?;
        let base = self.base.take().ok_or(ReviewError::Stage)?;
        let working = self.working.take().ok_or(ReviewError::Stage)?;
        let comparison = Comparison::between(
            base.capture.observation.clone(),
            working.capture.observation.clone(),
        )?;
        let token = self.ticket.token.clone();
        let pair = Arc::new(Pair {
            token: token.clone(),
            comparison,
            base,
            working,
            _slot: self.slot.take().ok_or(ReviewError::Stage)?,
        });
        let state = self.state.upgrade().ok_or(ReviewError::Stale)?;
        let old = {
            let mut state = lock(&state);
            if state.desired.as_ref() != Some(&self.ticket) {
                return Err(ReviewError::Stale);
            }
            state.building = None;
            state.current.replace(pair)
        };
        drop(old);
        Ok(token)
    }

    fn check(&self) -> Result<(), ReviewError> {
        let state = self.state.upgrade().ok_or(ReviewError::Stale)?;
        let state = lock(&state);
        if state.desired.as_ref() != Some(&self.ticket) {
            Err(ReviewError::Stale)
        } else {
            Ok(())
        }
    }

    fn capture(
        &self,
        input: &mut dyn ReviewInput,
        policy: &CapturePolicy,
        limits: CaptureLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Arc<InputOwner>, ReviewError> {
        self.check()?;
        let mut charge = Reservation::reserve(self.accounting.bytes.clone(), limits.max_bytes);
        let result = crate::review_source::acquire(
            input,
            policy,
            CaptureLimits {
                max_bytes: charge.bytes,
                ..limits
            },
            &|| cancelled() || self.check().is_err(),
        );
        self.check()?;
        let capture = result?;
        let actual = retained_bytes(&capture)?;
        charge.retain(actual)?;
        Ok(Arc::new(InputOwner {
            capture,
            policy: policy.clone(),
            _charge: charge,
        }))
    }
}

impl Drop for ReviewCandidate {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            let mut state = lock(&state);
            if state.building.as_ref() == Some(&self.ticket) {
                state.building = None;
            }
        }
    }
}

/// An in-flight query retaining one exact pair until the last lease clone is dropped.
#[derive(Clone)]
pub struct ReviewLease {
    pair: Arc<Pair>,
}

impl ReviewLease {
    /// Publication token; identical-content refreshes still have distinct tokens.
    pub fn token(&self) -> &str {
        &self.pair.token
    }
    /// Portable ordered comparison identity, independent of publication lifetime.
    pub fn comparison_id(&self) -> &ComparisonId {
        self.pair.comparison.id()
    }
    /// Borrowed metadata only; no captured source owners escape this lease.
    pub fn entries(&self) -> impl Iterator<Item = ComparedEntry<'_>> {
        self.pair.comparison.entries()
    }
    /// Projects bounded source/outline rows from the explicitly selected held side.
    pub fn window(
        &self,
        side: Side,
        path: &ReviewPath,
        query: &WindowQuery,
    ) -> Result<Window, ReviewError> {
        let document = self
            .pair
            .comparison
            .document(side, path)
            .ok_or(ReviewError::SourceUnavailable)?;
        Ok(document.window(query, None)?)
    }
    /// Computes complete bounded textual hunks without transferring source ownership.
    pub fn text_diff(
        &self,
        path: &ReviewPath,
        limits: DiffLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<TextDiff, DiffError> {
        self.pair.comparison.text_diff(path, limits, cancelled)
    }
    /// Returns path-scoped native acquisition notices on the selected side.
    pub fn notices(&self, side: Side) -> &[crate::review_source::CaptureNotice] {
        match side {
            Side::Base => &self.pair.base.capture.notices,
            Side::Working => &self.pair.working.capture.notices,
        }
    }
}

fn retained_bytes(capture: &Acquired) -> Result<usize, ReviewError> {
    let mut seen = std::collections::BTreeSet::new();
    let source = capture
        .observation
        .snapshot()
        .documents()
        .iter()
        .filter(|document| seen.insert(Arc::as_ptr(document.captured())))
        .map(|document| document.bytes());
    source
        .chain(capture.values.values().map(Vec::len))
        .try_fold(0usize, |total, bytes| {
            total.checked_add(bytes).ok_or_else(|| {
                ReviewError::Model(ModelError("retained source size overflow".into()))
            })
        })
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vizzi_core::{
        comparison::{EntryContent, EntryKind, EntryState, InventoryEntry, Observation},
        model::{CapturePolicyId, CapturedSource, Document, Snapshot, WorkspaceId},
        policy::{Classification, Provenance, SourceOnlyReason},
    };

    #[test]
    fn shared_source_storage_is_counted_by_owner_rather_than_equal_digest() {
        let workspace = WorkspaceId::derive(&[b"shared-owner-test"]);
        let shared = CapturedSource::capture(b"abc".to_vec(), None);
        let independent = CapturedSource::capture(b"abc".to_vec(), None);
        let mut documents = vec![];
        let mut entries = vec![];
        for (name, source) in [
            ("one", shared.clone()),
            ("two", shared),
            ("three", independent),
        ] {
            documents.push(Arc::new(
                Document::capture_source(
                    workspace.clone(),
                    name.into(),
                    source,
                    vec![],
                    Classification::SourceOnly {
                        language: None,
                        provenance: Provenance::ExplicitScope { scope: ".".into() },
                        reason: SourceOnlyReason::AnalysisNotRequested,
                    },
                )
                .unwrap(),
            ));
            entries.push(InventoryEntry {
                path: ReviewPath::parse(name.as_bytes().to_vec()).unwrap(),
                state: EntryState::Present {
                    kind: EntryKind::Regular { executable: false },
                    content: EntryContent::verified(b"abc"),
                },
            });
        }
        let policy = CapturePolicyId::derive(&[b"fixture-policy"]);
        let snapshot = Arc::new(Snapshot::new(workspace, documents, policy.clone()).unwrap());
        let observation = Arc::new(Observation::from_inventory(snapshot, policy, entries).unwrap());
        let capture = Acquired {
            observation,
            values: [(
                ReviewPath::parse(b"link".to_vec()).unwrap(),
                b"target".to_vec(),
            )]
            .into(),
            notices: vec![],
            bytes: 15,
        };
        assert_eq!(retained_bytes(&capture).unwrap(), 12);
    }

    #[test]
    fn sequence_exhaustion_never_wraps_into_a_live_token() {
        let store = ReviewStore::open(0).unwrap();
        let ticket = store.request().unwrap();
        lock(&store.state).sequence = u64::MAX;
        assert!(matches!(store.request(), Err(ReviewError::Exhausted)));
        assert!(store.begin(&ticket, None).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn held_internal_metadata_does_not_extend_a_closed_store_or_its_payloads() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.rs"), b"abc").unwrap();
        let mut input = crate::review_source::DirectoryInput::open(root.path()).unwrap();
        let store = ReviewStore::open(6).unwrap();
        let accounting = store.accounting();
        let ticket = store.request().unwrap();
        let mut candidate = store.begin(&ticket, None).unwrap();
        candidate
            .capture_base(
                &mut input,
                &CapturePolicy::scope(None, vec![]),
                CaptureLimits::default(),
                &|| false,
            )
            .unwrap();
        candidate
            .capture_working(&mut input, CaptureLimits::default(), &|| false)
            .unwrap();
        candidate.publish().unwrap();
        let ticket = store.request().unwrap();
        let mut pending = store.begin(&ticket, None).unwrap();
        // Models a native operation which already upgraded its weak metadata handle.
        let held_metadata = store.state.clone();
        drop(store);
        assert_eq!(accounting.stats().retained_bytes, 0);
        assert!(matches!(
            pending.capture_base(
                &mut input,
                &CapturePolicy::scope(None, vec![]),
                CaptureLimits::default(),
                &|| false
            ),
            Err(ReviewError::Stale)
        ));
        drop(pending);
        assert_eq!(accounting.stats().pair_slots, 0);
        drop(held_metadata);
    }
}
