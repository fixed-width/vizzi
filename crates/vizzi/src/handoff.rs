//! Editor-neutral captured-source handoffs with bounded, window-local ownership.
use crate::{
    companion::CompanionError, definitions::Position, host::Host, structural::StructureHost,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};
use tokio::sync::{Semaphore, watch};
use tokio::time::Instant;
use vizzi_core::{
    comparison::ReviewPath,
    model::{ContentId, Snapshot},
};

const SOURCE_BYTES: usize = 4 * 1024 * 1024;
const MAX_SEQUENCE: u64 = 9_007_199_254_740_991;

/// Native-owner resource and timing bounds; editor requests cannot override them.
#[derive(Clone, Copy)]
pub struct Limits {
    /// Fixed lifetime of an admitted handoff.
    pub deadline: Duration,
    /// Idle retention of an ordering scope.
    pub window_idle: Duration,
    /// Maximum simultaneously retained window scopes.
    pub windows: usize,
    /// Maximum time spent awaiting a result in a finite transport exchange.
    pub observe_wait: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            deadline: Duration::from_secs(30),
            window_idle: Duration::from_secs(300),
            windows: 64,
            observe_wait: Duration::from_millis(50),
        }
    }
}
/// One start or continuation, ordered within an attached editor window.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Native-issued window token.
    pub scope: String,
    /// Increasing window-local user intent.
    pub sequence: u64,
    /// Original input or an opaque continuation.
    pub action: Action,
}
/// Finite operations over the same captured-source handoff.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    /// Admit an original saved-source observation.
    Start {
        /// Normalised source-root-relative path.
        path: String,
        /// Original saved UTF-8 text, limited to 4 MiB.
        text: String,
        /// Requested reading destination.
        target: Target,
    },
    /// Retrieve the state without renewing its deadline.
    Continue {
        /// Opaque native continuation identity.
        handle: String,
    },
}
/// Selection semantics owned by the requested Vizzi view.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
    /// Enter the enclosing declaration and exact written call.
    Call {
        /// Zero-based UTF-16 call identifier position.
        position: Position,
    },
    /// Open the captured file and reveal its reading line.
    File {
        /// One-based requested line.
        line: usize,
    },
}
/// Idempotent cancellation of one window-local intent.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cancel {
    /// Native-issued window token.
    pub scope: String,
    /// Intent to retire; an older intent cannot cancel a newer one.
    pub sequence: u64,
}
/// Stable finite response shared by HTTP, stdio and native callers.
#[derive(Serialize)]
pub struct Reply {
    /// Response version.
    pub schema: u8,
    /// User intent represented by this response.
    pub sequence: u64,
    /// Current terminal state or continuation.
    #[serde(flatten)]
    pub outcome: Outcome,
}
/// Handoff progress and bounded refusals.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Outcome {
    /// Waiting for capture or bounded projection work.
    Pending {
        /// Opaque continuation identity.
        handle: String,
        /// Suggested delay between finite checks.
        retry_after_ms: u32,
    },
    /// A verified observation-bound destination.
    Ready {
        /// Identities interpreted by the existing browser navigation.
        destination: Destination,
    },
    /// An active request cannot complete.
    Refused {
        /// Stable refusal category.
        code: String,
        /// Bounded, actionable diagnostic.
        message: String,
    },
    /// A newer user intent owns this editor window.
    Superseded,
    /// The caller deliberately retired this intent.
    Cancelled,
    /// The fixed deadline or window scope expired.
    Expired,
}
/// Browser-safe destination without source text or native filesystem paths.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Destination {
    /// Ordinary call-trail entry.
    Call {
        /// Captured source observation.
        snapshot: String,
        /// Exact written call identity.
        call: String,
    },
    /// Ordinary source-browser entry.
    File {
        /// Captured source observation.
        snapshot: String,
        /// Captured file identity.
        file: String,
        /// Captured document version.
        version: String,
        /// One-based available reading line.
        line: usize,
    },
}
struct Witness {
    path: String,
    content: ContentId,
    target: Target,
    byte: Option<usize>,
}
struct Operation {
    handle: String,
    fingerprint: ContentId,
    witness: Option<Arc<Witness>>,
    deadline: Instant,
    state: watch::Sender<Outcome>,
}
impl Operation {
    fn pending(&self) -> bool {
        matches!(*self.state.borrow(), Outcome::Pending { .. })
    }
    fn finish(&self, outcome: Outcome) {
        self.state.send_if_modified(|state| {
            if matches!(state, Outcome::Pending { .. }) {
                *state = outcome;
                true
            } else {
                false
            }
        });
    }
    fn retire(&self, outcome: Outcome) {
        self.state.send_replace(outcome);
    }
}
struct Window {
    seen: u64,
    last_used: Instant,
    current: Option<Arc<Operation>>,
}
struct Runtime {
    host: Arc<Host>,
    structure: Arc<StructureHost>,
    compute: Arc<Semaphore>,
    #[cfg(test)]
    binding_started: tokio::sync::Notify,
}
/// Owns captured-source resolution, independently of presentation and provider connections.
pub struct Handoffs {
    runtime: Arc<Runtime>,
    limits: Limits,
    windows: Mutex<BTreeMap<String, Window>>,
}
impl Handoffs {
    /// Uses fixed production limits for a source workspace.
    pub fn new(host: Arc<Host>, structure: Arc<StructureHost>) -> Self {
        Self::with_limits(host, structure, Limits::default())
    }
    /// Supplies bounds selected by the native owner, including deterministic test deadlines.
    pub fn with_limits(host: Arc<Host>, structure: Arc<StructureHost>, limits: Limits) -> Self {
        Self {
            runtime: Arc::new(Runtime {
                host,
                structure,
                compute: Arc::new(Semaphore::new(2)),
                #[cfg(test)]
                binding_started: tokio::sync::Notify::new(),
            }),
            limits,
            windows: Mutex::new(BTreeMap::new()),
        }
    }
    fn windows(&self) -> MutexGuard<'_, BTreeMap<String, Window>> {
        let mut windows = self
            .windows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        windows.retain(|_, window| {
            let keep = window.last_used.elapsed() < self.limits.window_idle;
            if !keep && let Some(op) = &window.current {
                op.retire(Outcome::Expired);
            }
            keep
        });
        windows
    }
    /// Attaches a bounded ordering scope to this host's exact canonical source root.
    pub fn attach_window(&self, root: &Path) -> Result<String, CompanionError> {
        if root != self.runtime.host.root() {
            return Err(CompanionError::Conflict(
                "The selected source workspace changed".into(),
            ));
        }
        let mut windows = self.windows();
        if windows.len() >= self.limits.windows {
            return Err(CompanionError::Unavailable(
                "Editor handoff window allowance is occupied; retry later".into(),
            ));
        }
        let scope = nonce().map_err(CompanionError::Unavailable)?;
        windows.insert(
            scope.clone(),
            Window {
                seen: 0,
                last_used: Instant::now(),
                current: None,
            },
        );
        Ok(scope)
    }
    /// Starts or checks a handoff, waiting only briefly before returning a continuation.
    pub async fn handoff(&self, request: Request) -> Reply {
        let sequence = request.sequence;
        if sequence == 0 || sequence > MAX_SEQUENCE {
            return reply(
                sequence,
                refused("invalid_request", "Invalid editor request sequence"),
            );
        }
        let operation = {
            let (start, continuation) = match request.action {
                Action::Start { path, text, target } => {
                    let fingerprint = fingerprint(&path, &text, &target);
                    (Some((fingerprint, validate(path, text, target))), None)
                }
                Action::Continue { handle } => (None, Some(handle)),
            };
            let mut windows = self.windows();
            let Some(window) = windows.get_mut(&request.scope) else {
                return reply(
                    sequence,
                    refused(
                        "scope_expired",
                        "The editor handoff scope expired; retry from the editor",
                    ),
                );
            };
            window.last_used = Instant::now();
            if sequence < window.seen {
                return reply(sequence, Outcome::Superseded);
            }
            if let Some((fingerprint, witness)) = start {
                if sequence == window.seen {
                    let Some(current) = &window.current else {
                        return reply(sequence, Outcome::Cancelled);
                    };
                    if current.fingerprint != fingerprint {
                        return reply(
                            sequence,
                            refused(
                                "sequence_reused",
                                "Use a new sequence for a new editor observation",
                            ),
                        );
                    }
                    current.clone()
                } else {
                    if let Some(previous) = window.current.take() {
                        previous.retire(Outcome::Superseded);
                    }
                    window.seen = sequence;
                    let handle = match nonce() {
                        Ok(value) => value,
                        Err(error) => return reply(sequence, refused("unavailable", &error)),
                    };
                    let (witness, initial) = match witness {
                        Ok(witness) => (
                            Some(Arc::new(witness)),
                            Outcome::Pending {
                                handle: handle.clone(),
                                retry_after_ms: 100,
                            },
                        ),
                        Err(outcome) => (None, outcome),
                    };
                    let (state, _) = watch::channel(initial);
                    let operation = Arc::new(Operation {
                        handle,
                        fingerprint,
                        witness,
                        deadline: Instant::now() + self.limits.deadline,
                        state,
                    });
                    window.current = Some(operation.clone());
                    if operation.pending() {
                        let runtime = self.runtime.clone();
                        let op = operation.clone();
                        tokio::spawn(async move {
                            runtime.run(op).await;
                        });
                    }
                    operation
                }
            } else {
                if sequence > window.seen {
                    return reply(
                        sequence,
                        refused("invalid_request", "Unknown editor handoff sequence"),
                    );
                }
                let Some(current) = &window.current else {
                    return reply(sequence, Outcome::Cancelled);
                };
                if continuation.as_ref() != Some(&current.handle) {
                    return reply(
                        sequence,
                        refused("invalid_request", "Invalid handoff continuation"),
                    );
                }
                current.clone()
            }
        };
        let mut result = operation.state.subscribe();
        if operation.pending() && !self.limits.observe_wait.is_zero() {
            let _ = tokio::time::timeout(self.limits.observe_wait, result.changed()).await;
        }
        if operation.pending() && Instant::now() >= operation.deadline {
            operation.finish(Outcome::Expired);
        }
        reply(sequence, result.borrow().clone())
    }
    /// Retires only the selected intent, including cancellation delivered before its start.
    pub fn cancel(&self, request: Cancel) -> Reply {
        if request.sequence == 0 || request.sequence > MAX_SEQUENCE {
            return reply(
                request.sequence,
                refused("invalid_request", "Invalid editor request sequence"),
            );
        }
        let mut windows = self.windows();
        let Some(window) = windows.get_mut(&request.scope) else {
            return reply(
                request.sequence,
                refused("scope_expired", "The editor handoff scope expired"),
            );
        };
        window.last_used = Instant::now();
        if request.sequence < window.seen {
            return reply(request.sequence, Outcome::Superseded);
        }
        if request.sequence > window.seen {
            if let Some(previous) = window.current.take() {
                previous.retire(Outcome::Superseded);
            }
            window.seen = request.sequence;
        }
        if let Some(current) = &window.current {
            current.retire(Outcome::Cancelled);
        }
        reply(request.sequence, Outcome::Cancelled)
    }
}
impl Drop for Handoffs {
    fn drop(&mut self) {
        for window in self.windows().values() {
            if let Some(current) = &window.current {
                current.retire(Outcome::Cancelled);
            }
        }
    }
}
fn reply(sequence: u64, outcome: Outcome) -> Reply {
    Reply {
        schema: 1,
        sequence,
        outcome,
    }
}
fn refused(code: &str, message: &str) -> Outcome {
    Outcome::Refused {
        code: code.into(),
        message: crate::review_application::clip(message, 512).0,
    }
}
fn nonce() -> Result<String, String> {
    let mut value = [0; 32];
    getrandom::fill(&mut value).map_err(|e| e.to_string())?;
    Ok(blake3::Hash::from_bytes(value).to_hex().to_string())
}
fn fingerprint(path: &str, text: &str, target: &Target) -> ContentId {
    let point = match target {
        Target::Call { position } => format!("call:{}:{}", position.line, position.character),
        Target::File { line } => format!("file:{line}"),
    };
    ContentId::derive(&[path.as_bytes(), text.as_bytes(), point.as_bytes()])
}
fn validate(path: String, text: String, target: Target) -> Result<Witness, Outcome> {
    if text.len() > SOURCE_BYTES {
        return Err(refused(
            "invalid_source",
            "Editor source exceeds the handoff allowance",
        ));
    }
    if path.len() > 4096 || ReviewPath::parse(path.as_bytes().to_vec()).is_err() {
        return Err(refused(
            "invalid_source",
            "Select a normalised project source path",
        ));
    }
    let byte = match target {
        Target::Call { position } => Some(
            crate::definitions::byte_offset(&text, position)
                .map_err(|e| refused("invalid_position", &e))?,
        ),
        Target::File { line: 0 } => {
            return Err(refused("invalid_position", "Source lines start at one"));
        }
        Target::File { .. } => None,
    };
    Ok(Witness {
        path,
        content: ContentId::derive(&[text.as_bytes()]),
        target,
        byte,
    })
}

