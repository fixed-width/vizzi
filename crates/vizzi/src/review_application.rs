//! Latest-request review lifecycle and bounded projections, independent of transport.

use crate::{
    host::Host,
    review::{CaptureTicket, ReviewError, ReviewLease, ReviewStore},
    review_source::{CaptureLimits, CapturePolicy, GitRepository, GitRevision},
};
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

mod projection;
pub use projection::*;

/// Application/projection errors, shared by HTTP and native clients.
#[derive(Debug, thiserror::Error)]
pub enum ReviewApiError {
    /// Input context retaining the original failure category and recovery status.
    #[error("{0}: {1}")]
    Context(String, #[source] Box<ReviewApiError>),
    /// Malformed or unsupported request.
    #[error("{0}")]
    Invalid(String),
    /// A file is outside this held inventory.
    #[error("file is absent from this comparison")]
    NotFound,
    /// A document anchor differs from the selected immutable side.
    #[error("document version differs from the held side")]
    Version,
    /// Comparison/query admission or source failed.
    #[error(transparent)]
    Review(#[from] ReviewError),
    /// Textual hunks cannot be produced.
    #[error(transparent)]
    Diff(#[from] vizzi_core::text_diff::DiffError),
    /// Application is shutting down.
    #[error("review application is closed")]
    Closed,
    /// The bounded query executor is occupied.
    #[error("review query capacity is busy; retry inspection")]
    QueryCapacity,
    /// An owned native operation failed.
    #[error("review operation failed: {0}")]
    Internal(String),
}

/// Explicit local baseline and literal review subtree; no server filesystem path.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateComparison {
    /// Initial provider; only `git` is supported.
    pub provider: String,
    /// Explicit revision to resolve once.
    pub revision: String,
    /// Optional immutable right-hand revision; absent means observed working files.
    pub target_revision: Option<String>,
    /// Relative subtree; absent or `.` means root.
    pub scope: Option<String>,
}

/// Accepted request intent; completion is observed through status/subscription.
#[derive(Clone, Debug, Serialize)]
pub struct ReviewAccepted {
    /// Response schema.
    pub schema: u8,
    /// Store-lifetime request identity.
    pub ticket: String,
}

/// Immutable labels and observation interval bound to one published pair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ComparisonMetadata {
    /// Publication-lifetime token.
    pub token: String,
    /// Initial baseline capture token, preserved across working-source refreshes.
    pub origin: String,
    /// Portable comparison identity.
    pub comparison: String,
    /// Source provider.
    pub provider: String,
    /// Requested baseline label.
    pub revision: String,
    /// Resolved native object identity, unchanged by ref movement.
    pub resolved: String,
    /// Pinned right-hand revision; absent for a working-tree comparison.
    pub target: Option<RevisionTarget>,
    /// Literal review subtree.
    pub scope: String,
    /// Working capture start in Unix milliseconds.
    pub working_started_ms: u64,
    /// Working capture completion in Unix milliseconds, before analysis.
    pub working_finished_ms: u64,
}

/// Requested and resolved identity of an immutable comparison target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RevisionTarget {
    /// Requested revision label.
    pub revision: String,
    /// Resolved native object identity.
    pub resolved: String,
}

/// Bounded failure of the latest request, without replacing the active pair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReviewFailure {
    /// Stable error category.
    pub code: String,
    /// Literal human-readable description, at most 512 UTF-8 bytes.
    pub message: String,
}

/// Complete latest review state, separate from live workspace status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReviewStatus {
    /// Status schema.
    pub schema: u8,
    /// Latest accepted capture intent.
    pub ticket: Option<String>,
    /// Whether that request remains outstanding.
    pub pending: bool,
    /// Last coherent pair labels.
    pub active: Option<ComparisonMetadata>,
    /// Latest request failure.
    pub error: Option<ReviewFailure>,
}

/// Versioned latest-state notification; slow subscribers need no event replay.
#[derive(Clone, Debug, Serialize)]
pub struct ReviewChange {
    /// Notification schema.
    pub schema: u8,
    /// Fresh application stream identity.
    pub stream: String,
    /// Decimal-string monotonic status revision.
    pub revision: String,
    /// Complete current review status.
    pub status: ReviewStatus,
}

/// Native subscription retaining at most the latest status, without captured source.
pub struct ReviewSubscription {
    receiver: tokio::sync::watch::Receiver<Option<Arc<ReviewChange>>>,
    first: bool,
    closed: bool,
}
impl ReviewSubscription {
    /// Returns current status immediately, then newer complete states until closure.
    pub async fn next(&mut self) -> Option<Arc<ReviewChange>> {
        if self.closed {
            return None;
        }
        if self.first {
            self.first = false;
        } else if self.receiver.changed().await.is_err() {
            self.closed = true;
            return None;
        }
        let value = self.receiver.borrow_and_update().clone();
        self.closed = value.is_none();
        value
    }
}

struct Spec {
    pin: GitRevision,
    target: Option<(String, GitRevision)>,
    origin: String,
    revision: String,
    scope: String,
}
struct Active {
    metadata: ComparisonMetadata,
    spec: Arc<Spec>,
}
enum Operation {
    Create(CreateComparison),
    Refresh(String),
}
struct Job {
    ticket: CaptureTicket,
    operation: Operation,
}
struct Inner {
    status: ReviewStatus,
    active: Option<Active>,
    queued: Option<Job>,
    revision: u64,
}
struct Shared {
    host: Arc<Host>,
    store: ReviewStore,
    limits: CaptureLimits,
    inner: Mutex<Inner>,
    wake: Condvar,
    stop: AtomicBool,
    updates: tokio::sync::watch::Sender<Option<Arc<ReviewChange>>>,
    stream: String,
}

/// Owns one latest-request capture worker and a bounded native review store.
pub struct ReviewApplication {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}
impl ReviewApplication {
    /// Starts review work for the host's configured root and exact shared engine.
    pub fn start(host: Arc<Host>, limits: CaptureLimits) -> Result<Self, ReviewApiError> {
        let store = ReviewStore::open(limits.max_bytes)
            .map_err(|error| ReviewApiError::Internal(error.to_string()))?;
        let mut nonce = [0; 32];
        getrandom::fill(&mut nonce).map_err(|error| ReviewApiError::Internal(error.to_string()))?;
        let stream = blake3::Hash::from_bytes(nonce).to_hex().to_string();
        let status = ReviewStatus {
            schema: 1,
            ticket: None,
            pending: false,
            active: None,
            error: None,
        };
        let initial = Arc::new(ReviewChange {
            schema: 1,
            stream: stream.clone(),
            revision: "0".into(),
            status: status.clone(),
        });
        let (updates, _) = tokio::sync::watch::channel(Some(initial));
        let shared = Arc::new(Shared {
            host,
            store,
            limits,
            inner: Mutex::new(Inner {
                status,
                active: None,
                queued: None,
                revision: 0,
            }),
            wake: Condvar::new(),
            stop: AtomicBool::new(false),
            updates,
            stream,
        });
        let worker_shared = shared.clone();
        let worker = std::thread::Builder::new()
            .name("vizzi-review".into())
            .spawn(move || worker_loop(worker_shared))
            .map_err(|error| ReviewApiError::Internal(error.to_string()))?;
        Ok(Self {
            shared,
            worker: Some(worker),
        })
    }
    /// Validates and queues one explicit baseline request, replacing queued intent.
    pub fn create(&self, request: CreateComparison) -> Result<ReviewAccepted, ReviewApiError> {
        validate(&request)?;
        self.enqueue(Operation::Create(request), None)
    }
    /// Captures working source again against the exact active baseline and scope.
    pub fn refresh(&self, token: &str) -> Result<ReviewAccepted, ReviewApiError> {
        if self.query(token)?.metadata().target.is_some() {
            return Err(ReviewApiError::Invalid(
                "Commit comparisons are fixed. Choose new revisions to compare.".into(),
            ));
        }
        self.enqueue(Operation::Refresh(token.into()), Some(token))
    }
    /// Retires active lookup and cancels outstanding review intent.
    pub fn close(&self, token: &str) -> Result<(), ReviewApiError> {
        let mut retired = None;
        self.shared.store.close_with(token, || {
            let mut inner = lock(&self.shared.inner);
            retired = inner.active.take();
            inner.queued = None;
            inner.status = ReviewStatus {
                schema: 1,
                ticket: None,
                pending: false,
                active: None,
                error: None,
            };
            self.shared.changed(&mut inner);
        })?;
        drop(retired);
        Ok(())
    }
    /// Observes small latest-state metadata, without source capture or analysis.
    pub fn status(&self) -> ReviewStatus {
        lock(&self.shared.inner).status.clone()
    }
    /// Subscribes independently of live workspace updates.
    pub fn subscribe(&self) -> ReviewSubscription {
        ReviewSubscription {
            receiver: self.shared.updates.subscribe(),
            first: true,
            closed: false,
        }
    }
    /// Pins one current pair and its matching immutable application labels.
    pub fn query(&self, token: &str) -> Result<ComparisonQuery, ReviewApiError> {
        if self.shared.stop.load(Ordering::Acquire) {
            return Err(ReviewApiError::Closed);
        }
        let (lease, metadata) = self.shared.store.query_with(token, || {
            lock(&self.shared.inner)
                .active
                .as_ref()
                .map(|active| active.metadata.clone())
        })?;
        let metadata = metadata.filter(|meta| meta.token == token).ok_or_else(|| {
            ReviewApiError::Internal("published review metadata is unavailable".into())
        })?;
        Ok(ComparisonQuery { lease, metadata })
    }
    /// Cancels work and closes subscriptions; individual native IO is cooperative.
    pub fn shutdown(&self) {
        self.shared.store.cancel_with(|| {
            self.shared.stop.store(true, Ordering::Release);
            let _inner = lock(&self.shared.inner);
            self.shared.updates.send_replace(None);
        });
        self.shared.wake.notify_all();
    }