impl Runtime {
    async fn run(self: Arc<Self>, operation: Arc<Operation>) {
        let Some(witness) = operation.witness.clone() else {
            return;
        };
        let app = self.host.application();
        let mut updates = app.subscribe();
        let mut cancel = operation.state.subscribe();
        if !operation.pending() {
            return;
        }
        if Instant::now() >= operation.deadline {
            operation.finish(Outcome::Expired);
            return;
        }
        self.host.activate();
        let mut refresh = None;
        loop {
            if !operation.pending() {
                return;
            }
            if Instant::now() >= operation.deadline {
                operation.finish(Outcome::Expired);
                return;
            }
            let status = app.status();
            if !status.pending
                && let Some(snapshot) = app.snapshot()
            {
                let doc = snapshot.file_by_path(&witness.path);
                if doc.is_some_and(|doc| doc.content() == &witness.content) {
                    #[cfg(test)]
                    self.binding_started.notify_one();
                    let permit = tokio::select! {result=self.compute.clone().acquire_owned()=>match result{Ok(permit)=>permit,Err(_)=>{operation.finish(refused("unavailable","Editor handoff computation is unavailable"));return;}},_=cancel.changed()=>return,_=tokio::time::sleep_until(operation.deadline)=>{operation.finish(Outcome::Expired);return;}};
                    let structure = self.structure.clone();
                    let expected = snapshot.id().clone();
                    let observed = witness.clone();
                    let work = tokio::task::spawn_blocking(move || {
                        let _permit = permit;
                        resolve(&structure, &snapshot, &observed)
                    });
                    let answer = tokio::select! {answer=work=>answer,_=cancel.changed()=>return,_=tokio::time::sleep_until(operation.deadline)=>{operation.finish(Outcome::Expired);return;}};
                    if !operation.pending() {
                        return;
                    }
                    if Instant::now() >= operation.deadline {
                        operation.finish(Outcome::Expired);
                        return;
                    }
                    if app
                        .snapshot()
                        .is_some_and(|current| current.id() == &expected)
                        && !app.status().pending
                    {
                        match answer {
                            Ok(Ok(destination)) => {
                                operation.finish(Outcome::Ready { destination });
                                return;
                            }
                            Ok(Err(
                                CompanionError::Conflict(_) | CompanionError::Unavailable(_),
                            )) => {}
                            Ok(Err(error)) => {
                                operation.finish(refused("invalid_target", &error.to_string()));
                                return;
                            }
                            Err(error) => {
                                operation.finish(refused("unavailable", &error.to_string()));
                                return;
                            }
                        }
                    }
                } else if !self.host.handoff_path_admitted(&witness.path) {
                    operation.finish(refused(
                        "not_captured",
                        "This file is outside Vizzi's source scopes",
                    ));
                    return;
                } else if refresh.is_some_and(|ticket| app.desired() >= ticket) {
                    operation.finish(if let Some(error) = status.error {
                        refused("capture_failed", &error)
                    } else if doc.is_none() {
                        refused(
                            "not_captured",
                            "This file was not admitted by source capture",
                        )
                    } else {
                        refused(
                            "source_changed",
                            "Saved source changed before the requested capture was ready",
                        )
                    });
                    return;
                } else {
                    refresh = self.host.refresh_handoff_if_idle(&witness.path);
                }
            } else if status.snapshot.is_none()
                && !status.pending
                && let Some(error) = status.error
            {
                operation.finish(refused("capture_failed", &error));
                return;
            }
            tokio::select! {
                _=cancel.changed()=>return,
                _=tokio::time::sleep_until(operation.deadline)=>{operation.finish(Outcome::Expired);return;},
                next=updates.next()=>{if next.is_none(){operation.finish(refused("unavailable","The source workspace closed"));return;}},
                _=tokio::time::sleep(Duration::from_millis(25))=>{},
            }
        }
    }
}
fn resolve(
    structure: &StructureHost,
    snapshot: &Arc<Snapshot>,
    witness: &Witness,
) -> Result<Destination, CompanionError> {
    let doc = snapshot
        .file_by_path(&witness.path)
        .ok_or_else(|| CompanionError::Conflict("Source observation changed".into()))?;
    let text = doc
        .captured()
        .read_text(SOURCE_BYTES)
        .map_err(|e| CompanionError::Invalid(e.to_string()))?
        .ok_or_else(|| CompanionError::Invalid("Captured source is not UTF-8".into()))?;
    if ContentId::derive(&[text.as_bytes()]) != witness.content {
        return Err(CompanionError::Conflict(
            "Captured source identity differs".into(),
        ));
    }
    match witness.target {
        Target::File { line } => Ok(Destination::File {
            snapshot: snapshot.id().as_str().into(),
            file: doc.id().as_str().into(),
            version: doc.version().as_str().into(),
            line: line.min(doc.captured().line_count().max(1)),
        }),
        Target::Call { .. } => {
            let call = structure.editor_call_at(
                snapshot,
                &witness.path,
                witness.byte.ok_or_else(|| {
                    CompanionError::Invalid("Call position is unavailable".into())
                })?,
            )?;
            Ok(Destination::Call {
                snapshot: call.snapshot,
                call: call.invocation.id,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cache::Cache,
        engine::Engine,
        host::HostOptions,
        source::{CaptureScope, Directory},
    };
    use serde_json::{Value, json};

    #[tokio::test]
    async fn publication_changes_while_binding_waits_for_compute_preserve_current_source_identity()
    {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("source");
        std::fs::create_dir(&root).unwrap();
        let text = "fn first() {}\n";
        std::fs::write(root.join("a.rs"), text).unwrap();
        let host = Arc::new(
            Host::start_with_options(
                Engine::new(
                    Directory::open(&root).unwrap(),
                    Cache::new(temp.path().join("cache")),
                    1,
                )
                .unwrap(),
                HostOptions {
                    watch: false,
                    reconcile_interval: Duration::from_secs(600),
                    idle_maintenance: false,
                },
            )
            .unwrap(),
        );
        let handoffs = Handoffs::new(host.clone(), Arc::new(StructureHost::new(host.clone())));
        let scope = handoffs.attach_window(host.root()).unwrap();
        let occupied = handoffs
            .runtime
            .compute
            .clone()
            .acquire_many_owned(2)
            .await
            .unwrap();
        let first = host.application().snapshot().unwrap().id().clone();
        let pending:Value=serde_json::to_value(handoffs.handoff(serde_json::from_value(json!({"scope":scope,"sequence":1,"action":{"kind":"start","path":"a.rs","text":text,"target":{"kind":"file","line":1}}})).unwrap()).await).unwrap();
        assert_eq!(pending["state"], "pending");
        tokio::time::timeout(
            Duration::from_secs(1),
            handoffs.runtime.binding_started.notified(),
        )
        .await
        .unwrap();
        let mut updates = host.application().subscribe();
        std::fs::write(root.join("b.rs"), "fn second() {}\n").unwrap();
        host.request(CaptureScope::Paths(["b.rs".into()].into()));
        let current = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let change = updates.next().await.unwrap();
                if !change.status.pending
                    && let Some(snapshot) = &change.status.snapshot
                    && snapshot != first.as_str()
                {
                    break snapshot.clone();
                }
            }
        })
        .await
        .unwrap();
        drop(occupied);
        let result=tokio::time::timeout(Duration::from_secs(2),async{loop{let result:Value=serde_json::to_value(handoffs.handoff(serde_json::from_value(json!({"scope":scope,"sequence":1,"action":{"kind":"continue","handle":pending["handle"]}})).unwrap()).await).unwrap();if result["state"]!="pending"{break result;}}}).await.unwrap();
        assert_eq!(result["state"], "ready");
        assert_eq!(result["destination"]["snapshot"], current);
    }
}