    fn enqueue(
        &self,
        operation: Operation,
        expected: Option<&str>,
    ) -> Result<ReviewAccepted, ReviewApiError> {
        if self.shared.stop.load(Ordering::Acquire) {
            return Err(ReviewApiError::Closed);
        }
        let ticket = self.shared.store.request_with(expected, |ticket| {
            if self.shared.stop.load(Ordering::Acquire) {
                return;
            }
            let mut inner = lock(&self.shared.inner);
            inner.status.ticket = Some(ticket.as_str().into());
            inner.status.pending = true;
            inner.status.error = None;
            inner.queued = Some(Job {
                ticket: ticket.clone(),
                operation,
            });
            self.shared.changed(&mut inner);
        })?;
        if self.shared.stop.load(Ordering::Acquire) {
            return Err(ReviewApiError::Closed);
        }
        self.shared.wake.notify_one();
        Ok(ReviewAccepted {
            schema: 1,
            ticket: ticket.as_str().into(),
        })
    }
}

impl Drop for ReviewApplication {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Shared {
    fn changed(&self, inner: &mut Inner) {
        let Some(next) = inner.revision.checked_add(1) else {
            self.stop.store(true, Ordering::Release);
            self.updates.send_replace(None);
            return;
        };
        inner.revision = next;
        if !self.stop.load(Ordering::Acquire) {
            self.updates.send_replace(Some(Arc::new(ReviewChange {
                schema: 1,
                stream: self.stream.clone(),
                revision: next.to_string(),
                status: inner.status.clone(),
            })));
        }
    }
    fn fail(&self, ticket: &CaptureTicket, error: &ReviewApiError) {
        self.store.update_if_current(ticket, || {
            let mut inner = lock(&self.inner);
            inner.status.pending = false;
            inner.status.error = Some(ReviewFailure {
                code: error.code().into(),
                message: clip(&error.to_string(), 512).0,
            });
            self.changed(&mut inner);
        });
    }
}

fn worker_loop(shared: Arc<Shared>) {
    loop {
        let job = {
            let mut inner = lock(&shared.inner);
            while inner.queued.is_none() && !shared.stop.load(Ordering::Acquire) {
                inner = shared
                    .wake
                    .wait(inner)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
            if shared.stop.load(Ordering::Acquire) {
                return;
            }
            inner.queued.take()
        };
        let Some(job) = job else {
            continue;
        };
        let result = run_job(&shared, &job);
        match result {
            Ok((candidate, spec, started, finished)) => {
                let mut retired = None;
                let outcome = candidate.publish_with(|lease| {
                    let metadata = ComparisonMetadata {
                        token: lease.token().into(),
                        origin: spec.origin.clone(),
                        comparison: lease.comparison_id().as_str().into(),
                        provider: "git".into(),
                        revision: spec.revision.clone(),
                        resolved: spec.pin.resolved().into(),
                        target: spec.target.as_ref().map(|(revision, pin)| RevisionTarget {
                            revision: revision.clone(),
                            resolved: pin.resolved().into(),
                        }),
                        scope: spec.scope.clone(),
                        working_started_ms: started,
                        working_finished_ms: finished,
                    };
                    let mut inner = lock(&shared.inner);
                    retired = inner.active.replace(Active {
                        metadata: metadata.clone(),
                        spec: spec.clone(),
                    });
                    inner.status.active = Some(metadata);
                    inner.status.pending = false;
                    inner.status.error = None;
                    shared.changed(&mut inner);
                });
                drop(retired);
                if let Err(error) = outcome {
                    shared.fail(&job.ticket, &error.into());
                }
            }
            Err(error) => shared.fail(&job.ticket, &error),
        }
    }
}

fn run_job(
    shared: &Shared,
    job: &Job,
) -> Result<(crate::review::ReviewCandidate, Arc<Spec>, u64, u64), ReviewApiError> {
    let cancelled = || !shared.store.is_current(&job.ticket);
    let refresh = match &job.operation {
        Operation::Refresh(token) => Some(token.as_str()),
        Operation::Create(_) => None,
    };
    let mut candidate = shared.store.begin(&job.ticket, refresh)?;
    let spec = match &job.operation {
        Operation::Create(request) => {
            let repository = GitRepository::open(shared.host.root()).map_err(ReviewError::from)?;
            let mut pin = repository
                .pin(&request.revision, shared.limits, &cancelled)
                .map_err(|error| {
                    ReviewApiError::Context(
                        format!("Base revision {:?}", request.revision),
                        Box::new(ReviewError::from(error).into()),
                    )
                })?;
            let target = request
                .target_revision
                .as_ref()
                .map(|revision| {
                    repository
                        .pin(revision, shared.limits, &cancelled)
                        .map(|pin| (revision.clone(), pin))
                        .map_err(|error| {
                            ReviewApiError::Context(
                                format!("Target revision {revision:?}"),
                                Box::new(ReviewError::from(error).into()),
                            )
                        })
                })
                .transpose()?;
            let scope = scope(request)?;
            let policy = CapturePolicy::scope(scope, vec![]);
            candidate.capture_base(&mut pin, &policy, shared.limits, &cancelled)?;
            Arc::new(Spec {
                pin: pin.fork(),
                target,
                origin: job.ticket.as_str().into(),
                revision: request.revision.clone(),
                scope: request.scope.as_deref().unwrap_or(".").into(),
            })
        }
        Operation::Refresh(token) => {
            let mut spec = None;
            shared.store.update_if_current(&job.ticket, || {
                let inner = lock(&shared.inner);
                spec = inner
                    .active
                    .as_ref()
                    .filter(|active| active.metadata.token == *token)
                    .map(|active| active.spec.clone());
            });
            spec.ok_or(ReviewError::Stale)?
        }
    };
    let repository = GitRepository::open(shared.host.root()).map_err(ReviewError::from)?;
    let started = now_ms();
    if let Some((_, target)) = &spec.target {
        candidate.capture_working(&mut target.fork(), shared.limits, &cancelled)?;
    } else {
        let mut working = repository.working(&spec.pin).map_err(ReviewError::from)?;
        candidate.capture_working(&mut working, shared.limits, &cancelled)?;
    }
    let finished = now_ms();
    if spec.scope != "." && candidate.inventory_is_empty()? {
        return Err(ReviewApiError::Invalid(format!(
            "Repository path {:?} contains no files in either comparison side. Enter a file/directory path, or . for the repository root.",
            spec.scope
        )));
    }
    shared.host.analyze_review(&mut candidate, &cancelled)?;
    Ok((candidate, spec, started, finished))
}

fn validate(request: &CreateComparison) -> Result<(), ReviewApiError> {
    if request.provider != "git" {
        return Err(ReviewApiError::Invalid(
            "only the git provider is supported".into(),
        ));
    }
    if request.revision.is_empty()
        || request.revision.len() > 256
        || request.revision.contains('\0')
    {
        return Err(ReviewApiError::Invalid(
            "revision must contain 1..256 bytes without NUL".into(),
        ));
    }
    scope(request)?;
    if request
        .target_revision
        .as_ref()
        .is_some_and(|s| s.is_empty() || s.len() > 256 || s.contains('\0'))
    {
        return Err(ReviewApiError::Invalid(
            "Target revision must contain 1..256 bytes without NUL".into(),
        ));
    }
    Ok(())
}
fn scope(
    request: &CreateComparison,
) -> Result<Option<vizzi_core::comparison::ReviewPath>, ReviewApiError> {
    match request.scope.as_deref() {
        None | Some(".") => Ok(None),
        Some(value) if value.len() <= 4096 => {
            vizzi_core::comparison::ReviewPath::parse(value.as_bytes().to_vec())
                .map(Some)
                .map_err(|error| ReviewApiError::Invalid(error.to_string()))
        }
        _ => Err(ReviewApiError::Invalid(
            "review scope exceeds 4096 bytes".into(),
        )),
    }
}
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}
pub(crate) fn clip(text: &str, max: usize) -> (String, bool) {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].into(), end < text.len())
}

impl ReviewApiError {
    pub(crate) fn cause(&self) -> &Self {
        match self {
            Self::Context(_, inner) => inner.cause(),
            _ => self,
        }
    }
    /// Stable failure category, shared by status and transport errors.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Context(_, inner) => inner.code(),
            Self::Invalid(_) => "invalid",
            Self::NotFound => "not_found",
            Self::Version => "version",
            Self::Closed => "closed",
            Self::QueryCapacity => "query_capacity",
            Self::Internal(_) => "internal",
            Self::Diff(_) => "diff_unavailable",
            Self::Review(ReviewError::Stale) => "stale",
            Self::Review(ReviewError::Busy | ReviewError::PairCapacity) => "capacity",
            Self::Review(ReviewError::Capture(crate::review_source::CaptureError::Limit(_))) => {
                "capacity"
            }
            Self::Review(ReviewError::Analysis(_)) => "analysis_failed",
            Self::Review(_) => "capture_failed",
        }
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_clipping_respects_utf8_and_exact_limits() {
        assert_eq!(clip("a🙂🙂🙂", 10), ("a🙂🙂".into(), true));
        assert_eq!(clip("exact", 5), ("exact".into(), false));
        assert_eq!(clip("🙂", 0), (String::new(), true));
        assert_eq!(clip("", 0), (String::new(), false));
    }
    #[test]
    fn native_error_codes_distinguish_recovery_actions() {
        for (error, code) in [
            (ReviewApiError::Invalid("bad".into()), "invalid"),
            (ReviewApiError::NotFound, "not_found"),
            (ReviewApiError::Version, "version"),
            (ReviewApiError::Closed, "closed"),
            (ReviewApiError::QueryCapacity, "query_capacity"),
            (ReviewApiError::Review(ReviewError::Stale), "stale"),
            (
                ReviewApiError::Review(ReviewError::PairCapacity),
                "capacity",
            ),
            (
                ReviewApiError::Review(ReviewError::Capture(
                    crate::review_source::CaptureError::Limit("bytes"),
                )),
                "capacity",
            ),
            (ReviewApiError::Internal("fixture".into()), "internal"),
        ] {
            assert_eq!(error.code(), code);
        }
    }
}
